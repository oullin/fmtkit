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
            let location = match (item.line, item.column) {
                (0, _) => path.to_owned(),
                (line, 0) => format!("{path}:{line}"),
                (line, column) => format!("{path}:{line}:{column}"),
            };

            let warning = if item.severity == Severity::Warning { "warning: " } else { "" };
            let _ = writeln!(out, "{location} {} {warning}{}", item.rule, one_line(item.message));
        }

        if let Some(error) = group.error {
            let _ = writeln!(out, "{path} error {}", one_line(error));
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

fn one_line(message: &str) -> String {
    message.trim_end().replace('\r', "").replace('\n', "\\n")
}
