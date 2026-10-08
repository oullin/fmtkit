use std::any::Any;
use std::panic::{self, AssertUnwindSafe};

use fmtkit_cache::Lookup;
use fmtkit_config::Config;
use fmtkit_core::{FileOutcome, Lang, Mode, is_declaration, is_test_file};
use fmtkit_discover::SourceFile;
use fmtkit_lint::Linter;

use crate::{Run, write};

/// What one script or host document became, before it is written.
struct Processed {
    output: String,
    applied: Vec<String>,
    outcome: FileOutcome,
}

/// Process one TypeScript, JavaScript, or host file. `None` leaves the file out
/// of the report (a generated file). A panic inside a parser or formatter
/// becomes this file's error, so one hostile input cannot abort the run.
pub fn process(run: &Run<'_>, file: &SourceFile) -> Option<FileOutcome> {
    let outcome = panic::catch_unwind(AssertUnwindSafe(|| process_inner(run, file)))
        .unwrap_or_else(|payload| Some(FileOutcome::failed(&file.rel, Some(file.lang), internal_error(payload.as_ref()))));

    run.progress.tick();

    outcome
}

fn process_inner(run: &Run<'_>, file: &SourceFile) -> Option<FileOutcome> {
    let mut fresh = match run.cache.read(run.mode, &file.rel, &file.abs) {
        Ok(Lookup::Hit(outcome)) => return Some(outcome),
        Ok(Lookup::Miss(fresh)) => fresh,
        Err(e) => return Some(FileOutcome::failed(&file.rel, Some(file.lang), format!("read: {e}"))),
    };

    let Ok(source) = String::from_utf8(std::mem::take(&mut fresh.bytes)) else {
        return Some(FileOutcome::failed(&file.rel, Some(file.lang), "not valid UTF-8"));
    };

    if fmtkit_discover::is_generated(&source) {
        return None;
    }

    let score = file.lang.is_scorable() && !is_test_file(&file.abs) && !is_declaration(&file.abs);
    let processed = match transform(run.config, run.linter, run.mode, &file.rel, file.lang, &source, score) {
        Ok(processed) => processed,
        Err(message) => return Some(FileOutcome::failed(&file.rel, Some(file.lang), message)),
    };

    let mut outcome = processed.outcome;

    outcome.changed = processed.output != source;
    outcome.applied = processed.applied;

    // Check mode writes nothing, so even a file formatting would change has
    // its final outcome here.
    if !outcome.changed || run.mode == Mode::Check {
        run.cache.put_fresh(&fresh, &outcome);

        return Some(outcome);
    }

    if let Err(e) = write::atomic(&file.abs, processed.output.as_bytes()) {
        outcome.error = Some(format!("write: {e}"));

        return Some(outcome);
    }

    // The rewritten file is clean: a later run over it can skip straight to this outcome.
    let clean = FileOutcome { applied: Vec::new(), changed: false, ..outcome.clone() };

    run.cache.put(run.cache.key(run.mode, &file.rel, processed.output.as_bytes()), &clean);

    Some(outcome)
}

/// Lint fixes, then the lane's formatter, then lint diagnostics: in format
/// mode what is left in the final text, in check mode every finding in the
/// text on disk, fixable ones included, since nothing gets fixed.
fn transform(config: &Config, linter: Option<&Linter>, mode: Mode, rel: &str, lang: Lang, source: &str, score: bool) -> Result<Processed, String> {
    let linter = linter.filter(|l| lang.is_lintable() && !l.ignores(rel));
    let mut applied = Vec::new();
    let mut outcome = FileOutcome::new(rel, Some(lang));
    let mut linted = linter.map(|l| l.lint(rel, lang, source, true));
    let fixed = linted.as_mut().and_then(|l| l.fixed.take());

    if fixed.is_some() {
        applied.push("lint".to_owned());
    }

    let input = fixed.as_deref().unwrap_or(source);
    let output = if lang.is_host() {
        let formatted = fmtkit_hosts::format_host(rel, lang, input, &config.ts.format).map_err(|e| e.to_string())?;

        applied.extend(formatted.applied.iter().map(|s| (*s).to_owned()));

        formatted.output
    } else {
        let formatted = fmtkit_ts::format_source(rel, lang, input, &config.ts.format, score).map_err(|e| e.to_string())?;

        applied.extend(formatted.applied.iter().map(|s| (*s).to_owned()));

        // Check mode reports against the text on disk, as lint does; scores
        // from the formatted text would point at lines it moved.
        outcome.complexity = if score && mode == Mode::Check && formatted.output != source {
            fmtkit_ts::score(rel, lang, source).map_err(|e| e.to_string())?
        } else {
            formatted.complexity
        };

        formatted.output
    };

    if let (Some(linter), Some(linted)) = (linter, linted) {
        outcome.lint = if mode == Mode::Check && fixed.is_some() {
            linter.lint(rel, lang, source, false).diagnostics
        } else if linted.diagnostics.is_empty() || output == input {
            linted.diagnostics
        } else {
            // The formatter moved what is left to report.
            linter.lint(rel, lang, &output, false).diagnostics
        };
    }

    Ok(Processed { output, applied, outcome })
}

/// [`transform`] for `--stdin-filepath`: the formatted text, or the reason it failed.
pub fn format_text(config: &Config, linter: Option<&Linter>, rel: &str, lang: Lang, source: &str) -> Result<String, String> {
    panic::catch_unwind(AssertUnwindSafe(|| transform(config, linter, Mode::Format, rel, lang, source, false).map(|p| p.output)))
        .unwrap_or_else(|payload| Err(internal_error(payload.as_ref())))
}

fn internal_error(payload: &(dyn Any + Send)) -> String {
    let message = payload.downcast_ref::<&str>().copied().or_else(|| payload.downcast_ref::<String>().map(String::as_str)).unwrap_or("unknown panic");

    format!("internal error, please report it: {message}")
}
