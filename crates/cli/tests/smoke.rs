//! End-to-end probes against the built binary, ported from v1's
//! `scripts/test-binary-smoke.sh`. Every fixture is a scratch git repository;
//! the cache and the Go helper are pointed at per-test locations.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;

const BIN: &str = env!("CARGO_BIN_EXE_fmtkit");

struct Fixture {
    dir: tempfile::TempDir,
    cache: tempfile::TempDir,
}

impl Fixture {
    fn new(files: &[(&str, &str)]) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();

        git(dir.path(), &["init", "--quiet", "."]);

        let fixture = Self { dir, cache };

        for (path, content) in files {
            fixture.write(path, content);
        }

        fixture
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    fn write(&self, path: &str, content: &str) {
        let full = self.path().join(path);

        fs::create_dir_all(full.parent().unwrap()).unwrap();
        fs::write(full, content).unwrap();
    }

    fn read(&self, path: &str) -> String {
        fs::read_to_string(self.path().join(path)).unwrap()
    }

    fn run(&self, args: &[&str]) -> Output {
        let mut command = Command::new(BIN);

        command.args(args).current_dir(self.path()).env("FMTKIT_CACHE_DIR", self.cache.path()).env_remove("FMTKIT_CONFIG").env_remove("FMTKIT_JOBS");

        if let Some(helper) = go_helper() {
            command.env("FMTKIT_GO_HELPER", helper);
        }

        command.output().unwrap()
    }

    fn snapshot(&self) -> Vec<(PathBuf, Vec<u8>)> {
        let mut files: Vec<_> = walk(self.path()).into_iter().map(|p| (p.clone(), fs::read(&p).unwrap())).collect();

        files.sort();

        files
    }
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();

    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();

        if path.file_name().is_some_and(|n| n == ".git") {
            continue;
        }

        if path.is_dir() {
            out.extend(walk(&path));
        } else {
            out.push(path);
        }
    }

    out
}

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git").args(args).current_dir(dir).status().unwrap();

    assert!(status.success(), "git {args:?} failed");
}

/// The Go helper, built once per test binary, or `None` when Go is not installed.
fn go_helper() -> Option<&'static Path> {
    static HELPER: OnceLock<Option<PathBuf>> = OnceLock::new();

    HELPER
        .get_or_init(|| {
            let module = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../go/helper");
            let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("fmtkit-go-helper");
            let built = Command::new("go").arg("build").arg("-o").arg(&out).arg(".").current_dir(module).status().ok()?;

            built.success().then_some(out)
        })
        .as_deref()
}

fn text(output: &Output) -> String {
    format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr))
}

fn assert_exit(output: &Output, code: i32) {
    assert_eq!(output.status.code(), Some(code), "unexpected exit status; output:\n{}", text(output));
}

#[test]
fn version_prints_the_release() {
    let output = Fixture::new(&[]).run(&["version"]);

    assert_exit(&output, 0);
    assert_eq!(String::from_utf8_lossy(&output.stdout), format!("fmtkit {}\n", env!("CARGO_PKG_VERSION")));
}

// The template keeps `<div><p>…</p></div>` on one line: markup_fmt leaves short
// block-in-block content inline where v1's Prettier broke it (a 2.0 style change).
#[test]
fn formats_scripts_and_vue_then_settles() {
    let fixture = Fixture::new(&[
        ("app.ts", "const  a = { x:1, s:\"hi\" }\nexport default a\n"),
        (
            "app.vue",
            "<script setup lang=\"ts\">\nconst  a = { x:1, s:\"hi\" }\n</script>\n\n<template>\n<div><p>{{ a.s }}</p></div>\n</template>\n\n<style scoped>\n.box{color:red;padding:0}\n</style>\n",
        ),
    ]);

    assert_exit(&fixture.run(&["format", "--all"]), 0);
    assert_eq!(fixture.read("app.ts"), "const a = { s: 'hi', x: 1 };\n\nexport default a;\n");
    assert_eq!(
        fixture.read("app.vue"),
        "<script setup lang=\"ts\">\nconst a = { s: 'hi', x: 1 };\n</script>\n\n<template>\n\t<div><p>{{ a.s }}</p></div>\n</template>\n\n<style scoped>\n.box {\n\tcolor: red;\n\tpadding: 0;\n}\n</style>\n"
    );

    let settled = fixture.snapshot();

    assert_exit(&fixture.run(&["format", "--all"]), 0);
    assert_eq!(fixture.snapshot(), settled, "a second format changed the tree");
    assert_exit(&fixture.run(&["check", "--all"]), 0);
    assert_exit(&fixture.run(&["check", "--all", "--no-cache"]), 0);
}

#[test]
fn formats_go_through_the_helper() {
    if go_helper().is_none() {
        eprintln!("skipped: go is not installed");

        return;
    }

    let fixture = Fixture::new(&[
        ("go.mod", "module fixture\n\ngo 1.27.1\n"),
        ("app.go", "package p\n\nfunc f() {\n\tdefer println(\"d\")\n\treturn\n}\n"),
        ("gen.go", "// Code generated by test. DO NOT EDIT.\n\npackage p\nfunc  g() {}\n"),
    ]);

    assert_exit(&fixture.run(&["check", "--all"]), 1);
    assert_exit(&fixture.run(&["format", "--all"]), 0);
    assert_eq!(fixture.read("app.go"), "package p\n\nfunc f() {\n\tdefer println(\"d\")\n\n\treturn\n}\n");
    assert_eq!(fixture.read("gen.go"), "// Code generated by test. DO NOT EDIT.\n\npackage p\nfunc  g() {}\n", "generated file was touched");
    assert_exit(&fixture.run(&["check", "--all"]), 0);
}

#[test]
fn check_reports_complexity_on_the_lines_on_disk() {
    let mut files = vec![
        ("fmtkit.toml", "[complexity]\ncyclomatic = 1\n"),
        ("b.ts", "export const z = 1\nexport function g(a: number) {\n\tif (a > 0) { return 1 }\n\treturn 0\n}\n"),
    ];

    if go_helper().is_some() {
        files.push(("go.mod", "module fixture\n\ngo 1.27.1\n"));
        files.push(("a.go", "package p\nvar x = 1\nfunc f(a int) int {\n\tif a > 0 {\n\t\treturn 1\n\t}\n\treturn 0\n}\n"));
    }

    let fixture = Fixture::new(&files);
    let report = text(&fixture.run(&["check", "--all", "--format", "agent"]));

    // Formatting would move both functions down; check reports where they are now.
    assert!(report.contains("b.ts:2 complexity/cyclomatic b.ts#g"), "{report}");
    assert!(go_helper().is_none() || report.contains("a.go:3 complexity/cyclomatic a.go#f"), "{report}");
}

#[test]
fn check_writes_nothing_and_fails_on_changes() {
    let fixture = Fixture::new(&[("app.ts", "const  a = 1\nexport default a\n")]);
    let before = fixture.snapshot();
    let output = fixture.run(&["check", "--all", "--format", "agent"]);

    assert_exit(&output, 1);
    assert_eq!(fixture.snapshot(), before);
    assert!(text(&output).contains("app.ts"), "check did not name the file:\n{}", text(&output));
}

#[test]
fn bundled_policy_reports_every_plugin() {
    let fixture = Fixture::new(&[
        ("types.ts", "export type Foo = { a: number };\n"),
        ("uses.ts", "import { Foo } from './types';\n\nexport const value: Foo = { a: 1 };\n"),
        ("oxc.ts", "export function erasing(y: number): number {\n\treturn y * 0;\n}\n"),
        ("unicorn.ts", "import path from 'path';\n\nexport const p = path.sep;\n"),
        ("importdup.ts", "const a = 1;\nconst b = 2;\n\nexport { a as dup };\nexport { b as dup };\n"),
        ("bundled.ts", "const value = new Date();\nexport const bad = value instanceof Date;\ntest.only('focused', () => {});\n"),
        ("react.tsx", "export const Button = () => <button>Save</button>;\n"),
    ]);

    let output = fixture.run(&["check", "--all", "--format", "agent"]);
    let report = text(&output);

    assert_exit(&output, 1);

    for rule in ["consistent-type-imports", "erasing-op", "prefer-node-protocol", "import", "no-instanceof", "no-only-tests", "button-has-type"] {
        assert!(report.contains(rule), "the bundled policy did not report {rule}:\n{report}");
    }
}

#[test]
fn toml_overrides_the_bundled_policy() {
    let fixture = Fixture::new(&[
        ("fmtkit.toml", "[lint.rules]\n\"require-await\" = \"off\"\n\"typescript/no-explicit-any\" = \"off\"\n"),
        ("local.ts", "export async function localException(): Promise<number> {\n\treturn 1;\n}\n\nexport const anything: any = 1;\n"),
    ]);

    let output = fixture.run(&["check", "--all", "--format", "agent"]);
    let report = text(&output);

    assert!(!report.contains("require-await"), "override ignored:\n{report}");
    assert!(!report.contains("no-explicit-any"), "override ignored:\n{report}");
}

#[test]
fn unknown_config_is_a_usage_error() {
    let fixture = Fixture::new(&[("fmtkit.toml", "[lint]\nmystery = true\n")]);

    assert_exit(&fixture.run(&["check", "--all"]), 2);
}

#[test]
fn javascript_dialects_settle() {
    let fixture = Fixture::new(&[
        ("app.js", "export const result={zebra:1,alpha:2};\n"),
        ("Widget.jsx", "export const Widget=()=> <button type=\"button\">Go</button>;\n"),
        ("module.mjs", "export const ready= true;\n"),
        ("legacy.cjs", "module.exports={zebra:1,alpha:2};\n"),
    ]);

    fixture.run(&["format", "--all", "--ts"]);

    let first = fixture.snapshot();

    fixture.run(&["format", "--all", "--ts"]);

    assert_eq!(fixture.snapshot(), first, "JavaScript formatting is not idempotent");
    assert_exit(&fixture.run(&["check", "--all", "--ts"]), 0);
}

#[test]
fn grouped_case_labels_stay_together_and_lint_clean() {
    let fixture = Fixture::new(&[(
        "switch.ts",
        "export function classify(value: string): number {\n\tswitch (value) {\n\t\tcase 'a':\n\t\tcase 'b':\n\t\t\treturn 1;\n\t\tdefault:\n\t\t\treturn 0;\n\t}\n}\n",
    )]);

    assert_exit(&fixture.run(&["format", "--all"]), 0);
    assert_eq!(
        fixture.read("switch.ts"),
        "export function classify(value: string): number {\n\tswitch (value) {\n\t\tcase 'a':\n\t\tcase 'b':\n\t\t\treturn 1;\n\n\t\tdefault:\n\t\t\treturn 0;\n\t}\n}\n"
    );
    assert_exit(&fixture.run(&["check", "--all"]), 0);
}

#[test]
fn default_scope_is_the_changed_set() {
    let fixture = Fixture::new(&[("committed.ts", "const  a = 1\nexport default a\n")]);

    git(fixture.path(), &["add", "."]);
    git(fixture.path(), &["-c", "user.name=t", "-c", "user.email=t@t", "commit", "--quiet", "-m", "init"]);

    assert_exit(&fixture.run(&["check"]), 0);

    fixture.write("fresh.ts", "const  b = 2\nexport default b\n");

    let output = fixture.run(&["check", "--format", "agent"]);

    assert_exit(&output, 1);
    assert!(text(&output).contains("fresh.ts") && !text(&output).contains("committed.ts"), "{}", text(&output));
}

#[test]
fn named_paths_cover_unchanged_files_and_missing_ones_fail() {
    let fixture = Fixture::new(&[("committed.ts", "const  a = 1\nexport default a\n")]);

    git(fixture.path(), &["add", "."]);
    git(fixture.path(), &["-c", "user.name=t", "-c", "user.email=t@t", "commit", "--quiet", "-m", "init"]);

    let output = fixture.run(&["check", "committed.ts", "--format", "agent"]);

    assert_exit(&output, 1);
    assert!(text(&output).contains("committed.ts"), "{}", text(&output));
    assert_exit(&fixture.run(&["format", "committed.ts"]), 0);
    assert_exit(&fixture.run(&["check", "committed.ts"]), 0);
    assert_exit(&fixture.run(&["check", "committed.ts", "nope.ts"]), 1);
}

#[test]
fn lint_warnings_are_reported_without_failing() {
    let fixture =
        Fixture::new(&[("fmtkit.toml", "[lint.rules]\n\"typescript/no-explicit-any\" = \"warn\"\n"), ("local.ts", "export const anything: any = 1;\n")]);

    let output = fixture.run(&["check", "--all", "--format", "agent"]);

    assert_exit(&output, 0);
    assert!(text(&output).contains("no-explicit-any"), "{}", text(&output));
}

#[test]
fn check_stdin_filepath_reports_instead_of_printing() {
    let fixture = Fixture::new(&[]);
    let check = |input: &[u8]| {
        let mut child = Command::new(BIN)
            .args(["check", "--stdin-filepath", "src/app.ts"])
            .current_dir(fixture.path())
            .env("FMTKIT_CACHE_DIR", fixture.cache.path())
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();

        std::io::Write::write_all(child.stdin.as_mut().unwrap(), input).unwrap();

        child.wait_with_output().unwrap()
    };

    let unformatted = check(b"const  a = 1\nexport default a\n");

    assert_exit(&unformatted, 1);
    assert!(unformatted.stdout.is_empty() && text(&unformatted).contains("src/app.ts"), "{}", text(&unformatted));
    assert_exit(&check(b"const a = 1;\n\nexport default a;\n"), 0);
}

#[test]
fn stdin_filepath_formats_without_touching_disk() {
    let fixture = Fixture::new(&[]);
    let mut child = Command::new(BIN)
        .args(["format", "--stdin-filepath", "src/app.ts"])
        .current_dir(fixture.path())
        .env("FMTKIT_CACHE_DIR", fixture.cache.path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    std::io::Write::write_all(child.stdin.as_mut().unwrap(), b"const  a = { x:1, s:\"hi\" }\nexport default a\n").unwrap();

    let output = child.wait_with_output().unwrap();

    assert_exit(&output, 0);
    assert_eq!(String::from_utf8_lossy(&output.stdout), "const a = { s: 'hi', x: 1 };\n\nexport default a;\n");
    assert_eq!(walk(fixture.path()), Vec::<PathBuf>::new());
}

#[test]
fn json_report_carries_schema_two() {
    let fixture = Fixture::new(&[("app.ts", "export const a = 1;\n")]);
    let output = fixture.run(&["check", "--all", "--format", "json"]);
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    assert_eq!(report["schema"], 2);
}

#[test]
fn missing_helper_is_an_internal_error() {
    let fixture = Fixture::new(&[("app.go", "package p\n")]);
    let output = Command::new(BIN)
        .args(["check", "--all", "--go"])
        .current_dir(fixture.path())
        .env("FMTKIT_CACHE_DIR", fixture.cache.path())
        .env("FMTKIT_GO_HELPER", fixture.path().join("no-such-helper"))
        .output()
        .unwrap();

    assert_exit(&output, 3);
    assert!(text(&output).contains("helper"), "{}", text(&output));
}

// `.- ` reaches an `unreachable!()` in oxc-css-parser 0.0.15 (docs/known-issues.md).
// The panic must stay that file's error: the run finishes and formats the rest.
// Once upstream fixes it, the file simply formats and only the exit code moves.
#[test]
fn a_panicking_upstream_parser_fails_only_its_file() {
    let fixture = Fixture::new(&[("bad.html", "<style>.- o</style>\n"), ("app.ts", "const  a = 1\nexport default a\n")]);
    let output = fixture.run(&["format", "--all", "--format", "json"]);
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).expect("a JSON report");
    let files = report["files"].as_array().expect("files");

    assert_eq!(fixture.read("app.ts"), "const a = 1;\n\nexport default a;\n");
    assert!(files.iter().any(|f| f["file"] == "app.ts" && f["error"].is_null()), "{report:#}");

    if let Some(bad) = files.iter().find(|f| f["file"] == "bad.html" && !f["error"].is_null()) {
        assert!(bad["error"].as_str().unwrap().starts_with("internal error, please report it:"), "{report:#}");
        assert_exit(&output, 1);
    }
}
