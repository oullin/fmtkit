//! Renders a [`Report`] as text for people, JSON for tools, or a compact
//! line-oriented form for coding agents. Every machine form carries
//! `"schema": 2`.
//!
//! Output is deterministic: it depends only on the report, whose files are
//! already sorted, and every renderer orders findings by path, then line,
//! then column.

mod agent;
mod findings;
mod json;
mod text;

use std::io::{self, Write};

use fmtkit_core::{Mode, Report, RunResult};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Format {
    #[default]
    Text,
    Json,
    Agent,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Options {
    pub format: Format,
    /// ANSI colour in text output.
    pub color: bool,
    /// Text output: print only findings and the final line.
    pub quiet: bool,
}

/// Write `report` to `out` in one call.
pub fn render(report: &Report, options: Options, out: &mut dyn Write) -> io::Result<()> {
    let rendered = match options.format {
        Format::Text => text::render(report, options),
        Format::Json => json::render(report)?,
        Format::Agent => agent::render(report),
    };

    out.write_all(rendered.as_bytes())
}

/// The counts every renderer reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct Summary {
    /// Files in scope.
    pub files: usize,
    /// Files whose formatted text differs from the text on disk.
    pub changed: usize,
    pub violations: usize,
    pub lint: usize,
    /// Complexity breaches and stale allow entries.
    pub complexity: usize,
    /// `go vet` errors.
    pub vet: usize,
    /// Files that could not be processed.
    pub errors: usize,
    /// Scope paths that do not exist.
    pub missing: usize,
}

impl Summary {
    pub fn of(report: &Report) -> Self {
        let files = &report.files;

        Self {
            files: files.len(),
            changed: files.iter().filter(|file| file.changed).count(),
            violations: files.iter().map(|file| file.violations.len()).sum(),
            lint: files.iter().map(|file| file.lint.len()).sum(),
            complexity: report.complexity.len(),
            vet: report.vet.errors.len(),
            errors: files.iter().filter(|file| file.error.is_some()).count(),
            missing: report.missing.len(),
        }
    }
}

const fn mode_name(mode: Mode) -> &'static str {
    match mode {
        Mode::Format => "format",
        Mode::Check => "check",
    }
}

const fn result_name(result: RunResult) -> &'static str {
    match result {
        RunResult::Pass => "pass",
        RunResult::Fixed => "fixed",
        RunResult::Fail => "fail",
    }
}
