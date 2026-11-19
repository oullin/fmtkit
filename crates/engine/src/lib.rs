//! Runs one fmtkit invocation: discovers the files in scope, processes every
//! file in parallel across the script and Go lanes, runs `go vet` alongside,
//! judges complexity against the allow list, and returns a sorted [`Report`].

mod complexity;
mod go_lane;
mod progress;
mod script_lane;
mod type_aware_lane;
mod write;

use std::path::{Path, PathBuf};
use std::thread;
use std::time::SystemTime;

use rayon::prelude::*;

use fmtkit_cache::{Cache, Key};
use fmtkit_config::Config;
use fmtkit_core::{Diagnostic, FileOutcome, Lane, Lang, Mode, REPORT_SCHEMA, Report, RunResult, Severity, VetOutcome};
use fmtkit_discover::{Discovery, Memory, Repository, Scope, SourceFile};
use fmtkit_go::{GoError, Helper, VetMemo, VetTargets};
use fmtkit_lint::{LintError, Linter, TypeAwareError};

pub use progress::Progress;

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error(transparent)]
    Discover(#[from] fmtkit_discover::DiscoverError),
    #[error(transparent)]
    Lint(#[from] LintError),
    #[error(transparent)]
    Go(#[from] GoError),
    #[error(transparent)]
    TypeAware(#[from] TypeAwareError),
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
        match self {
            Self::Go(_) | Self::Pool(_) => true,
            Self::TypeAware(e) => !e.is_setup(),
            _ => false,
        }
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
    let mut session = Session::default();
    let report = session.run(config, options, progress);

    session.persist();

    report
}

/// What a process that runs fmtkit many times keeps between runs. For each
/// root it has run (every work tree of a repository is one): the outcome
/// cache, the directory memory, and the git repository; a run with another
/// configuration or cache setting starts the root's stores over. Across
/// roots: the linter for the last configuration and the worker pool.
#[derive(Default)]
pub struct Session {
    roots: Vec<Root>,
    /// The linter, with the hash of the configuration it was built for.
    linter: Option<([u8; 32], Linter)>,
    pool: Option<(usize, rayon::ThreadPool)>,
    /// Whether every root is watched (see [`Session::watch`]).
    watching: bool,
}

struct Root {
    stores: Stores,
    repository: Repository,
}

struct Stores {
    root: PathBuf,
    config_hash: [u8; 32],
    enabled: bool,
    cache: Cache,
    /// Directory listings are kept beside the outcomes, and only when they are.
    memory: Option<Memory>,
}

impl Session {
    /// Run fmtkit over the files `options.scope` selects.
    pub fn run(&mut self, config: &Config, options: &Options, progress: &Progress) -> Result<Report, EngineError> {
        let config_hash = config.hash();
        let lints = selected_lanes(&options.scope).contains(&Lane::Ts);

        if self.linter.as_ref().is_some_and(|(built_for, _)| *built_for != config_hash) {
            self.linter = None;
        }

        let at = if let Some(at) = self.roots.iter().position(|root| root.stores.root == options.root) {
            let stores = &mut self.roots[at].stores;

            if (stores.config_hash, stores.enabled) != (config_hash, options.cache) {
                stores.persist();
                *stores = Stores::open(&options.root, config_hash, options.cache);
            }

            at
        } else {
            let mut repository = Repository::default();

            if self.watching {
                repository.watch();
            }

            self.roots.push(Root { stores: Stores::open(&options.root, config_hash, options.cache), repository });
            self.roots.len() - 1
        };

        let Root { stores, repository } = &mut self.roots[at];

        stores.cache.next_run();

        let now = SystemTime::now();

        if let Some(memory) = &mut stores.memory {
            memory.next_run(now);
        }

        repository.next_run(now);

        let Stores { cache, memory, .. } = &*stores;

        // The stores and the linter do not depend on the file list, so they
        // load while discovery opens the repository and walks the tree.
        let (discovery, built) = thread::scope(|s| {
            s.spawn(|| cache.load());

            let linter = (lints && self.linter.is_none()).then(|| s.spawn(|| Linter::new(&config.lint)));

            if let Some(memory) = memory {
                s.spawn(|| memory.load());
            }

            let discovery = fmtkit_discover::discover_with(&options.root, &options.scope, &config.files, memory.as_ref(), repository);

            (discovery, linter.map(join))
        });

        let discovery = discovery?;
        let (go_files, script_files): (Vec<SourceFile>, Vec<SourceFile>) = discovery.files.iter().cloned().partition(|f| f.lang == Lang::Go);

        progress.start(discovery.files.len());

        // A linter no file needs is kept for a later run; a failure to build
        // one matters only to a run that needs it.
        let failed = match built {
            Some(Ok(linter)) => {
                self.linter = Some((config_hash, linter));

                None
            }
            Some(Err(e)) => Some(e),
            None => None,
        };

        if script_files.iter().any(|f| f.lang.is_lintable()) && self.linter.is_none() {
            self.linter = Some((
                config_hash,
                match failed {
                    Some(e) => return Err(e.into()),
                    None => Linter::new(&config.lint)?,
                },
            ));
        }

        let jobs = options.jobs.max(1);

        let (_, pool) = match self.pool.take() {
            Some((threads, pool)) if threads == jobs => self.pool.insert((threads, pool)),
            _ => self.pool.insert((jobs, rayon::ThreadPoolBuilder::new().num_threads(jobs).build().map_err(|e| EngineError::Pool(e.to_string()))?)),
        };
        let pool = &*pool;

        let linter = self.linter.as_ref().map(|(_, linter)| linter).filter(|_| script_files.iter().any(|f| f.lang.is_lintable()));
        let state = Run { config, mode: options.mode, cache, linter, progress };
        let to_check = type_aware_lane::files(linter, &script_files);

        // A run that needs tsrs and cannot find it stops before writing anything.
        let tsrs = match linter.filter(|_| !to_check.is_empty()) {
            Some(linter) => Some((linter, linter.locate_tsrs(&options.root)?)),
            None => None,
        };

        // tsrs reads the files from disk: in check mode, which writes
        // nothing, it runs beside the other lanes; in format mode, after them.
        let beside = options.mode == Mode::Check;
        let type_aware = |(linter, tsrs): &(&Linter, PathBuf)| type_aware_lane::run(&options.root, linter, tsrs, &to_check);

        let (mut scripts, go, vet, types) = thread::scope(|s| {
            let vet = s.spawn(|| vet_for(config, options, cache, &discovery, &go_files));
            let go = (!go_files.is_empty()).then(|| s.spawn(|| go_lane::process(&state, options.go_helper.as_deref(), &go_files, pool)));
            let types = tsrs.as_ref().filter(|_| beside).map(|tsrs| s.spawn(|| type_aware(tsrs)));
            let scripts: Vec<FileOutcome> = pool.install(|| script_files.par_iter().filter_map(|file| script_lane::process(&state, file)).collect());

            (scripts, go.map(join), join(vet), types.map(join))
        });

        let types = types.or_else(|| tsrs.as_ref().map(type_aware));

        if let Some(types) = types {
            type_aware_lane::merge(&mut scripts, &to_check, types?);
        }

        let go = go.transpose()?.unwrap_or_default();
        let lanes = selected_lanes(&options.scope);
        let whole = options.scope.all && options.scope.paths.is_empty() && lanes.contains(&Lane::Ts);
        let orphans = if whole { stale_exemptions(&options.root, &config.lint) } else { Vec::new() };
        let mut files: Vec<FileOutcome> = scripts.into_iter().chain(go).chain(orphans).collect();

        files.sort_unstable_by(|a, b| a.file.cmp(&b.file));

        let complexity = complexity::evaluate(&options.root, &config.complexity, &lanes, &discovery.files, &files);

        progress.finish();

        // A path the user named that does not exist fails the run, as it did in 0.x.
        let result = if discovery.missing.is_empty() { verdict(options.mode, &files, complexity.is_empty(), &vet) } else { RunResult::Fail };

        Ok(Report { schema: REPORT_SCHEMA, mode: options.mode, result, files, complexity, vet, missing: discovery.missing })
    }

    /// Watch every root between runs, the ones to come included, so each run
    /// looks only at what changed since the one before (see
    /// [`Repository::watch`]).
    pub fn watch(&mut self) {
        self.watching = true;

        for root in &mut self.roots {
            root.repository.watch();
        }
    }

    /// Why watching a root stopped, once per failure.
    pub fn watch_failure(&mut self) -> Option<String> {
        self.roots.iter_mut().find_map(|root| root.repository.watch_failure())
    }

    /// Forget the roots that are no longer directories (a work tree that was
    /// removed), after writing what their runs learned, and release their
    /// watches; the roots forgotten.
    pub fn forget_missing(&mut self) -> Vec<PathBuf> {
        let (gone, kept): (Vec<Root>, Vec<Root>) = self.roots.drain(..).partition(|root| !root.stores.root.is_dir());

        self.roots = kept;

        gone.into_iter()
            .map(|mut root| {
                root.stores.persist();
                root.stores.root
            })
            .collect()
    }

    /// Write what the runs since the last call learned.
    pub fn persist(&mut self) {
        for root in &mut self.roots {
            root.stores.persist();
        }
    }
}

impl Stores {
    fn open(root: &Path, config_hash: [u8; 32], enabled: bool) -> Self {
        let cache = Cache::open(root, config_hash, enabled);
        let memory = cache.path().map(|path| Memory::open(path.with_extension("dirs")));

        Self { root: root.to_path_buf(), config_hash, enabled, cache, memory }
    }

    /// A failed write only costs a later process its warm start.
    fn persist(&mut self) {
        let _ = self.cache.flush();

        if let Some(memory) = &mut self.memory {
            let _ = memory.save();
        }
    }
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

fn vet_for(config: &Config, options: &Options, cache: &Cache, discovery: &Discovery, go_files: &[SourceFile]) -> VetOutcome {
    if !config.go.vet {
        return VetOutcome { skipped: Some("disabled by [go] vet".into()), ..VetOutcome::default() };
    }

    if !selected_lanes(&options.scope).contains(&Lane::Go) {
        return VetOutcome { skipped: Some("the Go lane is not selected".into()), ..VetOutcome::default() };
    }

    let every_module = options.scope.all && options.scope.paths.is_empty();

    if go_files.is_empty() && !(every_module && discovery.git) {
        return VetOutcome { skipped: Some("no Go files in scope".into()), ..VetOutcome::default() };
    }

    let targets =
        if every_module { VetTargets::Modules(discovery.modules.clone()) } else { VetTargets::Files(go_files.iter().map(|f| f.rel.clone()).collect()) };

    let memo = cache.is_enabled().then_some(Marks(cache));

    fmtkit_go::vet(&options.root, &targets, memo.as_ref().map(|memo| memo as &dyn VetMemo), options.jobs.max(1))
}

/// Vet passes kept as cache marks.
struct Marks<'a>(&'a Cache);

impl VetMemo for Marks<'_> {
    fn passed(&self, key: &[u8; 32]) -> bool {
        self.0.marked(&Key(*key))
    }

    fn pass(&self, key: [u8; 32]) {
        self.0.mark(Key(key));
    }
}

/// The rule a stale `[[lint.exempt]]` path is reported under.
const STALE_EXEMPTION_RULE: &str = "lint/exempt";

/// One error per exact exemption path (a pattern with no glob) that names
/// nothing in the repository, so the list only shrinks. Glob patterns are not
/// judged.
fn stale_exemptions(root: &Path, lint: &fmtkit_config::Lint) -> Vec<FileOutcome> {
    let mut stale: std::collections::BTreeMap<&str, Vec<Diagnostic>> = std::collections::BTreeMap::new();

    for entry in &lint.exempt {
        for path in entry.exact_paths().filter(|path| !path.is_empty() && !root.join(path).exists()) {
            stale.entry(path).or_default().push(Diagnostic {
                rule: STALE_EXEMPTION_RULE.to_owned(),
                file: path.to_owned(),
                line: 0,
                column: 0,
                message: format!("exempt entry for {:?} names a path that does not exist", entry.rule),
                severity: Severity::Error,
            });
        }
    }

    stale.into_iter().map(|(path, lint)| FileOutcome { lint, ..FileOutcome::new(path, Lang::from_path(Path::new(path))) }).collect()
}

/// Join a scoped thread, re-raising its panic here.
fn join<T>(handle: thread::ScopedJoinHandle<'_, T>) -> T {
    handle.join().unwrap_or_else(|panic| std::panic::resume_unwind(panic))
}

fn selected_lanes(scope: &Scope) -> Vec<Lane> {
    if scope.lanes.is_empty() { vec![Lane::Ts, Lane::Go] } else { scope.lanes.clone() }
}

/// Lint warnings are reported but never fail a run, as oxlint's own exit code
/// treats them; only `error` rules do.
fn verdict(mode: Mode, files: &[FileOutcome], complexity_clean: bool, vet: &VetOutcome) -> RunResult {
    let unresolved =
        files.iter().any(|f| f.error.is_some() || f.lint.iter().any(|d| d.severity == Severity::Error) || (mode == Mode::Check && !f.violations.is_empty()));

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

    #[test]
    fn only_lint_errors_fail_a_run() {
        let vet = VetOutcome::default();
        let lint = |severity| FileOutcome {
            lint: vec![Diagnostic { rule: "eslint/eqeqeq".into(), file: "a.ts".into(), line: 1, column: 1, message: "m".into(), severity }],
            ..FileOutcome::new("a.ts", Some(Lang::Ts))
        };

        assert_eq!(verdict(Mode::Check, &[lint(Severity::Warning)], true, &vet), RunResult::Pass);
        assert_eq!(verdict(Mode::Check, &[lint(Severity::Error)], true, &vet), RunResult::Fail);
        assert_eq!(verdict(Mode::Format, &[lint(Severity::Error)], true, &vet), RunResult::Fail);
    }
}
