use std::fmt::Write;

use fmtkit_core::{Mode, Report, Severity};

use crate::findings;
use crate::{Summary, mode_name, result_name};

/// One line per finding, `path[:line[:column]] rule message`, sorted by path
/// and position, then one `key=value` summary line. Messages are kept on one
/// line by escaping newlines as `\n`, and warnings start with `warning:`. A
/// file that changed is one line with the `format` rule; a file that failed is
/// one line with the `error` rule.
pub(crate) fn render(report: &Report) -> String {
    let mut out = String::new();
    let check = report.mode == Mode::Check;

    for group in findings::groups(report) {
        let path = if group.file.is_empty() { "workspace" } else { group.file };

        for item in &group.items {
            let _ = match (item.line, item.column) {
                (0, _) => write!(out, "{path}"),
                (line, 0) => write!(out, "{path}:{line}"),
                (line, column) => write!(out, "{path}:{line}:{column}"),
            };

            let warning = if item.severity == Severity::Warning { "warning: " } else { "" };
            let _ = write!(out, " {} {warning}", item.rule);

            one_line(&mut out, item.message);
        }

        if let Some(error) = group.error {
            let _ = write!(out, "{path} error ");

            one_line(&mut out, error);
        }

        if let Some(applied) = group.applied {
            let verb = if check { "would apply" } else { "applied" };
            let steps = if applied.is_empty() { "formatting".to_owned() } else { applied.join(", ") };

            let _ = writeln!(out, "{path} format {verb} {steps}");
        }
    }

    for path in &report.missing {
        let _ = writeln!(out, "{path} missing path not found");
    }

    let summary = Summary::of(report);
    let _ = writeln!(
        out,
        "fmtkit schema={} mode={} result={} files={} changed={} violations={} lint={} complexity={} vet={} errors={} missing={}",
        fmtkit_core::REPORT_SCHEMA,
        mode_name(report.mode),
        result_name(report.result),
        summary.files,
        summary.changed,
        summary.violations,
        summary.lint,
        summary.complexity,
        summary.vet,
        summary.errors,
        summary.missing,
    );

    out
}

/// Push `message` and a newline, dropping carriage returns and escaping
/// newlines as `\n`.
fn one_line(out: &mut String, message: &str) {
    for (index, part) in message.trim_end().split('\n').enumerate() {
        if index > 0 {
            out.push_str("\\n");
        }

        out.extend(part.chars().filter(|&c| c != '\r'));
    }

    out.push('\n');
}
