//! Runs one fmtkit invocation: discovers the files in scope, processes every
//! file in parallel across the script and Go lanes, runs `go vet` alongside,
//! judges complexity against the allow list, and returns a sorted [`Report`].

mod complexity;
mod go_lane;
mod progress;
mod script_lane;
mod write;

use std::path::{Path, PathBuf};
use std::thread;

use rayon::prelude::*;

use fmtkit_cache::Cache;
use fmtkit_config::Config;
use fmtkit_core::{FileOutcome, Lane, Lang, Mode, REPORT_SCHEMA, Report, RunResult, VetOutcome};
use fmtkit_discover::{Discovery, Scope, SourceFile};
use fmtkit_go::{GoError, Helper, VetTargets};
use fmtkit_lint::{LintError, Linter};

pub use progress::Progress;

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error(transparent)]
    Discover(#[from] fmtkit_discover::DiscoverError),
    #[error(transparent)]
    Lint(#[from] LintError),
    #[error(transparent)]
    Go(#[from] GoError),
    #[error("worker pool: {0}")]
    Pool(String),
    #[error("{path}: unsupported file type")]
    Unsupported { path: String },
    #[error("{path}: {message}")]
    File { path: String, message: String },
}

impl EngineError {
    /// Whether the failure is fmtkit's own (exit 3) rather than the user's input (exit 2).
    pub fn is_internal(&self) -> bool {
        matches!(self, Self::Go(_) | Self::Pool(_))
    }
}

/// How to run.
#[derive(Debug, Clone)]
pub struct Options {
    pub root: PathBuf,
    pub mode: Mode,
    pub scope: Scope,
    pub jobs: usize,
    pub cache: bool,
    /// Overrides the helper lookup.
    pub go_helper: Option<PathBuf>,
}

/// Shared, read-only state for one run.
pub(crate) struct Run<'a> {
    pub config: &'a Config,
    pub mode: Mode,
    pub cache: &'a Cache,
    pub linter: Option<&'a Linter>,
    pub progress: &'a Progress,
}

/// Run fmtkit over the files `options.scope` selects.
pub fn run(config: &Config, options: &Options, progress: &Progress) -> Result<Report, EngineError> {
    let discovery = fmtkit_discover::discover(&options.root, &options.scope, &config.files)?;
    let (go_files, script_files): (Vec<SourceFile>, Vec<SourceFile>) = discovery.files.iter().cloned().partition(|f| f.lang == Lang::Go);

    progress.start(discovery.files.len());

    let linter = if script_files.iter().any(|f| f.lang.is_lintable()) { Some(Linter::new(&config.lint)?) } else { None };
    let helper = if go_files.is_empty() { None } else { Some(Helper::spawn(options.go_helper.as_deref())?) };
    let cache = Cache::open(&options.root, config.hash(), options.cache);
    let pool = rayon::ThreadPoolBuilder::new().num_threads(options.jobs.max(1)).build().map_err(|e| EngineError::Pool(e.to_string()))?;
    let state = Run { config, mode: options.mode, cache: &cache, linter: linter.as_ref(), progress };

    let (scripts, go, vet) = thread::scope(|s| {
        let vet = s.spawn(|| vet_for(config, options, &discovery, &go_files));
        let go = helper.as_ref().map(|helper| s.spawn(|| go_lane::process(&state, helper, &go_files, &pool)));
        let scripts: Vec<FileOutcome> = pool.install(|| script_files.par_iter().filter_map(|file| script_lane::process(&state, file)).collect());

        (scripts, go.map(thread::ScopedJoinHandle::join), vet.join())
    });

    let go = match go {
        Some(Ok(result)) => result?,
        Some(Err(panic)) => std::panic::resume_unwind(panic),
        None => Vec::new(),
    };

    if let Some(helper) = helper {
        helper.shutdown()?;
    }

    let vet = vet.unwrap_or_else(|panic| std::panic::resume_unwind(panic));
    let mut files: Vec<FileOutcome> = scripts.into_iter().chain(go).collect();

    files.sort_unstable_by(|a, b| a.file.cmp(&b.file));

    let lanes = selected_lanes(&options.scope);
    let complexity = complexity::evaluate(&options.root, &config.complexity, &lanes, &discovery.files, &files);

    // A failed flush only costs the next run its warm start.
    let _ = cache.flush();

    progress.finish();

    let result = verdict(options.mode, &files, complexity.is_empty(), &vet);

    Ok(Report { schema: REPORT_SCHEMA, mode: options.mode, result, files, complexity, vet, missing: discovery.missing })
}

/// Format `source` as if it were the file at `path`, returning the new text.
/// Nothing is read from or written to disk, and the cache is not used.
pub fn format_text(config: &Config, root: &Path, path: &Path, source: &str, go_helper: Option<&Path>) -> Result<String, EngineError> {
    let display = path.to_string_lossy().replace('\\', "/");
    let lang = Lang::from_path(path).ok_or_else(|| EngineError::Unsupported { path: display.clone() })?;
    let rel = path.strip_prefix(root).unwrap_or(path).to_string_lossy().replace('\\', "/");
    let abs = if path.is_absolute() { path.to_path_buf() } else { root.join(path) };

    if lang == Lang::Go {
        let helper = Helper::spawn(go_helper)?;
        let reply = go_lane::format_one(&helper, config, &rel, &abs, source.as_bytes().to_vec())?;

        helper.shutdown()?;

        return match reply.error {
            Some(message) => Err(EngineError::File { path: display, message }),
            None => String::from_utf8(reply.output).map_err(|_| EngineError::File { path: display, message: "output is not UTF-8".into() }),
        };
    }

    let linter = if lang.is_lintable() { Some(Linter::new(&config.lint)?) } else { None };

    script_lane::format_text(config, linter.as_ref(), &rel, lang, source).map_err(|message| EngineError::File { path: display, message })
}

fn vet_for(config: &Config, options: &Options, discovery: &Discovery, go_files: &[SourceFile]) -> VetOutcome {
    if !config.go.vet {
        return VetOutcome { skipped: Some("disabled by [go] vet".into()), ..VetOutcome::default() };
    }

    if go_files.is_empty() && !(options.scope.all && discovery.git) {
        return VetOutcome { skipped: Some("no Go files in scope".into()), ..VetOutcome::default() };
    }

    let targets =
        if options.scope.all && options.scope.paths.is_empty() { VetTargets::All } else { VetTargets::Files(go_files.iter().map(|f| f.rel.clone()).collect()) };

    fmtkit_go::vet(&options.root, &targets)
}

fn selected_lanes(scope: &Scope) -> Vec<Lane> {
    if scope.lanes.is_empty() { vec![Lane::Ts, Lane::Go] } else { scope.lanes.clone() }
}

fn verdict(mode: Mode, files: &[FileOutcome], complexity_clean: bool, vet: &VetOutcome) -> RunResult {
    let unresolved = files.iter().any(|f| f.error.is_some() || !f.lint.is_empty() || (mode == Mode::Check && !f.violations.is_empty()));

    if unresolved || !complexity_clean || !vet.errors.is_empty() {
        return RunResult::Fail;
    }

    match (mode, files.iter().any(|f| f.changed)) {
        (Mode::Check, true) => RunResult::Fail,
        (Mode::Format, true) => RunResult::Fixed,
        _ => RunResult::Pass,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fmtkit_core::{Diagnostic, Severity};

    fn changed() -> FileOutcome {
        FileOutcome { changed: true, ..FileOutcome::new("a.ts", Some(Lang::Ts)) }
    }

    #[test]
    fn verdicts() {
        let vet = VetOutcome::default();
        let violation = Diagnostic { rule: "spacing".into(), file: "a.go".into(), line: 1, column: 0, message: "m".into(), severity: Severity::Error };
        let violating = FileOutcome { violations: vec![violation], ..changed() };

        assert_eq!(verdict(Mode::Format, &[], true, &vet), RunResult::Pass);
        assert_eq!(verdict(Mode::Format, &[changed()], true, &vet), RunResult::Fixed);
        assert_eq!(verdict(Mode::Check, &[changed()], true, &vet), RunResult::Fail);
        assert_eq!(verdict(Mode::Format, std::slice::from_ref(&violating), true, &vet), RunResult::Fixed);
        assert_eq!(verdict(Mode::Check, &[violating], true, &vet), RunResult::Fail);
        assert_eq!(verdict(Mode::Format, &[], false, &vet), RunResult::Fail);
    }
}
