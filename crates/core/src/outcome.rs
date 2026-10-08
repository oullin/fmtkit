use serde::{Deserialize, Serialize};

use crate::Lang;

/// Whether a run may rewrite files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Rewrite files in place.
    Format,
    /// Report what would change, write nothing.
    Check,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

/// One finding against one file: a formatting violation, a lint diagnostic, a
/// syntax error, or a vet failure. `line` and `column` are 1-based; 0 means the
/// finding has no position.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Diagnostic {
    pub rule: String,
    pub file: String,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub line: u32,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub column: u32,
    pub message: String,
    pub severity: Severity,
}

/// One function's complexity scores. `key` is `<repo-relative path>#<name>`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ComplexityScore {
    pub key: String,
    pub name: String,
    pub line: u32,
    pub cyclomatic: u32,
    pub cognitive: u32,
}

/// Everything one file produced in one run. `file` is repository-relative with
/// forward slashes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileOutcome {
    pub file: String,
    pub lang: Option<Lang>,
    /// The steps that changed the text, in order (`spacing`, `gofmt`, `oxlint`, `oxfmt`, `blank-lines`, ...).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub applied: Vec<String>,
    /// Whether the final text differs from the file on disk.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub changed: bool,
    /// Formatting violations (the Go spacing rule reports these).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub violations: Vec<Diagnostic>,
    /// Lint diagnostics that remain after fixes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lint: Vec<Diagnostic>,
    /// Complexity scores for every function in the file (not only breaches).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub complexity: Vec<ComplexityScore>,
    /// A failure that stopped processing this file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl FileOutcome {
    pub fn new(file: impl Into<String>, lang: Option<Lang>) -> Self {
        Self { file: file.into(), lang, ..Self::default() }
    }

    pub fn failed(file: impl Into<String>, lang: Option<Lang>, error: impl Into<String>) -> Self {
        Self { error: Some(error.into()), ..Self::new(file, lang) }
    }
}

/// A function over a complexity limit, or a stale allow entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ComplexityFinding {
    pub file: String,
    /// `complexity/cyclomatic`, `complexity/cognitive`, or `complexity/allow`.
    pub rule: String,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub line: u32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub key: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AllowEntryStatus {
    Used,
    Stale,
    NotJudged,
}

/// The `go vet` result for a run.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VetOutcome {
    /// Why vet did not run (disabled, no Go toolchain, no module), if it did not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skipped: Option<String>,
    /// The packages or module directories vet was run against.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub targets: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<Diagnostic>,
}

/// The overall verdict of a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RunResult {
    /// Nothing to change and nothing to report.
    Pass,
    /// `format` rewrote files and nothing else is wrong.
    Fixed,
    /// `check` found changes, or any mode found errors, lint diagnostics,
    /// complexity breaches, or vet failures.
    Fail,
}

/// Everything a run produced, sorted by file, ready to render.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Report {
    pub schema: u32,
    pub mode: Mode,
    pub result: RunResult,
    /// Every file in scope, sorted by path.
    pub files: Vec<FileOutcome>,
    pub complexity: Vec<ComplexityFinding>,
    pub vet: VetOutcome,
    /// Scoped paths that did not exist.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub missing: Vec<String>,
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_zero(value: &u32) -> bool {
    *value == 0
}
