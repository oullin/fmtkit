//! The type-aware lane: one `tsrs` call per run over the script files in
//! scope, its findings merged into their outcomes.
//!
//! Its results bypass the outcome cache. A type-aware finding in one file
//! depends on every file its program reads: the other sources, `.d.ts`
//! files that discovery skips, generated and ignored files, the tsconfig
//! chain and `node_modules`. A cache key that covered all of that would have
//! to read and hash it on every run, so tsrs runs on every run that has a
//! script file in scope and a type-aware rule on, and its findings are merged
//! only after each file's own outcome was cached.

use std::path::Path;

use fmtkit_core::{Diagnostic, FileOutcome, Lang};
use fmtkit_discover::SourceFile;
use fmtkit_lint::{Linter, TypeAwareError};

/// The files to send: every script file in scope that `[lint] ignore` does
/// not skip. Vue files are left out, as the protocol has no Vue support.
pub fn files(linter: Option<&Linter>, script_files: &[SourceFile]) -> Vec<String> {
    let Some(linter) = linter.filter(|linter| !linter.type_aware_rules().is_empty()) else {
        return Vec::new();
    };

    script_files.iter().filter(|file| file.lang.is_script() && !linter.ignores(&file.rel)).map(|file| file.rel.clone()).collect()
}

pub fn run(root: &Path, linter: &Linter, tsrs: &Path, files: &[String]) -> Result<Vec<Diagnostic>, TypeAwareError> {
    linter.lint_types(root, tsrs, files, &[])
}

/// Add each finding to its file's outcome. A file that was sent but has no
/// outcome (a generated file) or failed keeps nothing; a finding for a file
/// that was not sent (a tsconfig error) gets an outcome of its own.
pub fn merge(outcomes: &mut Vec<FileOutcome>, sent: &[String], diagnostics: Vec<Diagnostic>) {
    let mut touched = Vec::new();

    for diagnostic in diagnostics {
        match outcomes.iter().position(|outcome| outcome.file == diagnostic.file) {
            Some(at) if outcomes[at].error.is_none() => {
                outcomes[at].lint.push(diagnostic);
                touched.push(at);
            }
            Some(_) => {}
            None if sent.contains(&diagnostic.file) => {}
            None => {
                let file = diagnostic.file.clone();

                outcomes.push(FileOutcome { lint: vec![diagnostic], ..FileOutcome::new(&file, Lang::from_path(Path::new(&file))) });
                touched.push(outcomes.len() - 1);
            }
        }
    }

    touched.sort_unstable();
    touched.dedup();

    for at in touched {
        let lint = &mut outcomes[at].lint;

        lint.sort_by(|a, b| (a.line, a.column, &a.rule, &a.message).cmp(&(b.line, b.column, &b.rule, &b.message)));
        lint.dedup();
    }
}

#[cfg(test)]
mod tests {
    use fmtkit_core::{Diagnostic, FileOutcome, Lang, Severity};

    use super::merge;

    fn finding(file: &str, line: u32, rule: &str) -> Diagnostic {
        Diagnostic { rule: rule.into(), file: file.into(), line, column: 1, message: "m".into(), severity: Severity::Error }
    }

    #[test]
    fn merges_findings_into_outcomes_in_position_order() {
        let mut outcomes = vec![
            FileOutcome { lint: vec![finding("a.ts", 5, "eqeqeq")], ..FileOutcome::new("a.ts", Some(Lang::Ts)) },
            FileOutcome::failed("broken.ts", Some(Lang::Ts), "read: gone"),
        ];
        let sent = ["a.ts", "broken.ts", "generated.ts"].map(String::from);
        let found = vec![
            finding("a.ts", 2, "typescript/no-floating-promises"),
            finding("broken.ts", 1, "typescript/no-floating-promises"),
            finding("generated.ts", 1, "typescript/no-floating-promises"),
            finding("tsconfig.json", 0, "typescript/program"),
        ];

        merge(&mut outcomes, &sent, found);

        let rules: Vec<(&str, u32)> = outcomes[0].lint.iter().map(|d| (d.rule.as_str(), d.line)).collect();

        assert_eq!(rules, [("typescript/no-floating-promises", 2), ("eqeqeq", 5)]);
        assert_eq!(outcomes[1].lint, []);
        assert_eq!(outcomes.len(), 3);
        assert_eq!(outcomes[2].file, "tsconfig.json");
        assert_eq!(outcomes[2].lint[0].rule, "typescript/program");
    }
}
