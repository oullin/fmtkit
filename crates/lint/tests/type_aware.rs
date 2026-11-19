//! Type-aware rules against a real `tsrs`, over a copy of
//! `fixtures/type-aware`. Each test passes without running when no tsrs can
//! be found (set `FMTKIT_TSRS` to its native binary), so the suite needs no
//! Node.js install.

use std::fs;
use std::path::{Path, PathBuf};

use fmtkit_config::{Exemption, Lint, TypeAwarePreset};
use fmtkit_core::Severity;
use fmtkit_lint::{Linter, UNUSED_DIRECTIVE_RULE};
use serde_json::json;

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/type-aware");

/// A scratch copy of the fixture, so a test can add files.
struct Project {
    dir: tempfile::TempDir,
    tsrs: PathBuf,
}

impl Project {
    fn new() -> Option<Self> {
        let Ok(tsrs) = fmtkit_lint::type_aware::locate(Path::new(FIXTURE), None) else {
            eprintln!("skipped: no tsrs (set FMTKIT_TSRS to its native binary)");

            return None;
        };

        let dir = tempfile::tempdir().unwrap();

        for file in ["tsconfig.json", "src/api.ts", "src/main.ts"] {
            fs::create_dir_all(dir.path().join(file).parent().unwrap()).unwrap();
            fs::copy(Path::new(FIXTURE).join(file), dir.path().join(file)).unwrap();
        }

        Some(Self { dir, tsrs })
    }

    fn root(&self) -> &Path {
        self.dir.path()
    }

    fn write(&self, rel: &str, text: &str) {
        fs::write(self.root().join(rel), text).unwrap();
    }

    fn lint(&self, config: &Lint, files: &[&str], overrides: &[(&str, &str)]) -> Vec<(String, String, u32, u32, Severity)> {
        let linter = Linter::new(config).unwrap();
        let files: Vec<String> = files.iter().map(|f| (*f).to_owned()).collect();
        let overrides: Vec<(String, String)> = overrides.iter().map(|(f, t)| ((*f).to_owned(), (*t).to_owned())).collect();
        let found = linter.lint_types(self.root(), &self.tsrs, &files, &overrides).unwrap();

        found.into_iter().map(|d| (d.file, d.rule, d.line, d.column, d.severity)).collect()
    }
}

fn nkzw() -> Lint {
    Lint { type_aware: Some(TypeAwarePreset::Nkzw), ..Lint::default() }
}

fn only(rule: &str, setting: serde_json::Value) -> Lint {
    Lint { rules: [(rule.to_owned(), setting)].into(), ..Lint::default() }
}

fn error(file: &str, rule: &str, line: u32, column: u32) -> (String, String, u32, u32, Severity) {
    (file.to_owned(), rule.to_owned(), line, column, Severity::Error)
}

const FILES: [&str; 2] = ["src/api.ts", "src/main.ts"];

#[test]
fn reports_the_nkzw_type_aware_rules_at_their_positions() {
    let Some(project) = Project::new() else { return };

    assert_eq!(
        project.lint(&nkzw(), &FILES, &[]),
        [
            error("src/main.ts", "typescript/no-floating-promises", 6, 3),
            error("src/main.ts", "typescript/no-for-in-array", 8, 3),
            error("src/main.ts", "typescript/no-array-delete", 12, 3),
            error("src/main.ts", "typescript/await-thenable", 14, 17),
            error("src/main.ts", "typescript/require-array-sort-compare", 15, 18),
            error("src/main.ts", "typescript/no-unsafe-unary-minus", 16, 20),
            error("src/main.ts", "typescript/no-floating-promises", 18, 3),
        ]
    );
}

/// Why type-aware results stay out of the per-file cache: editing `api.ts`
/// changes what `main.ts` reports while `main.ts` is byte-identical.
#[test]
fn a_change_in_another_file_changes_the_findings() {
    let Some(project) = Project::new() else { return };
    let config = only("typescript/no-floating-promises", json!("error"));
    let sync_api = "export function save(id: string): void {}\nexport const later = (value: number) => value;\n";

    assert_eq!(project.lint(&config, &FILES, &[]).len(), 2);
    assert_eq!(project.lint(&config, &FILES, &[("src/api.ts", sync_api)]), []);

    project.write("src/api.ts", sync_api);

    assert_eq!(project.lint(&config, &FILES, &[]), []);
}

#[test]
fn rules_run_at_their_configured_severity_and_only_on_the_files_given() {
    let Some(project) = Project::new() else { return };
    let warn = only("@typescript-eslint/no-floating-promises", json!("warn"));
    let found = project.lint(&warn, &["src/main.ts"], &[]);

    assert_eq!(found.len(), 2);
    assert!(found.iter().all(|(_, rule, _, _, severity)| rule == "typescript/no-floating-promises" && *severity == Severity::Warning), "{found:?}");
    assert_eq!(project.lint(&warn, &["src/api.ts"], &[]), []);
    assert_eq!(project.lint(&Lint::default(), &FILES, &[]), [], "the bundled policy runs no type-aware rule");
}

const DIRECTIVES: &str = "import { save } from './api';

export function go(): void {
\t// oxlint-disable-next-line typescript/no-floating-promises -- fire and forget
\tsave('a');
\t// eslint-disable-next-line @typescript-eslint/no-floating-promises
\tsave('b');
\t// oxlint-disable-next-line typescript/no-floating-promises
\tvoid save('c');
\t// oxlint-disable-next-line typescript/no-unsafe-assignment
\tvoid save('d');
\tsave('e'); // oxlint-disable-line no-floating-promises
\t/* oxlint-disable-next-line typescript/no-floating-promises, eqeqeq */
\tsave('f');
}
";

/// Both directive prefixes and every rule spelling suppress a tsrs finding. A
/// directive for a type-aware rule that is on and suppressed nothing is
/// unused; one for a type-aware rule that is off is never judged.
#[test]
fn directives_suppress_findings_and_unused_ones_are_reported() {
    let Some(project) = Project::new() else { return };

    project.write("src/directives.ts", DIRECTIVES);

    let config = only("typescript/no-floating-promises", json!("error"));

    assert_eq!(project.lint(&config, &["src/directives.ts"], &[]), [error("src/directives.ts", UNUSED_DIRECTIVE_RULE, 8, 30)]);

    let quiet = Lint {
        rules: [("typescript/no-floating-promises".to_owned(), json!("error")), (UNUSED_DIRECTIVE_RULE.to_owned(), json!("off"))].into(),
        ..Lint::default()
    };

    assert_eq!(project.lint(&quiet, &["src/directives.ts"], &[]), []);

    // With the rule off, the same directives are left alone.
    let other = only("typescript/no-for-in-array", json!("error"));

    assert_eq!(project.lint(&other, &["src/directives.ts"], &[]), []);
}

#[test]
fn exemptions_and_ignore_patterns_drop_findings() {
    let Some(project) = Project::new() else { return };
    let exempt = vec![Exemption { rule: "@typescript-eslint/no-floating-promises".into(), reason: "r".into(), paths: vec!["src/main.ts".into()] }];
    let found = project.lint(&Lint { exempt, ..nkzw() }, &FILES, &[]);

    assert_eq!(found.len(), 5, "{found:?}");
    assert!(!found.iter().any(|(_, rule, ..)| rule.ends_with("no-floating-promises")), "{found:?}");

    let ignored = Lint { ignore: vec!["src/main.ts".into()], ..nkzw() };

    assert_eq!(project.lint(&ignored, &FILES, &[]), []);
}

/// nkzw turns `no-floating-promises` on in its TypeScript override only; a
/// `[lint.rules]` entry applies to every file.
#[test]
fn the_preset_keeps_its_typescript_override() {
    let Some(project) = Project::new() else { return };

    project.write("src/plain.js", "async function f() {}\n\nexport function g() {\n\tf();\n}\n");

    let floating =
        |config: &Lint| project.lint(config, &["src/plain.js"], &[]).into_iter().filter(|(_, rule, ..)| rule.ends_with("no-floating-promises")).count();

    assert_eq!(floating(&nkzw()), 0);
    assert_eq!(floating(&Lint { rules: [("typescript/no-floating-promises".to_owned(), json!("error"))].into(), ..nkzw() }), 1);
}

#[test]
fn a_named_tsrs_that_is_not_executable_is_an_error() {
    if std::env::var_os(fmtkit_lint::type_aware::TSRS_ENV).is_some_and(|v| !v.is_empty()) {
        eprintln!("skipped: FMTKIT_TSRS takes precedence over [lint] tsrs");

        return;
    }

    let config = Lint { tsrs: Some("missing/tsrs".into()), ..only("typescript/no-floating-promises", json!("error")) };
    let error = Linter::new(&config).unwrap().locate_tsrs(Path::new(FIXTURE)).unwrap_err();

    assert!(error.is_setup());
    assert!(error.to_string().contains("[lint] tsrs names") && error.to_string().contains("missing/tsrs"), "{error}");
}
