use std::path::Path;

use rayon::prelude::*;

use fmtkit_cache::{Fresh, Lookup};
use fmtkit_config::Config;
use fmtkit_core::{FileOutcome, Lang, Mode, is_test_file};
use fmtkit_discover::SourceFile;
use fmtkit_go::{GoError, Helper, Limits, Reply, Request, Steps, Ticket};

use crate::{Run, write};

enum Pending<'f> {
    Done(Option<FileOutcome>),
    Read { file: &'f SourceFile, fresh: Fresh },
    Waiting { file: &'f SourceFile, fresh: Fresh, ticket: Ticket },
}

/// Look every Go file up in the cache, send the misses to the helper, then
/// collect the replies in order. The helper (found from `go_helper`) is
/// started only when a file misses, so a run the cache answers never waits
/// for it to start or stop.
///
/// Reading and submitting run on the pool, but nothing there waits for a
/// reply: this thread does the waiting, so script work never stalls behind
/// the helper.
pub fn process(run: &Run<'_>, go_helper: Option<&Path>, files: &[SourceFile], pool: &rayon::ThreadPool) -> Result<Vec<FileOutcome>, GoError> {
    let read: Vec<Pending<'_>> = pool.install(|| files.par_iter().map(|file| read(run, file)).collect());
    let helper = read.iter().any(|item| matches!(item, Pending::Read { .. })).then(|| Helper::spawn(go_helper)).transpose()?;

    let pending = match &helper {
        Some(helper) => pool.install(|| read.into_par_iter().map(|item| submit(run, helper, item)).collect::<Result<_, _>>())?,
        None => read,
    };

    let outcomes = collect(run, helper.as_ref(), pending)?;

    if let Some(helper) = helper {
        helper.shutdown()?;
    }

    Ok(outcomes)
}

/// Wait for each reply in turn. Only a run with misses has a `helper`.
fn collect(run: &Run<'_>, helper: Option<&Helper>, pending: Vec<Pending<'_>>) -> Result<Vec<FileOutcome>, GoError> {
    let mut outcomes = Vec::with_capacity(pending.len());
    let mut rescores = Vec::new();

    for item in pending {
        let outcome = match item {
            Pending::Done(outcome) => outcome,
            Pending::Read { .. } => None,
            Pending::Waiting { file, mut fresh, ticket } => {
                let outcome = finish(run, file, &fresh, ticket.wait()?);

                // Check mode reports against the text on disk, as lint does;
                // the helper scored and linted the formatted text, whose lines
                // moved. The outcome is stored once it is final.
                if run.mode == Mode::Check && outcome.changed && outcome.error.is_none() {
                    match (helper, recheck(run.config, &outcome)) {
                        (Some(helper), Some(steps)) => {
                            let relint = !outcome.lint.is_empty();
                            let request = Request { rel: file.rel.clone(), abs: file.abs.clone(), source: std::mem::take(&mut fresh.bytes), steps };

                            rescores.push((outcomes.len(), fresh, relint, helper.submit(request)?));
                        }
                        _ => run.cache.put_fresh(&fresh, &outcome),
                    }
                }

                Some(outcome)
            }
        };

        run.progress.tick();
        outcomes.extend(outcome);
    }

    for (index, fresh, relint, ticket) in rescores {
        let reply = ticket.wait()?;

        if reply.error.is_none() {
            if !outcomes[index].complexity.is_empty() {
                outcomes[index].complexity = reply.complexity;
            }

            if relint {
                outcomes[index].lint = reply.lint;
            }

            run.cache.put_fresh(&fresh, &outcomes[index]);
        }
    }

    Ok(outcomes)
}

fn read<'f>(run: &Run<'_>, file: &'f SourceFile) -> Pending<'f> {
    let fresh = match run.cache.read(run.mode, &file.rel, &file.abs) {
        Ok(Lookup::Hit(outcome)) => return Pending::Done(Some(outcome)),
        Ok(Lookup::Miss(fresh)) => fresh,
        Err(e) => return Pending::Done(Some(FileOutcome::failed(&file.rel, Some(Lang::Go), format!("read: {e}")))),
    };

    if std::str::from_utf8(&fresh.bytes).is_ok_and(fmtkit_discover::is_generated) {
        return Pending::Done(None);
    }

    Pending::Read { file, fresh }
}

fn submit<'f>(run: &Run<'_>, helper: &Helper, item: Pending<'f>) -> Result<Pending<'f>, GoError> {
    let Pending::Read { file, fresh } = item else {
        return Ok(item);
    };

    let steps = steps(run.config, &file.abs);
    let ticket = helper.submit(Request { rel: file.rel.clone(), abs: file.abs.clone(), source: fresh.bytes.clone(), steps })?;

    Ok(Pending::Waiting { file, fresh, ticket })
}

fn finish(run: &Run<'_>, file: &SourceFile, fresh: &Fresh, reply: Reply) -> FileOutcome {
    let mut outcome = FileOutcome::new(&file.rel, Some(Lang::Go));

    if let Some(error) = reply.error {
        outcome.error = Some(error);

        return outcome;
    }

    outcome.changed = reply.output != fresh.bytes;
    outcome.applied = reply.applied;
    outcome.violations = reply.violations;
    outcome.complexity = reply.complexity;
    outcome.lint = reply.lint;

    if !outcome.changed {
        run.cache.put_fresh(fresh, &outcome);

        return outcome;
    }

    if run.mode == Mode::Format {
        if let Err(e) = write::atomic(&file.abs, &reply.output) {
            outcome.error = Some(format!("write: {e}"));

            return outcome;
        }

        // The written text is clean of spacing violations; naming findings
        // were judged on it and stay.
        let clean = FileOutcome { applied: Vec::new(), changed: false, violations: Vec::new(), ..outcome.clone() };

        run.cache.put(run.cache.key(run.mode, &file.rel, &reply.output), &clean);
    }

    outcome
}

fn steps(config: &Config, abs: &Path) -> Steps {
    Steps {
        spacing: config.go.spacing,
        gofmt: config.go.gofmt,
        goimports: config.go.goimports,
        resolve_imports: config.go.resolve_imports,
        complexity: !is_test_file(abs),
        ..checks(config)
    }
}

/// What a check-mode run sends again, against the text on disk, for a file
/// formatting would change: only the kinds of finding the formatted text
/// produced, so the lines they report are the ones on disk. A file whose
/// formatted text produced none is not sent again (`None`), which keeps a
/// cold check of an unformatted tree to one request per file.
fn recheck(config: &Config, outcome: &FileOutcome) -> Option<Steps> {
    let complexity = !outcome.complexity.is_empty();
    let lint = !outcome.lint.is_empty();

    if !complexity && !lint {
        return None;
    }

    let base = if lint { checks(config) } else { Steps::default() };

    Some(Steps { complexity, ..base })
}

/// Only the report-only checks `config` turns on: `naming/*` and the `go/*`
/// nesting and line-length limits. Test files get them too, as they do in
/// the script lane.
fn checks(config: &Config) -> Steps {
    let (max_words, max_local_words) = config.go.word_limits().unwrap_or_default();

    Steps { max_words, max_local_words, package_stutter: config.go.no_package_stutter, lint: limits(config), ..Steps::default() }
}

/// The `go/*` limits. A tab counts as the TypeScript printer's `tab_width`,
/// so both lanes measure a line alike.
fn limits(config: &Config) -> Limits {
    Limits {
        max_depth: config.go.max_depth,
        max_nested_funcs: config.go.max_nested_funcs,
        max_line_length: config.go.max_line_length,
        tab_width: config.ts.format.tab_width.into(),
    }
}

/// Format one Go source without touching disk. Only the text is wanted, so
/// nothing is scored, named or linted.
pub fn format_one(helper: &Helper, config: &Config, rel: &str, abs: &Path, source: Vec<u8>) -> Result<Reply, GoError> {
    let steps = Steps { complexity: false, max_words: 0, max_local_words: 0, package_stutter: false, lint: Limits::OFF, ..steps(config, abs) };

    helper.submit(Request { rel: rel.to_owned(), abs: abs.to_path_buf(), source, steps })?.wait()
}

#[cfg(test)]
mod tests {
    use fmtkit_config::Config;
    use fmtkit_core::{ComplexityScore, Diagnostic, FileOutcome, Lang, Severity};
    use fmtkit_go::{Limits, Steps};

    use super::recheck;

    fn changed() -> FileOutcome {
        FileOutcome { changed: true, ..FileOutcome::new("a.go", Some(Lang::Go)) }
    }

    fn finding(rule: &str) -> Diagnostic {
        Diagnostic { rule: rule.into(), file: "a.go".into(), line: 3, column: 1, message: "m".into(), severity: Severity::Error }
    }

    #[test]
    fn a_changed_file_without_findings_is_not_sent_again() {
        assert_eq!(recheck(&Config::default(), &changed()), None);
    }

    #[test]
    fn a_recheck_asks_only_for_the_kinds_of_finding_the_formatted_text_had() {
        let config = Config::default();
        let score = ComplexityScore { key: "a.go#f".into(), name: "f".into(), line: 3, cyclomatic: 1, cognitive: 0 };
        let scored = FileOutcome { complexity: vec![score], ..changed() };

        assert_eq!(recheck(&config, &scored), Some(Steps { complexity: true, ..Steps::default() }));

        let linted = FileOutcome { lint: vec![finding("go/line-length")], ..changed() };
        let steps = recheck(&config, &linted).unwrap();

        assert!(!steps.complexity && !steps.spacing && !steps.gofmt && !steps.goimports);
        assert_eq!(steps.lint, Limits { max_depth: 4, max_nested_funcs: 3, max_line_length: 120, tab_width: 4 });
        assert_eq!((steps.max_words, steps.package_stutter), (super::checks(&config).max_words, config.go.no_package_stutter));
    }
}
