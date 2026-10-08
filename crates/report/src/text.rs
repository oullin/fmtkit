use std::fmt::Write;

use fmtkit_core::{Mode, Report, RunResult, Severity};

use crate::findings::{self, Group, Item};
use crate::{Options, Summary, result_name};

const BOLD: &str = "1";
const RED: &str = "31";
const GREEN: &str = "32";
const YELLOW: &str = "33";
const MAGENTA: &str = "35";
const PATH: &str = "1;36";

/// The human report: a scope line, one block per path with something to say,
/// the vet status, missing paths, and a final `Result:` line.
pub(crate) fn render(report: &Report, options: Options) -> String {
    let summary = Summary::of(report);
    let check = report.mode == Mode::Check;
    let mut text = Text { out: String::new(), color: options.color };

    if !options.quiet {
        if summary.files == 0 {
            text.line(YELLOW, "  No files in scope.");
        } else {
            let action = if check { "Checked" } else { "Formatted" };

            text.line(&format!("{BOLD};{GREEN}"), &format!("  {action} {} file(s).", summary.files));
        }

        text.blank();
    }

    for group in findings::groups(report) {
        let show_applied = check || !options.quiet;

        if group.items.is_empty() && group.error.is_none() && !show_applied {
            continue;
        }

        text.group(&group, check, show_applied);
    }

    if !options.quiet && text.vet(report) {
        text.blank();
    }

    for path in &report.missing {
        text.line(YELLOW, &format!("  ! path not found: {path}"));
    }

    if !report.missing.is_empty() {
        text.blank();
    }

    let tone = match report.result {
        RunResult::Pass | RunResult::Fixed => GREEN,
        RunResult::Fail => RED,
    };

    let counts = format!(
        "  Result: {}. {} file(s), {} changed, {} violation(s), {} lint, {} complexity, {} vet, {} error(s).",
        result_name(report.result),
        summary.files,
        summary.changed,
        summary.violations,
        summary.lint,
        summary.complexity,
        summary.vet,
        summary.errors,
    );

    text.line(&format!("{BOLD};{tone}"), &counts);
    text.out
}

struct Text {
    out: String,
    color: bool,
}

impl Text {
    fn paint(&self, style: &str, text: &str) -> String {
        if self.color { format!("\x1b[{style}m{text}\x1b[0m") } else { text.to_owned() }
    }

    fn line(&mut self, style: &str, text: &str) {
        let painted = self.paint(style, text);

        self.out.push_str(&painted);
        self.out.push('\n');
    }

    fn blank(&mut self) {
        self.out.push('\n');
    }

    /// Push `message`, indenting its continuation lines under the first.
    fn message(&mut self, message: &str) {
        self.out.push_str(&message.replace('\n', "\n      "));
        self.out.push('\n');
    }

    fn group(&mut self, group: &Group<'_>, check: bool, show_applied: bool) {
        let label = if group.file.is_empty() { "workspace" } else { group.file };

        self.line(PATH, &format!("  {label}"));

        for item in &group.items {
            self.item(item);
        }

        if let Some(error) = group.error {
            let painted = self.paint(RED, &format!("    ! {error}"));

            self.out.push_str(&painted.replace('\n', "\n      "));
            self.out.push('\n');
        }

        if let Some(applied) = group.applied.filter(|_| show_applied) {
            let verb = match (check, applied.is_empty()) {
                (true, true) => "would reformat".to_owned(),
                (false, true) => "reformatted".to_owned(),
                (true, false) => format!("would apply {}", applied.join(", ")),
                (false, false) => format!("applied {}", applied.join(", ")),
            };

            self.line(GREEN, &format!("    ✓ {verb}"));
        }

        self.blank();
    }

    fn item(&mut self, item: &Item<'_>) {
        let rule = self.paint(MAGENTA, &format!("[{}]", item.rule));
        let location = match (item.line, item.column) {
            (0, _) => String::new(),
            (line, 0) => format!("line {line}: "),
            (line, column) => format!("line {line}:{column}: "),
        };

        let warning = match item.severity {
            Severity::Warning => format!("{}: ", self.paint(YELLOW, "warning")),
            Severity::Error => String::new(),
        };

        let _ = write!(self.out, "    {rule} {location}{warning}");
        self.message(item.message);
    }

    /// The vet status line, if vet has one to give; whether a line was written.
    fn vet(&mut self, report: &Report) -> bool {
        let vet = &report.vet;

        if let Some(reason) = &vet.skipped {
            self.line(YELLOW, &format!("  Skipped go vet: {reason}."));

            return true;
        }

        if vet.errors.is_empty() && !vet.targets.is_empty() {
            self.line(GREEN, &format!("  go vet passed on {} target(s).", vet.targets.len()));

            return true;
        }

        false
    }
}
