use std::collections::{HashMap, HashSet};
use std::fs;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::SystemTime;

use gix::bstr::{BStr, BString, ByteSlice};
use gix::diff::index::ChangeRef;
use gix::index::entry::Mode;
use gix::status::UntrackedFiles;
use gix::status::index_worktree::Item;
use gix::status::plumbing::index_as_worktree::{Change, EntryStatus};
use gix::status::tree_index::TrackRenames;

use crate::filter::Filter;
use crate::memory::{Memory, Staged, Stamp, settled_before};
use crate::walk::{self, Rules};
use crate::watch::{Baseline, Sight, Touched, Tracking, Watch};
use crate::{DiscoverError, SourceFile, Walked};

/// Tracked files that differ from `HEAD` in the index or the worktree, plus
/// untracked files that are not ignored. Files deleted from the worktree are
/// left out; a rename contributes its new path.
///
/// The three comparisons (`HEAD` against the index, the index against the
/// worktree, and the untracked walk) run on their own threads.
pub(crate) fn changed(repository: &mut Repository, root: &Path, filter: &Arc<Filter>, memory: Option<&Memory>) -> Result<Vec<SourceFile>, DiscoverError> {
    let repo = repository.open(root)?;
    let index = repo.index_or_empty().map_err(git)?;
    let patterns = filter.pathspecs();
    let shared = repo.clone().into_sync();
    let ignore_case = ignore_case(&repo);
    let rules = rules(&repo, ignore_case);
    let mut watch = repository.watched(root);
    let (mut tracking, sight) = watch.as_deref_mut().map(Watch::split).unzip();
    // A listing is replayed from memory only.
    let sight = sight.filter(|_| memory.is_some());

    // The worktree comparison is the longest of the three; the tracked set
    // only the walk needs is built on the walk's thread.
    let (staged, worktree, untracked) = thread::scope(|scope| {
        let staged = scope.spawn(|| staged(&shared.to_thread_local(), &index, filter, memory));
        let untracked = scope.spawn(|| {
            let tracked = Tracked::new(&index, ignore_case);

            walk::walk(root, filter, &rules, memory, sight.as_ref(), |rel| !tracked.visit(rel)).map(|walked| walked.files)
        });
        let worktree = worktree(&repo, &index, &patterns, filter, tracking.as_mut());

        (join(staged), worktree, join(untracked))
    });

    if let (Some(seen), Some(watch)) = (sight.map(Sight::into_seen), watch) {
        watch.settle(root, seen);
    }

    let (mut files, removed) = worktree?;

    files.extend(staged?);
    files.extend(untracked?);

    if !removed.is_empty() {
        files.retain(|file| !removed.contains(&file.rel));
    }

    Ok(files)
}

/// Every tracked file present in the worktree plus every untracked file that
/// is not ignored, from one parallel directory walk. Tracked files the walk
/// does not reach (ignored ones, or ones under an ignored directory) are
/// looked up one by one. When the scope names only tracked files, nothing is
/// walked.
pub(crate) fn all(repository: &mut Repository, root: &Path, filter: &Arc<Filter>, memory: Option<&Memory>) -> Result<Walked, DiscoverError> {
    let repo = repository.open(root)?;
    let index = repo.index_or_empty().map_err(git)?;
    let tracked = Tracked::new(&index, ignore_case(&repo));

    if let Some(prefixes) = filter.prefixes()
        && prefixes.iter().all(|rel| tracked.contains(rel))
    {
        let files = prefixes.iter().filter_map(|rel| filter.source_file(rel)).filter(|file| is_regular(&file.abs)).collect();

        return Ok(Walked { files, modules: Vec::new() });
    }

    let mut watch = repository.watched(root).filter(|_| memory.is_some());
    let sight = watch.as_deref_mut().map(|watch| watch.split().1);

    let mut walked = walk::walk(root, filter, &rules(&repo, tracked.ignore_case), memory, sight.as_ref(), |rel| {
        tracked.visit(rel);

        true
    })?;

    if let (Some(seen), Some(watch)) = (sight.map(Sight::into_seen), watch) {
        watch.settle(root, seen);
    }

    for rel in tracked.unvisited() {
        if let Some(file) = filter.source_file(rel)
            && is_regular(&file.abs)
        {
            walked.files.push(file);
        }
    }

    Ok(walked)
}

/// The index's regular files, each flagged once a walk passes it.
struct Tracked<'i> {
    files: HashMap<String, (&'i str, AtomicBool)>,
    ignore_case: bool,
}

impl<'i> Tracked<'i> {
    fn new(index: &'i gix::index::State, ignore_case: bool) -> Self {
        let files = index
            .entries()
            .iter()
            .filter(|entry| is_file(entry.mode))
            .filter_map(|entry| entry.path(index).to_str().ok())
            .map(|rel| (if ignore_case { rel.to_ascii_lowercase() } else { rel.to_owned() }, (rel, AtomicBool::new(false))))
            .collect();

        Self { files, ignore_case }
    }

    /// Flag `rel` as passed; whether it is tracked.
    fn visit(&self, rel: &str) -> bool {
        self.get(rel).inspect(|(_, visited)| visited.store(true, Ordering::Relaxed)).is_some()
    }

    fn contains(&self, rel: &str) -> bool {
        self.get(rel).is_some()
    }

    fn get(&self, rel: &str) -> Option<&(&'i str, AtomicBool)> {
        if self.ignore_case { self.files.get(&rel.to_ascii_lowercase()) } else { self.files.get(rel) }
    }

    fn unvisited(&self) -> impl Iterator<Item = &'i str> {
        self.files.values().filter(|(_, visited)| !visited.load(Ordering::Relaxed)).map(|(rel, _)| *rel)
    }
}

/// Git's ignore rules for this repository.
fn rules(repo: &gix::Repository, ignore_case: bool) -> Rules {
    Rules::Git { ignore_case, exclude: repo.common_dir().join("info").join("exclude") }
}

/// Whether git compares paths case-insensitively here (`core.ignoreCase`).
fn ignore_case(repo: &gix::Repository) -> bool {
    repo.filesystem_options().is_ok_and(|caps| caps.ignore_case)
}

/// A git repository kept open from one run to the next.
///
/// gix reads loose refs and objects as they are needed, but reads the
/// configuration only when it opens the repository, and rereads the index
/// and packed refs only once their modification time changes, which a
/// coarse timestamp may not show. A kept repository is therefore reopened
/// once any of those files changes, or a configuration file appears where
/// git would read one. One whose files changed within
/// [`SETTLE`](crate::SETTLE) of the run is not kept, since a coarse
/// timestamp could miss a second change in the same tick.
#[derive(Default)]
pub struct Repository {
    /// Nanoseconds since the epoch before which a stamp is settled; `None`
    /// until [`Repository::next_run`], and nothing is kept until then.
    settled: Option<i64>,
    kept: Option<Kept>,
    watching: bool,
    /// The watch on the work tree at the path, while watching.
    watch: Option<(PathBuf, Watch)>,
    /// Why watching stopped, until asked.
    failure: Option<String>,
}

struct Kept {
    root: PathBuf,
    repo: gix::ThreadSafeRepository,
    /// [`sources`], with their stamps when the repository was opened.
    sources: Vec<(PathBuf, Option<Stamp>)>,
}

impl Repository {
    /// Begin another run taking place at `now`, which decides what has
    /// settled; the repository is kept for the runs after it.
    pub fn next_run(&mut self, now: SystemTime) {
        self.settled = Some(settled_before(now));
    }

    /// Watch the work tree from now on, so each run compares and lists only
    /// what changed since the one before; see [`crate::watch`]. Where the
    /// platform has no watcher, or the watch fails, runs look at everything
    /// again, and [`Repository::watch_failure`] says why.
    pub fn watch(&mut self) {
        self.watching = true;
    }

    /// Why watching stopped, once.
    pub fn watch_failure(&mut self) -> Option<String> {
        self.stop_failed_watch();
        self.failure.take()
    }

    fn stop_failed_watch(&mut self) {
        if let Some((_, watch)) = &mut self.watch
            && let Some(failure) = watch.failure.take()
        {
            self.stop_watching(failure);
        }
    }

    fn stop_watching(&mut self, failure: String) {
        self.watching = false;
        self.watch = None;
        self.failure = Some(failure);
    }

    /// The watch on the work tree at `root`, with what was reported since
    /// the last run taken; `None` when not watching.
    fn watched(&mut self, root: &Path) -> Option<&mut Watch> {
        self.stop_failed_watch();

        if !self.watching {
            return None;
        }

        if self.watch.as_ref().is_some_and(|(watched, _)| watched != root) {
            self.watch = None;
        }

        if self.watch.is_none() {
            match Watch::new() {
                Ok(watch) => self.watch = Some((root.to_path_buf(), watch)),
                Err(err) => {
                    self.stop_watching(err.to_string());

                    return None;
                }
            }
        }

        let begun = self.watch.as_mut().map(|(_, watch)| watch.begin());

        if let Some(Err(err)) = begun {
            self.stop_watching(err.to_string());

            return None;
        }

        self.watch.as_mut().map(|(_, watch)| watch)
    }

    /// The repository at `root`: the kept one while its files are
    /// unchanged, else a newly opened one.
    fn open(&mut self, root: &Path) -> Result<gix::Repository, DiscoverError> {
        if let Some(kept) = &self.kept
            && kept.root == root
            && kept.sources.iter().all(|(path, stamp)| Stamp::of(path) == *stamp)
        {
            return Ok(kept.repo.to_thread_local());
        }

        self.kept = None;

        let repo = open(root)?;

        // The stamps are taken after the configuration was read, and before
        // the index is: a file that changed in between has not settled, so
        // nothing stale is kept.
        if let Some(settled) = self.settled
            && cfg!(unix)
        {
            let sources: Vec<_> = sources(root, &repo).into_iter().map(|path| (Stamp::of(&path), path)).map(|(stamp, path)| (path, stamp)).collect();

            if sources.iter().all(|(_, stamp)| stamp.is_none_or(|stamp| stamp.is_settled(settled))) {
                self.kept = Some(Kept { root: root.to_path_buf(), repo: repo.clone().into_sync(), sources });
            }
        }

        Ok(repo)
    }
}

/// The files a kept `repo` depends on: the index and packed refs; the
/// files its configuration was read from, and the ones it would be read from
/// if they existed (the system, global, and repository files, and any file
/// they include); `HEAD`, which an `includeIf "onbranch:"` follows; and a
/// linked worktree's `.git` file. git replaces the index and packed refs by
/// renaming a new file over them, so every write changes their stamp.
fn sources(root: &Path, repo: &gix::Repository) -> Vec<PathBuf> {
    let mut env = |name: &str| std::env::var_os(name);
    let dot_git = root.join(".git");
    let mut paths: Vec<PathBuf> = [gix::config::Source::System, gix::config::Source::Git, gix::config::Source::User]
        .into_iter()
        .filter_map(|source| source.storage_location(&mut env))
        .chain([
            repo.index_path(),
            repo.common_dir().join("packed-refs"),
            repo.common_dir().join("config"),
            repo.git_dir().join("config.worktree"),
            repo.git_dir().join("HEAD"),
        ])
        .chain(dot_git.is_file().then_some(dot_git))
        .chain(repo.config_snapshot().plumbing().sections().filter_map(|section| section.meta().path.clone()))
        .collect();

    paths.sort_unstable();
    paths.dedup();
    paths
}

/// Open the repository at `root`. The index checksum is not verified, as
/// `git` itself verifies it only in `git fsck`; it is half the cost of
/// reading the index.
fn open(root: &Path) -> Result<gix::Repository, DiscoverError> {
    gix::open_opts(root, gix::open::Options::default().config_overrides(["index.skipHash=true"])).map_err(git)
}

#[allow(clippy::needless_pass_by_value)]
fn git(err: impl ToString) -> DiscoverError {
    DiscoverError::Git(err.to_string())
}

fn join<T>(handle: thread::ScopedJoinHandle<'_, T>) -> T {
    handle.join().unwrap_or_else(|payload| std::panic::resume_unwind(payload))
}

/// Whether `path` is a regular file, not following a symbolic link.
fn is_regular(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|meta| meta.is_file())
}

fn is_file(mode: Mode) -> bool {
    mode.contains(Mode::FILE)
}

fn push(files: &mut Vec<SourceFile>, filter: &Filter, rela_path: &BStr) {
    if let Ok(rel) = rela_path.to_str()
        && let Some(file) = filter.source_file(rel)
    {
        files.push(file);
    }
}

/// Entries added or modified in the index against `HEAD`. `memory` keeps the
/// result for the next run with the same tree and index.
fn staged(repo: &gix::Repository, index: &gix::index::File, filter: &Filter, memory: Option<&Memory>) -> Result<Vec<SourceFile>, DiscoverError> {
    let tree = repo.head_tree_id_or_empty().map_err(git)?.detach();
    let key = memory.zip(index.checksum()).map(|(memory, checksum)| (memory, tree.as_bytes().to_vec(), checksum.as_bytes().to_vec()));
    let mut files = Vec::new();

    if let Some(paths) = key.as_ref().and_then(|(memory, tree, index)| memory.staged(tree, index)) {
        for path in paths {
            push(&mut files, filter, path.as_bytes().as_bstr());
        }

        return Ok(files);
    }

    let mut paths = Vec::new();

    repo.tree_index_status(&tree, index, None, TrackRenames::Disabled, |change, _, _| {
        match change {
            ChangeRef::Addition { location, entry_mode, .. }
            | ChangeRef::Modification { location, entry_mode, .. }
            | ChangeRef::Rewrite { location, entry_mode, .. }
                if is_file(entry_mode) =>
            {
                if let Ok(rel) = location.to_str() {
                    paths.push(rel.to_owned());
                }
            }
            _ => {}
        }

        Ok(ControlFlow::Continue(()))
    })
    .map_err(git)?;

    for path in &paths {
        push(&mut files, filter, path.as_bytes().as_bstr());
    }

    if let Some((memory, tree, index)) = key {
        memory.remember_staged(Staged { tree, index, paths });
    }

    Ok(files)
}

/// Past this many entries that stat alone cannot vouch for, gix checks the
/// whole index rather than a pathspec per entry.
const SUSPECTS: usize = 256;

/// Threads comparing the index with the worktree; file status calls contend
/// in the kernel past a handful.
const STATTERS: usize = 4;

/// Tracked files whose worktree copy differs from the index, and the paths
/// that no longer exist as files in the worktree.
///
/// Every tracked file is first compared with its index entry by stat alone,
/// in parallel. An entry whose stat matches exactly, and that was written
/// before the index was, is clean, as git assumes; only the rest go to gix,
/// which reads a file's content when its stat cannot decide.
fn worktree(
    repo: &gix::Repository,
    index: &gix::worktree::Index,
    patterns: &[BString],
    filter: &Filter,
    tracking: Option<&mut Tracking<'_>>,
) -> Result<(Vec<SourceFile>, HashSet<String>), DiscoverError> {
    if patterns.is_empty()
        && let Some(suspects) = repo.workdir().and_then(|workdir| suspects(workdir, index, tracking))
        && suspects.len() <= SUSPECTS
    {
        if suspects.is_empty() {
            return Ok((Vec::new(), HashSet::new()));
        }

        let specs: Vec<BString> = suspects.into_iter().map(|path| [b":(top,literal)".as_slice(), path.as_ref()].concat().into()).collect();

        return status(repo, index, &specs, filter);
    }

    status(repo, index, patterns, filter)
}

/// The paths of the index entries whose worktree copy stat cannot vouch for,
/// or `None` where the platform keeps no comparable stat data.
fn suspects<'i>(workdir: &Path, index: &'i gix::index::File, tracking: Option<&mut Tracking<'_>>) -> Option<Vec<&'i BStr>> {
    if !cfg!(unix) {
        return None;
    }

    if let Some(tracking) = tracking
        && let Some(suspects) = watched_suspects(workdir, index, tracking)
    {
        return Some(suspects);
    }

    let entries: Vec<_> = index.entries().iter().collect();

    Some(entries.iter().zip(vouch(workdir, index, &entries)).filter(|(_, vouched)| !vouched).map(|(entry, _)| entry.path(index)).collect())
}

/// [`suspects`] under a watch, which looks only at the entries the watch
/// cannot vouch for: the ones the last run under the same index found
/// suspect or could not watch, and the ones a change was reported for. Each
/// entry looked at, and every directory above it, is watched before the
/// look. `None` when the index has no checksum, or the watch failed.
fn watched_suspects<'i>(workdir: &Path, index: &'i gix::index::File, tracking: &mut Tracking<'_>) -> Option<Vec<&'i BStr>> {
    let touched = std::mem::take(tracking.touched);

    let Some(key) = index_key(index) else {
        *tracking.baseline = None;

        return None;
    };

    let entries = index.entries();
    let same = tracking.baseline.as_ref().is_some_and(|baseline| baseline.index == key);

    let picked: Vec<usize> = match tracking.baseline.as_ref() {
        Some(baseline) if same => {
            let mut picked = baseline.recheck.clone();

            picked.extend(reported(index, &touched));
            picked.sort_unstable();
            picked.dedup();
            picked
        }
        _ => (0..entries.len()).filter(|&at| !exempt(&entries[at])).collect(),
    };

    let rels: Vec<&str> = picked.iter().filter_map(|&at| entries[at].path(index).to_str().ok()).collect();

    if !tracking.watch_files(workdir, rels.iter().copied()) {
        return None;
    }

    if !same {
        tracking.keep_files(&rels.iter().copied().collect());
    }

    let looked: Vec<_> = picked.iter().map(|&at| &entries[at]).collect();
    let vouched = vouch(workdir, index, &looked);
    let mut suspects = Vec::new();
    let mut recheck = Vec::new();

    for ((&at, entry), vouched) in picked.iter().zip(looked).zip(vouched) {
        let path = entry.path(index);

        if !vouched {
            suspects.push(path);
        }

        if !vouched || !path.to_str().is_ok_and(|rel| tracking.covers(rel)) {
            recheck.push(at);
        }
    }

    *tracking.baseline = Some(Baseline { index: key, recheck });

    Some(suspects)
}

/// The positions of the entries a report may concern: the files reported,
/// the files directly in a directory reported, and every file below a
/// directory removed or renamed.
fn reported(index: &gix::index::State, touched: &Touched) -> Vec<usize> {
    let entries = index.entries();
    let mut picked = Vec::new();

    for path in &touched.paths {
        if let Ok(at) = index.entry_index_by_path(path.as_bytes().as_bstr()) {
            picked.push(at);
        }

        let prefix = if path.is_empty() { String::new() } else { format!("{path}/") };

        if let Some(range) = index.prefixed_entries_range(prefix.as_bytes().as_bstr()) {
            picked.extend(range.filter(|&at| !entries[at].path(index)[prefix.len()..].contains(&b'/')));
        }
    }

    for subtree in &touched.subtrees {
        let prefix = if subtree.is_empty() { String::new() } else { format!("{subtree}/") };

        picked.extend(index.prefixed_entries_range(prefix.as_bytes().as_bstr()).into_iter().flatten());
    }

    picked
}

/// What names the index's content: its checksum, and its timestamp, which
/// decides which entries are racy.
fn index_key(index: &gix::index::File) -> Option<Vec<u8>> {
    let checksum = index.checksum().filter(|checksum| !checksum.is_null())?;
    let timestamp = index.timestamp();
    let mut key = checksum.as_bytes().to_vec();

    key.extend(timestamp.unix_seconds().to_le_bytes());
    key.extend(timestamp.nanoseconds().to_le_bytes());

    Some(key)
}

/// Whether `entries` are vouched for, compared on a few threads when there
/// are many.
fn vouch(workdir: &Path, index: &gix::index::State, entries: &[&gix::index::Entry]) -> Vec<bool> {
    if entries.len() <= SUSPECTS {
        return entries.iter().map(|entry| vouched(workdir, index, entry)).collect();
    }

    let chunk = entries.len().div_ceil(STATTERS);

    thread::scope(|scope| {
        let parts: Vec<_> =
            entries.chunks(chunk).map(|part| scope.spawn(move || part.iter().map(|entry| vouched(workdir, index, entry)).collect::<Vec<_>>())).collect();

        parts.into_iter().flat_map(join).collect()
    })
}

/// Whether git never reports `entry` here: it is not a file, or git is told
/// to leave its worktree copy alone.
fn exempt(entry: &gix::index::Entry) -> bool {
    use gix::index::entry::Flags;

    !is_file(entry.mode) || entry.flags.intersects(Flags::SKIP_WORKTREE | Flags::ASSUME_VALID)
}

/// Whether `entry` needs no closer look: git does not report it here, or its
/// file's stat matches the index exactly and the entry is not racy (written
/// in the same second as the index, so a later change could share its
/// timestamp).
#[cfg(unix)]
fn vouched(workdir: &Path, index: &gix::index::State, entry: &gix::index::Entry) -> bool {
    use std::os::unix::fs::MetadataExt;

    use gix::index::entry::Flags;

    if exempt(entry) {
        return true;
    }

    if entry.stage_raw() != 0 || entry.flags.contains(Flags::INTENT_TO_ADD) {
        return false;
    }

    let Some(meta) = gix::path::try_from_bstr(entry.path(index)).ok().and_then(|rel| fs::symlink_metadata(workdir.join(rel)).ok()) else {
        return false;
    };

    let stat = &entry.stat;
    // git reads only the owner's execute bit.
    let executable = meta.mode() & 0o100 != 0;

    meta.is_file()
        && i64::from(stat.mtime.secs) < index.timestamp().unix_seconds()
        && (low(meta.mtime()), low(meta.mtime_nsec())) == (stat.mtime.secs, stat.mtime.nsecs)
        && (low(meta.ctime()), low(meta.ctime_nsec())) == (stat.ctime.secs, stat.ctime.nsecs)
        && (low(meta.size()), low(meta.ino()), meta.uid(), meta.gid()) == (stat.size, stat.ino, stat.uid, stat.gid)
        && executable == (entry.mode == Mode::FILE_EXECUTABLE)
}

#[cfg(not(unix))]
fn vouched(_workdir: &Path, _index: &gix::index::State, _entry: &gix::index::Entry) -> bool {
    false
}

/// The low 32 bits, which is what the index keeps of times, sizes, and inodes.
#[cfg(unix)]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn low(value: impl Into<i128>) -> u32 {
    value.into() as u32
}

/// gix's comparison of the index with the worktree, limited to `patterns`.
fn status(
    repo: &gix::Repository,
    index: &gix::worktree::Index,
    patterns: &[BString],
    filter: &Filter,
) -> Result<(Vec<SourceFile>, HashSet<String>), DiscoverError> {
    let iter = repo
        .status(gix::progress::Discard)
        .map_err(git)?
        .index(index.clone().into())
        .index_worktree_submodules(None)
        .index_worktree_rewrites(None)
        .untracked_files(UntrackedFiles::None)
        .into_index_worktree_iter(patterns.iter().cloned())
        .map_err(git)?;

    let mut files = Vec::new();
    let mut removed = HashSet::new();

    for item in iter {
        let Item::Modification { entry, rela_path, status, .. } = item.map_err(git)? else {
            continue;
        };

        match status {
            EntryStatus::NeedsUpdate(_) | EntryStatus::Change(Change::SubmoduleModification(_)) => {}
            EntryStatus::Change(Change::Removed) => {
                removed.insert(rela_path.to_string());
            }
            EntryStatus::Change(Change::Type { worktree_mode }) if !is_file(worktree_mode) => {
                removed.insert(rela_path.to_string());
            }
            _ if is_file(entry.mode) => push(&mut files, filter, rela_path.as_ref()),
            _ => {}
        }
    }

    Ok((files, removed))
}

#[cfg(all(test, any(target_os = "macos", target_os = "linux")))]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::process::Command;
    use std::time::{Duration, SystemTime};

    use super::Repository;
    use crate::{Memory, Scope, discover_with};

    fn git(root: &Path, args: &[&str]) {
        let status = Command::new("git")
            .args(["-c", "user.name=fmtkit", "-c", "user.email=fmtkit@example.com", "-c", "commit.gpgsign=false"])
            .args(args)
            .current_dir(root)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .status()
            .unwrap();

        assert!(status.success(), "git {args:?}");
    }

    #[test]
    fn a_quiet_watched_run_looks_at_nothing() {
        let repo = tempfile::tempdir().unwrap();
        let store = tempfile::tempdir().unwrap();
        let root = repo.path();

        git(root, &["init", "-q"]);

        for rel in ["a.ts", "src/b.ts", "src/deep/c.ts"] {
            fs::create_dir_all(root.join(rel).parent().unwrap()).unwrap();
            fs::write(root.join(rel), "export {};\n").unwrap();
            fs::File::options().write(true).open(root.join(rel)).unwrap().set_modified(SystemTime::now() - Duration::from_secs(3600)).unwrap();
        }

        git(root, &["add", "-A"]);
        git(root, &["commit", "-q", "-m", "init"]);

        let mut memory = Memory::open(store.path().join("dirs"));
        let mut repository = Repository::default();

        repository.watch();

        for _ in 0..2 {
            let later = SystemTime::now() + Duration::from_secs(60);

            memory.next_run(later);
            repository.next_run(later);

            let found = discover_with(root, &Scope::default(), &fmtkit_config::Files::default(), Some(&memory), &mut repository).unwrap();

            memory.save().unwrap();
            assert_eq!(found.files, Vec::new());
        }

        let (_, watch) = repository.watch.as_ref().unwrap();
        let mut clean: Vec<_> = watch.clean.keys().map(String::as_str).collect();

        clean.sort_unstable();

        assert_eq!(watch.baseline.as_ref().unwrap().recheck, Vec::<usize>::new());
        assert_eq!(clean, ["", "src", "src/deep"]);
        assert_eq!(repository.failure, None);
    }
}
