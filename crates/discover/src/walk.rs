mod git;

use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use ignore::{WalkBuilder, WalkState};

use crate::filter::Filter;
use crate::memory::Memory;
use crate::watch::Sight;
use crate::{DiscoverError, Walked};

/// Which ignore rules a walk follows.
#[derive(Debug, Clone)]
pub(crate) enum Rules {
    /// `.gitignore` and `.ignore` files, wherever they are. Used outside git.
    Plain,
    /// Git's: `.gitignore` files, `exclude` (the repository's
    /// `info/exclude`), and the global excludes file, matched
    /// case-insensitively under `core.ignoreCase`. Nested repositories and
    /// submodules are not entered.
    Git { ignore_case: bool, exclude: PathBuf },
}

/// Every file under `root` that `.gitignore` and `.ignore` files do not
/// exclude, walked in parallel. Used outside git.
pub(crate) fn all(root: &Path, filter: &Arc<Filter>) -> Result<Walked, DiscoverError> {
    walk(root, filter, &Rules::Plain, None, None, |_| true)
}

/// Walk `root` in parallel, recording every file that `rules` do not exclude
/// and `keep` accepts. Directories the filter rejects are not entered. Under
/// git's rules, `memory` replays the listings of unchanged directories, and
/// of the directories a watch vouches for (`sight`) without looking at them.
pub(crate) fn walk(
    root: &Path,
    filter: &Arc<Filter>,
    rules: &Rules,
    memory: Option<&Memory>,
    sight: Option<&Sight<'_>>,
    keep: impl Fn(&str) -> bool + Sync,
) -> Result<Walked, DiscoverError> {
    match rules {
        Rules::Plain => plain(root, filter, keep),
        Rules::Git { ignore_case, exclude } => git::walk(root, filter, *ignore_case, exclude, memory, sight, walkers(), &keep),
    }
}

fn plain(root: &Path, filter: &Arc<Filter>, keep: impl Fn(&str) -> bool + Sync) -> Result<Walked, DiscoverError> {
    let found = Mutex::new(Walked::default());
    let failure = Mutex::new(None);
    let prune = Arc::clone(filter);
    let base = root.to_path_buf();

    WalkBuilder::new(root)
        .hidden(false)
        .parents(false)
        .follow_links(false)
        .require_git(false)
        .threads(walkers())
        .filter_entry(move |entry| {
            if entry.depth() == 0 || !entry.file_type().is_some_and(|kind| kind.is_dir()) {
                return true;
            }

            relative(&base, entry.path()).is_some_and(|rel| prune.enters(rel))
        })
        .build_parallel()
        .run(|| {
            let mut sink = Sink { walked: Walked::default(), found: &found };
            let failure = &failure;
            let keep = &keep;

            Box::new(move |result| {
                let entry = match result {
                    Ok(entry) => entry,
                    Err(err) if err.io_error().is_some_and(|err| err.kind() == io::ErrorKind::NotFound) => return WalkState::Continue,
                    Err(err) => {
                        failure.lock().unwrap_or_else(PoisonError::into_inner).get_or_insert(err);

                        return WalkState::Quit;
                    }
                };

                if entry.file_type().is_some_and(|kind| kind.is_file())
                    && let Some(rel) = relative(root, entry.path())
                    && keep(rel)
                {
                    sink.walked.see(filter, rel);
                }

                WalkState::Continue
            })
        });

    if let Some(err) = failure.into_inner().unwrap_or_else(PoisonError::into_inner) {
        return Err(DiscoverError::Walk { path: root.to_path_buf(), message: err.to_string() });
    }

    Ok(found.into_inner().unwrap_or_else(PoisonError::into_inner))
}

/// Directory reads contend in the kernel past a handful of threads: on a
/// 16-core machine six walkers finish sooner than the twelve `ignore` picks.
fn walkers() -> usize {
    std::thread::available_parallelism().map_or(1, |n| n.get().min(6))
}

fn relative<'a>(root: &Path, path: &'a Path) -> Option<&'a str> {
    path.strip_prefix(root).ok()?.to_str()
}

/// One walker thread's finds, handed over when the thread's visitor is dropped.
struct Sink<'a> {
    walked: Walked,
    found: &'a Mutex<Walked>,
}

impl Drop for Sink<'_> {
    fn drop(&mut self) {
        let mut found = self.found.lock().unwrap_or_else(PoisonError::into_inner);

        found.files.append(&mut self.walked.files);
        found.modules.append(&mut self.walked.modules);
    }
}
