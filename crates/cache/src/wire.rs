//! The on-disk shape of an outcome.
//!
//! `postcard` is not self-describing, so it cannot read back the core types,
//! whose serde attributes skip empty fields. These mirrors carry every field
//! unconditionally; the exhaustive conversions stop a new core field from
//! being dropped silently.

use fmtkit_core::{ComplexityScore, Diagnostic, FileOutcome, Lang, Severity};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) struct Outcome {
    file: String,
    lang: Option<Lang>,
    applied: Vec<String>,
    changed: bool,
    violations: Vec<Finding>,
    lint: Vec<Finding>,
    complexity: Vec<Score>,
    error: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct Finding {
    rule: String,
    file: String,
    line: u32,
    column: u32,
    message: String,
    severity: Severity,
}

#[derive(Serialize, Deserialize)]
struct Score {
    key: String,
    name: String,
    line: u32,
    cyclomatic: u32,
    cognitive: u32,
}

impl From<&FileOutcome> for Outcome {
    fn from(outcome: &FileOutcome) -> Self {
        let FileOutcome { file, lang, applied, changed, violations, lint, complexity, error } = outcome;

        Self {
            file: file.clone(),
            lang: *lang,
            applied: applied.clone(),
            changed: *changed,
            violations: violations.iter().map(Finding::from).collect(),
            lint: lint.iter().map(Finding::from).collect(),
            complexity: complexity.iter().map(Score::from).collect(),
            error: error.clone(),
        }
    }
}

impl From<Outcome> for FileOutcome {
    fn from(outcome: Outcome) -> Self {
        let Outcome { file, lang, applied, changed, violations, lint, complexity, error } = outcome;

        Self {
            file,
            lang,
            applied,
            changed,
            violations: violations.into_iter().map(Diagnostic::from).collect(),
            lint: lint.into_iter().map(Diagnostic::from).collect(),
            complexity: complexity.into_iter().map(ComplexityScore::from).collect(),
            error,
        }
    }
}

impl From<&Diagnostic> for Finding {
    fn from(diagnostic: &Diagnostic) -> Self {
        let Diagnostic { rule, file, line, column, message, severity } = diagnostic;

        Self { rule: rule.clone(), file: file.clone(), line: *line, column: *column, message: message.clone(), severity: *severity }
    }
}

impl From<Finding> for Diagnostic {
    fn from(finding: Finding) -> Self {
        let Finding { rule, file, line, column, message, severity } = finding;

        Self { rule, file, line, column, message, severity }
    }
}

impl From<&ComplexityScore> for Score {
    fn from(score: &ComplexityScore) -> Self {
        let ComplexityScore { key, name, line, cyclomatic, cognitive } = score;

        Self { key: key.clone(), name: name.clone(), line: *line, cyclomatic: *cyclomatic, cognitive: *cognitive }
    }
}

impl From<Score> for ComplexityScore {
    fn from(score: Score) -> Self {
        let Score { key, name, line, cyclomatic, cognitive } = score;

        Self { key, name, line, cyclomatic, cognitive }
    }
}
