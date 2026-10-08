use std::io;

use fmtkit_core::{ComplexityFinding, Diagnostic, FileOutcome, Lang, Report, Severity};
use serde::Serialize;

use crate::{Summary, mode_name, result_name};

/// The schema-2 document: the verdict, the counts, and only the files with
/// something to say. Per-function complexity scores are left out; breaches
/// appear under `complexity`.
#[derive(Serialize)]
struct Document<'a> {
    schema: u32,
    mode: &'static str,
    result: &'static str,
    summary: Summary,
    files: Vec<Entry<'a>>,
    complexity: &'a [ComplexityFinding],
    vet: Vet<'a>,
    missing: &'a [String],
}

/// One file in the `files` array.
#[derive(Serialize)]
struct Entry<'a> {
    file: &'a str,
    lang: Option<Lang>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    changed: bool,
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    applied: &'a [String],
    #[serde(skip_serializing_if = "Vec::is_empty")]
    violations: Vec<Finding<'a>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    lint: Vec<Finding<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<&'a str>,
}

/// A diagnostic without the path its file entry already carries.
#[derive(Serialize)]
struct Finding<'a> {
    rule: &'a str,
    #[serde(skip_serializing_if = "is_zero")]
    line: u32,
    #[serde(skip_serializing_if = "is_zero")]
    column: u32,
    message: &'a str,
    severity: Severity,
}

#[derive(Serialize)]
struct Vet<'a> {
    /// `pass`, `fail`, or `skipped`.
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<&'a str>,
    targets: &'a [String],
    errors: &'a [Diagnostic],
}

pub(crate) fn render(report: &Report) -> io::Result<String> {
    let document = Document {
        schema: fmtkit_core::REPORT_SCHEMA,
        mode: mode_name(report.mode),
        result: result_name(report.result),
        summary: Summary::of(report),
        files: report.files.iter().filter(|file| says_something(file)).map(Entry::from).collect(),
        complexity: &report.complexity,
        vet: Vet::from(report),
        missing: &report.missing,
    };

    let mut text = serde_json::to_string_pretty(&document).map_err(io::Error::other)?;

    text.push('\n');

    Ok(text)
}

fn says_something(file: &FileOutcome) -> bool {
    file.changed || file.error.is_some() || !file.violations.is_empty() || !file.lint.is_empty()
}

impl<'a> From<&'a FileOutcome> for Entry<'a> {
    fn from(file: &'a FileOutcome) -> Self {
        Self {
            file: &file.file,
            lang: file.lang,
            changed: file.changed,
            applied: if file.changed { &file.applied } else { &[] },
            violations: file.violations.iter().map(Finding::from).collect(),
            lint: file.lint.iter().map(Finding::from).collect(),
            error: file.error.as_deref(),
        }
    }
}

impl<'a> From<&'a Diagnostic> for Finding<'a> {
    fn from(diagnostic: &'a Diagnostic) -> Self {
        Self { rule: &diagnostic.rule, line: diagnostic.line, column: diagnostic.column, message: &diagnostic.message, severity: diagnostic.severity }
    }
}

impl<'a> From<&'a Report> for Vet<'a> {
    fn from(report: &'a Report) -> Self {
        let vet = &report.vet;
        let status = match (&vet.skipped, vet.errors.is_empty()) {
            (_, false) => "fail",
            (None, true) if !vet.targets.is_empty() => "pass",
            _ => "skipped",
        };

        Self { status, reason: vet.skipped.as_deref(), targets: &vet.targets, errors: &vet.errors }
    }
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_zero(value: &u32) -> bool {
    *value == 0
}
