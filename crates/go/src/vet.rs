//! `go vet`, scoped to the modules and packages a run touches.
//!
//! Every module (a directory with `go.mod`) is vetted separately from its own
//! directory, because `./...` stops at nested module boundaries. Under
//! [`VetTargets::Modules`] each module gets `./...`; under [`VetTargets::Files`]
//! only the packages holding the listed files are named. Modules run in
//! parallel; the output is parsed into `go/vet` diagnostics. With a
//! [`VetMemo`], a module whose inputs match a run that passed is not vetted
//! again (see [`inputs`]).

mod inputs;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Output, Stdio};

use fmtkit_core::{Diagnostic, Severity, VetOutcome};

use crate::locate::which;
use crate::{VetMemo, VetTargets};

/// The rule every vet diagnostic carries.
pub const RULE: &str = "go/vet";

/// One `go vet` invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ModuleRun {
    /// The module directory, under the root.
    dir: PathBuf,
    /// The package patterns, relative to `dir`: `./...`, `.`, `./sub/pkg`.
    packages: Vec<String>,
    /// Set `GOWORK=off`: a `go.work` above the module does not list it, and
    /// `go` would refuse to build it in workspace mode.
    outside_workspace: bool,
}

pub fn run(root: &Path, targets: &VetTargets, memo: Option<&dyn VetMemo>) -> VetOutcome {
    let Some(go) = which("go") else {
        return skipped("go is not on PATH");
    };

    let modules = match targets {
        VetTargets::Modules(dirs) => plan_modules(root, dirs),
        VetTargets::Files(files) => plan_files(root, files),
    };

    if modules.is_empty() {
        return skipped(match targets {
            VetTargets::Modules(_) => "no Go module under the root",
            VetTargets::Files(_) => "no Go package in scope belongs to a module",
        });
    }

    let gowork_set = std::env::var_os("GOWORK").is_some_and(|value| !value.is_empty());

    let found: Vec<Vec<Diagnostic>> = std::thread::scope(|scope| {
        let handles: Vec<_> = modules.iter().map(|module| scope.spawn(|| vet_module(&go, root, module, gowork_set, memo))).collect();

        handles
            .into_iter()
            .zip(&modules)
            .map(|(handle, module)| handle.join().unwrap_or_else(|_| diagnostics(root, &module.dir, Err(std::io::Error::other("go vet thread panicked")))))
            .collect()
    });

    let mut outcome = VetOutcome::default();

    for (module, errors) in modules.iter().zip(found) {
        outcome.targets.extend(module.packages.iter().map(|package| display_target(root, &module.dir, package)));
        outcome.errors.extend(errors);
    }

    outcome.errors.sort_by(|a, b| (&a.file, a.line, a.column, &a.message).cmp(&(&b.file, b.line, b.column, &b.message)));
    outcome.errors.dedup();

    outcome
}

fn skipped(reason: &str) -> VetOutcome {
    VetOutcome { skipped: Some(reason.to_owned()), ..VetOutcome::default() }
}

/// Vet one module, unless `memo` holds a pass for exactly its inputs. A clean
/// run over settled inputs is remembered.
fn vet_module(go: &Path, root: &Path, module: &ModuleRun, gowork_set: bool, memo: Option<&dyn VetMemo>) -> Vec<Diagnostic> {
    let inputs = memo.and_then(|memo| Some((memo, inputs::fingerprint(go, module)?)));

    if let Some((memo, inputs)) = &inputs
        && memo.passed(&inputs.key)
    {
        return Vec::new();
    }

    let output = invoke(go, module, gowork_set);

    if let Some((memo, inputs)) = inputs
        && inputs.settled
        && output.as_ref().is_ok_and(|output| output.status.success())
    {
        memo.pass(inputs.key);
    }

    diagnostics(root, &module.dir, output)
}

fn invoke(go: &Path, module: &ModuleRun, gowork_set: bool) -> std::io::Result<Output> {
    let mut command = Command::new(go);

    command.arg("vet").args(&module.packages).current_dir(&module.dir).stdin(Stdio::null());

    if module.outside_workspace && !gowork_set {
        command.env("GOWORK", "off");
    }

    command.output()
}

/// The modules in `dirs` (repository-relative), each vetted with `./...`,
/// leaving out what `go` itself skips: `vendor`, `testdata`, and directories
/// starting with `.` or `_`.
fn plan_modules(root: &Path, dirs: &[String]) -> Vec<ModuleRun> {
    let mut dirs: Vec<PathBuf> = dirs
        .iter()
        .filter(|dir| !Path::new(dir.as_str()).components().any(|c| matches!(c, Component::Normal(name) if skipped_dir(&name.to_string_lossy()))))
        .map(|dir| root.join(dir))
        .collect();

    dirs.sort();
    dirs.dedup();

    dirs.into_iter().map(|dir| module_run(dir, vec!["./...".to_owned()])).collect()
}

/// The modules and packages holding `files` (repository-relative). Files
/// outside any module, in directories `go` skips, or in directories with no Go
/// file left (deletions) are dropped.
fn plan_files(root: &Path, files: &[String]) -> Vec<ModuleRun> {
    let mut modules: BTreeMap<PathBuf, BTreeSet<String>> = BTreeMap::new();
    let package_dirs: BTreeSet<&Path> = files
        .iter()
        .filter(|file| Path::new(file.as_str()).extension().is_some_and(|ext| ext == "go"))
        .filter_map(|file| Path::new(file.as_str()).parent())
        .collect();

    for rel_dir in package_dirs {
        if rel_dir.components().any(|c| matches!(c, Component::Normal(name) if skipped_dir(&name.to_string_lossy()))) {
            continue;
        }

        let dir = root.join(rel_dir);

        if !has_go_files(&dir) {
            continue;
        }

        let Some(module) = dir.ancestors().take_while(|ancestor| ancestor.starts_with(root)).find(|ancestor| ancestor.join("go.mod").is_file()) else {
            continue;
        };

        let package = package_pattern(dir.strip_prefix(module).unwrap_or(Path::new("")));

        modules.entry(module.to_path_buf()).or_default().insert(package);
    }

    modules.into_iter().map(|(dir, packages)| module_run(dir, packages.into_iter().collect())).collect()
}

fn module_run(dir: PathBuf, packages: Vec<String>) -> ModuleRun {
    let outside_workspace = outside_workspace(&dir);

    ModuleRun { dir, packages, outside_workspace }
}

fn skipped_dir(name: &str) -> bool {
    name.starts_with('.') || name.starts_with('_') || name == "vendor" || name == "testdata" || name == "node_modules"
}

fn has_go_files(dir: &Path) -> bool {
    std::fs::read_dir(dir).is_ok_and(|entries| {
        entries.filter_map(Result::ok).any(|entry| entry.path().extension().is_some_and(|ext| ext == "go") && entry.file_type().is_ok_and(|t| t.is_file()))
    })
}

/// `.` for the module root, `./a/b` below it.
fn package_pattern(rel: &Path) -> String {
    if rel.as_os_str().is_empty() { ".".to_owned() } else { format!("./{}", slash(rel)) }
}

/// The package as the report shows it: relative to the root, `./`-prefixed.
fn display_target(root: &Path, module: &Path, package: &str) -> String {
    let module_rel = slash(module.strip_prefix(root).unwrap_or(module));
    let package = package.strip_prefix("./").unwrap_or(package);

    match (module_rel.as_str(), package) {
        ("", ".") => ".".to_owned(),
        ("", package) => format!("./{package}"),
        (module_rel, ".") => format!("./{module_rel}"),
        (module_rel, package) => format!("./{module_rel}/{package}"),
    }
}

/// Whether the nearest `go.work` above `module` exists and does not `use` it.
fn outside_workspace(module: &Path) -> bool {
    let Some(work) = module.ancestors().map(|dir| dir.join("go.work")).find(|path| path.is_file()) else {
        return false;
    };

    let Ok(text) = std::fs::read_to_string(&work) else {
        return false;
    };

    let base = work.parent().unwrap_or(Path::new(""));
    let module = normalize(module);

    !workspace_uses(&text).iter().any(|dir| normalize(&base.join(dir)) == module)
}

/// The directories a `go.work` file lists in `use` directives.
fn workspace_uses(text: &str) -> Vec<String> {
    let mut uses = Vec::new();
    let mut in_block = false;

    for line in text.lines() {
        let line = line.split("//").next().unwrap_or("").trim();

        if in_block {
            if line.starts_with(')') {
                in_block = false;
            } else if !line.is_empty() {
                uses.push(unquote(line));
            }

            continue;
        }

        let Some(rest) = line.strip_prefix("use").filter(|rest| rest.starts_with([' ', '\t', '('])) else {
            continue;
        };

        let rest = rest.trim();

        if let Some(inner) = rest.strip_prefix('(') {
            let inner = inner.trim();

            if let Some(single) = inner.strip_suffix(')') {
                uses.extend(single.split_whitespace().map(unquote));
            } else {
                in_block = true;

                if !inner.is_empty() {
                    uses.push(unquote(inner));
                }
            }
        } else if !rest.is_empty() {
            uses.push(unquote(rest));
        }
    }

    uses
}

fn unquote(text: &str) -> String {
    text.trim().trim_matches(|c| c == '"' || c == '`').to_owned()
}

/// Turn one module's `go vet` result into diagnostics. A clean exit has none.
fn diagnostics(root: &Path, module: &Path, output: std::io::Result<Output>) -> Vec<Diagnostic> {
    let module_file = rel_slash(root, &module.join("go.mod"));

    let output = match output {
        Ok(output) => output,
        Err(e) => return vec![diagnostic(module_file, 0, 0, format!("could not run go vet: {e}"))],
    };

    if output.status.success() {
        return Vec::new();
    }

    let text = format!("{}{}", String::from_utf8_lossy(&output.stderr), String::from_utf8_lossy(&output.stdout));
    let (mut found, unparsed) = parse(root, module, &text);

    if !unparsed.is_empty() {
        found.push(diagnostic(module_file, 0, 0, format!("go vet failed:\n{}", unparsed.join("\n"))));
    } else if found.is_empty() {
        found.push(diagnostic(module_file, 0, 0, format!("go vet failed: {}", output.status)));
    }

    found
}

/// Split vet output into positioned diagnostics and the lines that carry no
/// position. `# package` headers are dropped; indented lines continue the
/// previous message.
fn parse(root: &Path, module: &Path, text: &str) -> (Vec<Diagnostic>, Vec<String>) {
    let mut found: Vec<Diagnostic> = Vec::new();
    let mut unparsed = Vec::new();

    for line in text.lines() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with([' ', '\t']) {
            match found.last_mut() {
                Some(last) => {
                    last.message.push('\n');
                    last.message.push_str(line.trim());
                }
                None => unparsed.push(line.trim().to_owned()),
            }

            continue;
        }

        let body = line.strip_prefix("vet: ").unwrap_or(line);

        match split_position(body) {
            Some((path, line_no, column, message)) => {
                let file = rel_slash(root, &normalize(&module.join(path)));

                found.push(diagnostic(file, line_no, column, message.to_owned()));
            }
            None => unparsed.push(line.to_owned()),
        }
    }

    (found, unparsed)
}

/// `path.go:line[:column]: message`.
fn split_position(text: &str) -> Option<(&str, u32, u32, &str)> {
    let end = text.find(".go:")? + 3;
    let (path, rest) = (&text[..end], &text[end + 1..]);
    let (line, rest) = leading_number(rest)?;

    let (column, rest) = match rest.strip_prefix(':').and_then(leading_number) {
        Some((column, rest)) => (column, rest),
        None => (0, rest),
    };

    let message = rest.strip_prefix(':')?.trim();

    Some((path, line, column, message))
}

fn leading_number(text: &str) -> Option<(u32, &str)> {
    let digits = text.bytes().take_while(u8::is_ascii_digit).count();

    Some((text[..digits].parse().ok()?, &text[digits..]))
}

fn diagnostic(file: String, line: u32, column: u32, message: String) -> Diagnostic {
    Diagnostic { rule: RULE.to_owned(), file, line, column, message, severity: Severity::Error }
}

/// `path` relative to `root` with forward slashes, or as given when it lies
/// outside the root.
fn rel_slash(root: &Path, path: &Path) -> String {
    slash(path.strip_prefix(normalize(root)).or_else(|_| path.strip_prefix(root)).unwrap_or(path))
}

fn slash(path: &Path) -> String {
    path.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/").replace("//", "/")
}

/// Resolve `.` and `..` lexically.
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();

    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    out.push("..");
                }
            }
            other => out.push(other),
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use std::sync::Mutex;
    use std::time::{Duration, SystemTime};

    use super::{ModuleRun, display_target, normalize, outside_workspace, parse, plan_files, plan_modules, run, split_position, workspace_uses};
    use crate::{VetMemo, VetTargets};

    /// Remembers passes in memory; `everything` claims every run passed.
    #[derive(Default)]
    struct Memo {
        passes: Mutex<Vec<[u8; 32]>>,
        everything: bool,
    }

    impl VetMemo for Memo {
        fn passed(&self, key: &[u8; 32]) -> bool {
            self.everything || self.passes.lock().unwrap().contains(key)
        }

        fn pass(&self, key: [u8; 32]) {
            self.passes.lock().unwrap().push(key);
        }
    }

    /// Write a file old enough for its stamp to be trusted.
    fn write_settled(path: &Path, text: &str) {
        write(path, text);
        fs::File::options().write(true).open(path).unwrap().set_modified(SystemTime::now() - Duration::from_secs(60)).unwrap();
    }

    fn write(path: &Path, text: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn tree() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        write(&root.join("go.mod"), "module example.com/root\n\ngo 1.27\n");
        write(&root.join("main.go"), "package main\n");
        write(&root.join("pkg/a/a.go"), "package a\n");
        write(&root.join("pkg/a/testdata/x.go"), "package x\n");
        write(&root.join("tools/go.mod"), "module example.com/tools\n\ngo 1.27\n");
        write(&root.join("tools/cmd/t.go"), "package main\n");
        write(&root.join("vendor/v/go.mod"), "module v\n");
        write(&root.join(".hidden/go.mod"), "module h\n");
        write(&root.join("_skip/go.mod"), "module s\n");
        write(&root.join("empty/README"), "no go here\n");

        dir
    }

    fn canonical(dir: &tempfile::TempDir) -> PathBuf {
        dir.path().canonicalize().unwrap()
    }

    #[test]
    fn plans_every_module_but_those_go_skips() {
        let dir = tree();
        let root = canonical(&dir);
        let modules = ["tools", "", "vendor/v", ".hidden", "_skip", "pkg/a/testdata", "tools"].map(String::from);
        let plan = plan_modules(&root, &modules);
        let dirs: Vec<_> = plan.iter().map(|m| m.dir.strip_prefix(&root).unwrap().to_path_buf()).collect();

        assert_eq!(dirs, [PathBuf::new(), PathBuf::from("tools")]);
        assert!(plan.iter().all(|m| m.packages == ["./..."] && !m.outside_workspace));
    }

    #[test]
    fn maps_files_to_packages_in_their_modules() {
        let dir = tree();
        let root = canonical(&dir);
        let files = ["main.go", "pkg/a/a.go", "pkg/a/other.go", "tools/cmd/t.go", "pkg/a/testdata/x.go", "gone/deleted.go", "README.md", "empty/x.go"];
        let plan = plan_files(&root, &files.map(String::from));

        assert_eq!(
            plan,
            [
                ModuleRun { dir: root.clone(), packages: vec![".".into(), "./pkg/a".into()], outside_workspace: false },
                ModuleRun { dir: root.join("tools"), packages: vec!["./cmd".into()], outside_workspace: false },
            ]
        );
    }

    #[test]
    fn ignores_files_outside_any_module() {
        let dir = tempfile::tempdir().unwrap();

        write(&dir.path().join("loose.go"), "package loose\n");

        assert_eq!(plan_files(dir.path(), &["loose.go".to_owned()]), []);
        assert_eq!(plan_modules(dir.path(), &[]), []);
    }

    #[test]
    fn displays_targets_relative_to_the_root() {
        let root = Path::new("/r");

        assert_eq!(display_target(root, root, "./..."), "./...");
        assert_eq!(display_target(root, root, "."), ".");
        assert_eq!(display_target(root, root, "./pkg/a"), "./pkg/a");
        assert_eq!(display_target(root, &root.join("tools"), "./..."), "./tools/...");
        assert_eq!(display_target(root, &root.join("tools"), "."), "./tools");
        assert_eq!(display_target(root, &root.join("tools"), "./cmd"), "./tools/cmd");
    }

    #[test]
    fn reads_workspace_use_directives() {
        let text = "go 1.27\n\nuse ./a // first\nuse (\n\t./b\n\t\"./c d\"\n)\nuse (./e)\nuser ./nope\n";

        assert_eq!(workspace_uses(text), ["./a", "./b", "./c d", "./e"]);
    }

    #[test]
    fn turns_workspace_mode_off_for_unlisted_modules() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        write(&root.join("go.work"), "go 1.27\n\nuse ./listed\n");
        write(&root.join("listed/go.mod"), "module listed\n");
        write(&root.join("unlisted/go.mod"), "module unlisted\n");

        assert!(!outside_workspace(&root.join("listed")));
        assert!(outside_workspace(&root.join("unlisted")));
        assert!(!outside_workspace(&tempfile::tempdir().unwrap().path().join("x")));
    }

    #[test]
    fn parses_vet_output() {
        let text = "# example.com/root/pkg/a\n# [example.com/root/pkg/a]\n./a.go:6:2: fmt.Printf format %d has arg \"x\" of wrong type string\nvet: ../b/b.go:3: undefined: foo\n\tcontinued detail\nsomething else\n";
        let (found, unparsed) = parse(Path::new("/r"), Path::new("/r/pkg/a"), text);

        assert_eq!(found.len(), 2);
        assert_eq!((found[0].file.as_str(), found[0].line, found[0].column), ("pkg/a/a.go", 6, 2));
        assert_eq!(found[0].rule, "go/vet");
        assert_eq!((found[1].file.as_str(), found[1].line, found[1].column), ("pkg/b/b.go", 3, 0));
        assert_eq!(found[1].message, "undefined: foo\ncontinued detail");
        assert_eq!(unparsed, ["something else"]);
    }

    #[test]
    fn splits_positions() {
        assert_eq!(split_position("a.go:1:2: m"), Some(("a.go", 1, 2, "m")));
        assert_eq!(split_position("dir/a.go:10: m: n"), Some(("dir/a.go", 10, 0, "m: n")));
        assert_eq!(split_position("a.go: m"), None);
        assert_eq!(split_position("go.mod:3: m"), None);
        assert_eq!(split_position("a.go:"), None);
    }

    #[test]
    fn normalizes_lexically() {
        assert_eq!(normalize(Path::new("/r/pkg/a/../b/./c.go")), PathBuf::from("/r/pkg/b/c.go"));
    }

    #[test]
    fn vets_a_module_end_to_end() {
        if super::which("go").is_none() {
            eprintln!("skipping: go is not on PATH");

            return;
        }

        let dir = tempfile::tempdir().unwrap();
        let root = canonical(&dir);

        write(&root.join("go.mod"), "module example.com/vetted\n\ngo 1.27\n");
        write(&root.join("bad/bad.go"), "package bad\n\nimport \"fmt\"\n\nfunc Run() {\n\tfmt.Printf(\"%d\", \"not-a-number\")\n}\n");
        write(&root.join("good/good.go"), "package good\n\nfunc Run() {}\n");

        let all = run(&root, &VetTargets::Modules(vec![String::new()]), None);

        assert_eq!(all.skipped, None);
        assert_eq!(all.targets, ["./..."]);
        assert_eq!(all.errors.len(), 1, "{:?}", all.errors);
        assert_eq!((all.errors[0].file.as_str(), all.errors[0].line), ("bad/bad.go", 6));
        assert!(all.errors[0].message.contains("wrong type"), "{}", all.errors[0].message);

        let scoped = run(&root, &VetTargets::Files(vec!["good/good.go".into()]), None);

        assert_eq!(scoped.targets, ["./good"]);
        assert!(scoped.errors.is_empty(), "{:?}", scoped.errors);

        let nothing = run(&root, &VetTargets::Files(vec!["README.md".into()]), None);

        assert!(nothing.skipped.is_some());
    }

    #[test]
    fn skips_modules_whose_inputs_passed_before() {
        if super::which("go").is_none() {
            eprintln!("skipping: go is not on PATH");

            return;
        }

        let dir = tempfile::tempdir().unwrap();
        let root = canonical(&dir);
        let modules = VetTargets::Modules(vec!["good".into(), "bad".into()]);

        write_settled(&root.join("good/go.mod"), "module example.com/good\n\ngo 1.27\n");
        write_settled(&root.join("good/good.go"), "package good\n\nfunc Run() {}\n");
        write_settled(&root.join("bad/go.mod"), "module example.com/bad\n\ngo 1.27\n");
        write_settled(&root.join("bad/bad.go"), "package bad\n\nimport \"fmt\"\n\nfunc Run() {\n\tfmt.Printf(\"%d\", \"x\")\n}\n");

        let memo = Memo::default();

        // Only the clean module is remembered.
        assert_eq!(run(&root, &modules, Some(&memo)).errors.len(), 1);
        assert_eq!(memo.passes.lock().unwrap().len(), 1);
        assert_eq!(run(&root, &modules, Some(&memo)).errors.len(), 1);
        assert_eq!(memo.passes.lock().unwrap().len(), 1);

        // A remembered pass is answered without running vet.
        let claims = Memo { everything: true, ..Memo::default() };
        let skipped = run(&root, &modules, Some(&claims));

        assert!(skipped.errors.is_empty(), "{:?}", skipped.errors);
        assert_eq!(skipped.targets, ["./bad/...", "./good/..."]);

        // A file written just now is not trusted yet.
        write(&root.join("good/good.go"), "package good\n\nfunc Run() { _ = 1 }\n");

        let fresh = Memo::default();

        assert_eq!(run(&root, &VetTargets::Modules(vec!["good".into()]), Some(&fresh)).errors.len(), 0);
        assert!(fresh.passes.lock().unwrap().is_empty());
    }

    #[test]
    fn reports_a_broken_module_without_positions() {
        if super::which("go").is_none() {
            eprintln!("skipping: go is not on PATH");

            return;
        }

        let dir = tempfile::tempdir().unwrap();
        let root = canonical(&dir);

        write(&root.join("go.mod"), "this is not a go.mod\n");
        write(&root.join("a.go"), "package a\n");

        let outcome = run(&root, &VetTargets::Modules(vec![String::new()]), None);

        assert_eq!(outcome.errors.len(), 1, "{:?}", outcome.errors);
        assert_eq!(outcome.errors[0].file, "go.mod");
        assert!(outcome.errors[0].message.starts_with("go vet failed"), "{}", outcome.errors[0].message);
    }
}
