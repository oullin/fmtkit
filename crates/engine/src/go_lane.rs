use std::fs;
use std::path::Path;

use rayon::prelude::*;

use fmtkit_cache::Key;
use fmtkit_config::Config;
use fmtkit_core::{FileOutcome, Lang, Mode, is_test_file};
use fmtkit_discover::SourceFile;
use fmtkit_go::{GoError, Helper, Reply, Request, Steps, Ticket};

use crate::{Run, write};

enum Pending<'f> {
    Done(Option<FileOutcome>),
    Waiting { file: &'f SourceFile, key: Key, source: Vec<u8>, ticket: Ticket },
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
            Pending::Waiting { file, key, source, ticket } => {
                let outcome = finish(run, file, key, &source, ticket.wait()?);

                // Check mode reports against the text on disk, as lint does;
                // the helper scored the formatted text, whose lines moved.
                if run.mode == Mode::Check && outcome.changed && !outcome.complexity.is_empty() {
                    let steps = Steps { complexity: true, ..Steps::default() };
                    let request = Request { rel: file.rel.clone(), abs: file.abs.clone(), source, steps };

                    rescores.push((outcomes.len(), helper.submit(request)?));
                }

                Some(outcome)
            }
        };

        run.progress.tick();
        outcomes.extend(outcome);
    }

    for (index, ticket) in rescores {
        let reply = ticket.wait()?;

        if reply.error.is_none() {
            outcomes[index].complexity = reply.complexity;
        }
    }

    Ok(outcomes)
}

fn submit<'f>(run: &Run<'_>, helper: &Helper, file: &'f SourceFile) -> Result<Pending<'f>, GoError> {
    let source = match fs::read(&file.abs) {
        Ok(source) => source,
        Err(e) => return Ok(Pending::Done(Some(FileOutcome::failed(&file.rel, Some(Lang::Go), format!("read: {e}"))))),
    };

    let key = run.cache.key(run.mode, &file.rel, &source);

    if let Some(outcome) = run.cache.get(&key) {
        return Ok(Pending::Done(Some(outcome)));
    }

    if std::str::from_utf8(&source).is_ok_and(fmtkit_discover::is_generated) {
        return Ok(Pending::Done(None));
    }

    let steps = steps(run.config, &file.abs);
    let ticket = helper.submit(Request { rel: file.rel.clone(), abs: file.abs.clone(), source: source.clone(), steps })?;

    Ok(Pending::Waiting { file, key, source, ticket })
}

fn finish(run: &Run<'_>, file: &SourceFile, key: Key, source: &[u8], reply: Reply) -> FileOutcome {
    let mut outcome = FileOutcome::new(&file.rel, Some(Lang::Go));

    if let Some(error) = reply.error {
        outcome.error = Some(error);

        return outcome;
    }

    outcome.changed = reply.output != source;
    outcome.applied = reply.applied;
    outcome.violations = reply.violations;
    outcome.complexity = reply.complexity;

    if !outcome.changed {
        run.cache.put(key, &outcome);

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
