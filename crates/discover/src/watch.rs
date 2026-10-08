//! Watching a work tree between runs, for a process that runs fmtkit many
//! times (`fmtkit serve --watch`).
//!
//! The kernel reports each change to a watched file or directory as it is
//! made: kqueue on macOS, where every watched file and directory holds a
//! descriptor, and inotify on Linux, where a watched directory reports the
//! changes to its entries as well as its own. A run first takes what was
//! reported since the last one, then looks only at what may have changed:
//!
//! - An index entry found clean by an earlier run under the same index, whose
//!   file and every directory above it have been watched since, and that
//!   nothing was reported for, is still clean; only the other entries are
//!   compared with their files.
//! - A directory confirmed by an earlier walk and watched since, with nothing
//!   reported for it or its `.gitignore`, replays its stored listing without
//!   being looked at.
//!
//! A watch is in place before what it covers is looked at, so a change made
//! after the look is reported to the next run. A run that finds more reports
//! than are worth sorting, or a kernel that dropped some, starts the watch
//! over and looks at everything.

#[cfg(target_os = "linux")]
mod inotify;
#[cfg(target_os = "macos")]
mod kqueue;

use std::collections::{HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

#[cfg(target_os = "linux")]
use inotify::Watcher;
#[cfg(target_os = "macos")]
use kqueue::Watcher;

use crate::memory::Stamp;

/// Past this many reports in one run, looking at everything is cheaper
/// than sorting them out.
const REPORTS: usize = 2048;

/// What a watched path is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    File,
    Dir,
}

/// What was reported.
#[derive(Debug, Default)]
pub(crate) struct Touched {
    /// Files and directories that changed; a directory changes when an
    /// entry is added, removed, or renamed, or its own attributes change.
    pub paths: HashSet<String>,
    /// Directories removed or renamed: the paths below them may name other
    /// files now.
    pub subtrees: Vec<String>,
}

impl Touched {
    fn len(&self) -> usize {
        self.paths.len() + self.subtrees.len()
    }
}

/// What a walk saw of one directory: its stamp, and its `.gitignore`'s.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Sighting {
    pub stamp: Stamp,
    pub rules: Option<Stamp>,
}

/// The index entries to look at again, for the index they were found in.
pub(crate) struct Baseline {
    /// The index's checksum and timestamp.
    pub index: Vec<u8>,
    /// Positions of the entries found suspect, or not watched.
    pub recheck: Vec<usize>,
}

/// A watch on one work tree, kept from one run to the next.
pub(crate) struct Watch {
    pub watcher: Watcher,
    /// Directories whose stored listing holds while nothing is reported for
    /// them, with what the walk that confirmed them saw.
    pub clean: HashMap<String, Sighting>,
    pub baseline: Option<Baseline>,
    /// What was reported and not yet compared with the index: a run that
    /// does not compare it (`--all`) leaves it to the next.
    pub touched: Touched,
    /// Why the watch failed, once it has; the run goes on without it.
    pub failure: Option<String>,
}

/// The part of a [`Watch`] that follows the index entries.
pub(crate) struct Tracking<'w> {
    pub watcher: &'w mut Watcher,
    pub baseline: &'w mut Option<Baseline>,
    pub touched: &'w mut Touched,
    pub failure: &'w mut Option<String>,
}

/// A walk's part of a [`Watch`]: the directories it may replay without
/// looking, and what it saw of the ones it looked at.
pub(crate) struct Sight<'w> {
    clean: &'w HashMap<String, Sighting>,
    seen: Mutex<Vec<(String, Sighting)>>,
}

impl Watch {
    pub(crate) fn new() -> io::Result<Self> {
        Ok(Self { watcher: Watcher::new()?, clean: HashMap::new(), baseline: None, touched: Touched::default(), failure: None })
    }

    /// Take what was reported since the last run, and forget what it
    /// invalidates.
    pub(crate) fn begin(&mut self) -> io::Result<()> {
        let Some(touched) = self.watcher.drain() else {
            // Every watch may have missed something: start over.
            self.watcher = Watcher::new()?;
            let _ = self.watcher.drain();
            self.forget();

            return Ok(());
        };

        if self.touched.len() + touched.len() > REPORTS {
            self.forget();

            return Ok(());
        }

        for path in &touched.paths {
            self.clean.remove(path);

            if let Some(dir) = ruled(path) {
                self.clean.remove(dir);
            }
        }

        if !touched.subtrees.is_empty() {
            self.clean.retain(|rel, _| !touched.subtrees.iter().any(|subtree| is_under(rel, subtree)));
        }

        self.touched.paths.extend(touched.paths);
        self.touched.subtrees.extend(touched.subtrees);

        Ok(())
    }

    /// Look at everything again.
    fn forget(&mut self) {
        self.clean.clear();
        self.baseline = None;
        self.touched = Touched::default();
    }

    /// Split into the parts the index comparison and the walk use at once.
    pub(crate) fn split(&mut self) -> (Tracking<'_>, Sight<'_>) {
        let Self { watcher, clean, baseline, touched, failure } = self;

        (Tracking { watcher, baseline, touched, failure }, Sight { clean, seen: Mutex::new(Vec::new()) })
    }

    /// Watch each directory a walk looked at, and count it clean once its
    /// watch is known to have been in place since the look.
    pub(crate) fn settle(&mut self, root: &Path, seen: Vec<(String, Sighting)>) {
        let mut fresh = Vec::new();

        for (rel, sighting) in seen {
            // Watched before the run began: anything since the look is reported.
            if self.watcher.watched_before(&rel, Kind::Dir) && sighting.rules.is_none_or(|_| self.watcher.watched_before(&rules_of(&rel), Kind::File)) {
                self.clean.insert(rel, sighting);
            } else {
                fresh.push((rel, sighting));
            }
        }

        if fresh.is_empty() {
            return;
        }

        let paths: Vec<_> =
            fresh.iter().flat_map(|(rel, sighting)| [Some((rel.clone(), Kind::Dir)), sighting.rules.map(|_| (rules_of(rel), Kind::File))]).flatten().collect();

        if let Err(err) = self.watcher.add(root, paths.iter().map(|(rel, kind)| (rel.as_str(), *kind))) {
            self.failure = Some(err.to_string());

            return;
        }

        // Watched only now: the directory counts as clean if it still is what
        // the walk saw.
        for (rel, sighting) in fresh {
            let dir = absolute(root, &rel);

            if self.watcher.watches(&rel, Kind::Dir)
                && Stamp::of(&dir) == Some(sighting.stamp)
                && sighting.rules.is_none_or(|rules| self.watcher.watches(&rules_of(&rel), Kind::File) && Stamp::of(&dir.join(".gitignore")) == Some(rules))
            {
                self.clean.insert(rel, sighting);
            }
        }
    }
}

impl Tracking<'_> {
    /// Watch `rels`, files, and the directories above each, recording a
    /// failure; whether the watch still stands.
    pub(crate) fn watch_files<'r>(&mut self, root: &Path, rels: impl Iterator<Item = &'r str> + Clone) -> bool {
        let mut dirs = HashSet::new();

        for rel in rels.clone() {
            dirs.extend(ancestors(rel));
        }

        let paths = dirs.into_iter().map(|dir| (dir, Kind::Dir)).chain(rels.map(|rel| (rel, Kind::File)));

        if let Err(err) = self.watcher.add(root, paths) {
            *self.failure = Some(err.to_string());

            return false;
        }

        true
    }

    /// Whether a change to the file at `rel` would be reported: it and every
    /// directory above it are watched.
    pub(crate) fn covers(&self, rel: &str) -> bool {
        self.watcher.watches(rel, Kind::File) && ancestors(rel).all(|dir| self.watcher.watches(dir, Kind::Dir))
    }

    /// Stop watching files that are neither `tracked` nor a `.gitignore`.
    pub(crate) fn keep_files(&mut self, tracked: &HashSet<&str>) {
        self.watcher.retain_files(|rel| tracked.contains(rel) || ruled(rel).is_some());
    }
}

impl Sight<'_> {
    /// What an earlier walk saw of `rel`, when nothing changed it since.
    pub(crate) fn clean(&self, rel: &str) -> Option<&Sighting> {
        self.clean.get(rel)
    }

    /// Record what the walk saw of `rel`.
    pub(crate) fn saw(&self, rel: &str, sighting: Sighting) {
        self.seen.lock().unwrap_or_else(PoisonError::into_inner).push((rel.to_owned(), sighting));
    }

    pub(crate) fn into_seen(self) -> Vec<(String, Sighting)> {
        self.seen.into_inner().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The directories above the path `rel`, from the root (`""`) down.
fn ancestors(rel: &str) -> impl Iterator<Item = &str> {
    std::iter::once("").chain(rel.match_indices('/').map(|(at, _)| &rel[..at]))
}

/// Whether `rel` is `dir` or lies below it.
pub(crate) fn is_under(rel: &str, dir: &str) -> bool {
    dir.is_empty() || rel.strip_prefix(dir).is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
}

/// The directory whose rules `rel` holds, when it is a `.gitignore`.
fn ruled(rel: &str) -> Option<&str> {
    if rel == ".gitignore" { Some("") } else { rel.strip_suffix("/.gitignore") }
}

/// The `.gitignore` of the directory `rel`.
fn rules_of(rel: &str) -> String {
    if rel.is_empty() { ".gitignore".to_owned() } else { format!("{rel}/.gitignore") }
}

fn absolute(root: &Path, rel: &str) -> PathBuf {
    if rel.is_empty() { root.to_path_buf() } else { root.join(rel) }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
mod unsupported {
    use std::io;
    use std::path::Path;

    use super::{Kind, Touched};

    /// No watcher on this platform: [`Watcher::new`] fails.
    pub(crate) enum Watcher {}

    impl Watcher {
        pub(crate) fn new() -> io::Result<Self> {
            Err(io::Error::new(io::ErrorKind::Unsupported, "watching needs kqueue (macOS) or inotify (Linux)"))
        }

        pub(crate) fn add<'r>(&mut self, _root: &Path, _paths: impl IntoIterator<Item = (&'r str, Kind)>) -> io::Result<()> {
            match *self {}
        }

        pub(crate) fn watches(&self, _rel: &str, _kind: Kind) -> bool {
            match *self {}
        }

        pub(crate) fn watched_before(&self, _rel: &str, _kind: Kind) -> bool {
            match *self {}
        }

        pub(crate) fn retain_files(&mut self, _keep: impl Fn(&str) -> bool) {
            match *self {}
        }

        pub(crate) fn drain(&mut self) -> Option<Touched> {
            match *self {}
        }
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
use unsupported::Watcher;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ancestors_run_from_the_root_down() {
        assert_eq!(ancestors("a/b/c.ts").collect::<Vec<_>>(), ["", "a", "a/b"]);
        assert_eq!(ancestors("c.ts").collect::<Vec<_>>(), [""]);
    }

    #[test]
    fn a_path_is_under_itself_and_its_directories() {
        assert!(is_under("a/b", "a"));
        assert!(is_under("a", "a"));
        assert!(is_under("a", ""));
        assert!(!is_under("ab", "a"));
        assert!(!is_under("a", "a/b"));
    }

    #[test]
    fn a_gitignore_names_its_directory() {
        assert_eq!(ruled(".gitignore"), Some(""));
        assert_eq!(ruled("a/.gitignore"), Some("a"));
        assert_eq!(ruled("a/b.ts"), None);
        assert_eq!(rules_of(""), ".gitignore");
        assert_eq!(rules_of("a"), "a/.gitignore");
    }
}
