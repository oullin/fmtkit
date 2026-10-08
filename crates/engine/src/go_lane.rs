use std::path::Path;

use rayon::prelude::*;

use fmtkit_cache::{Fresh, Lookup};
use fmtkit_config::Config;
use fmtkit_core::{FileOutcome, Lang, Mode, is_test_file};
use fmtkit_discover::SourceFile;
use fmtkit_go::{GoError, Helper, Reply, Request, Steps, Ticket};

use crate::{Run, write};

enum Pending<'f> {
    Done(Option<FileOutcome>),
    Waiting { file: &'f SourceFile, fresh: Fresh, ticket: Ticket },
}

/// Send every Go file to the helper, then collect the replies in order.
///
/// Reading and submitting run on the pool, but nothing there waits for a
/// reply: this thread does the waiting, so script work never stalls behind
/// the helper.
pub fn process(run: &Run<'_>, helper: &Helper, files: &[SourceFile], pool: &rayon::ThreadPool) -> Result<Vec<FileOutcome>, GoError> {
    let pending: Vec<Pending<'_>> = pool.install(|| files.par_iter().map(|file| submit(run, helper, file)).collect::<Result<_, _>>())?;
    let mut outcomes = Vec::with_capacity(pending.len());
    let mut rescores = Vec::new();

    for item in pending {
        let outcome = match item {
            Pending::Done(outcome) => outcome,
            Pending::Waiting { file, mut fresh, ticket } => {
                let outcome = finish(run, file, &fresh, ticket.wait()?);

                // Check mode reports against the text on disk, as lint does;
                // the helper scored the formatted text, whose lines moved. The
                // outcome is stored once it is final.
                if run.mode == Mode::Check && outcome.changed && outcome.error.is_none() {
                    if outcome.complexity.is_empty() {
                        run.cache.put_fresh(&fresh, &outcome);
                    } else {
                        let steps = Steps { complexity: true, ..Steps::default() };
                        let request = Request { rel: file.rel.clone(), abs: file.abs.clone(), source: std::mem::take(&mut fresh.bytes), steps };

                        rescores.push((outcomes.len(), fresh, helper.submit(request)?));
                    }
                }

                Some(outcome)
            }
        };

        run.progress.tick();
        outcomes.extend(outcome);
    }

    for (index, fresh, ticket) in rescores {
        let reply = ticket.wait()?;

        if reply.error.is_none() {
            outcomes[index].complexity = reply.complexity;
            run.cache.put_fresh(&fresh, &outcomes[index]);
        }
    }

    Ok(outcomes)
}

fn submit<'f>(run: &Run<'_>, helper: &Helper, file: &'f SourceFile) -> Result<Pending<'f>, GoError> {
    let fresh = match run.cache.read(run.mode, &file.rel, &file.abs) {
        Ok(Lookup::Hit(outcome)) => return Ok(Pending::Done(Some(outcome))),
        Ok(Lookup::Miss(fresh)) => fresh,
        Err(e) => return Ok(Pending::Done(Some(FileOutcome::failed(&file.rel, Some(Lang::Go), format!("read: {e}"))))),
    };

    if std::str::from_utf8(&fresh.bytes).is_ok_and(fmtkit_discover::is_generated) {
        return Ok(Pending::Done(None));
    }

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

    if !outcome.changed {
        run.cache.put_fresh(fresh, &outcome);

        return outcome;
    }

    if run.mode == Mode::Format {
        if let Err(e) = write::atomic(&file.abs, &reply.output) {
            outcome.error = Some(format!("write: {e}"));

            return outcome;
        }

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
    }
}

/// Format one Go source without touching disk.
pub fn format_one(helper: &Helper, config: &Config, rel: &str, abs: &Path, source: Vec<u8>) -> Result<Reply, GoError> {
    let steps = Steps { complexity: false, ..steps(config, abs) };

    helper.submit(Request { rel: rel.to_owned(), abs: abs.to_path_buf(), source, steps })?.wait()
}
