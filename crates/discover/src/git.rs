use std::collections::{HashMap, HashSet};
use std::fs;
use std::ops::ControlFlow;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use gix::bstr::{BStr, BString, ByteSlice};
use gix::diff::index::ChangeRef;
use gix::index::entry::Mode;
use gix::status::UntrackedFiles;
use gix::status::index_worktree::Item;
use gix::status::plumbing::index_as_worktree::{Change, EntryStatus};
use gix::status::tree_index::TrackRenames;

use crate::filter::Filter;
use crate::memory::{Memory, Staged};
use crate::walk::{self, Rules};
use crate::{DiscoverError, SourceFile, Walked};

/// Tracked files that differ from `HEAD` in the index or the worktree, plus
/// untracked files that are not ignored. Files deleted from the worktree are
/// left out; a rename contributes its new path.
///
/// The three comparisons (`HEAD` against the index, the index against the
/// worktree, and the untracked walk) run on their own threads.
pub(crate) fn changed(root: &Path, filter: &Arc<Filter>, memory: Option<&Memory>) -> Result<Vec<SourceFile>, DiscoverError> {
    let repo = open(root)?;
    let index = repo.index_or_empty().map_err(git)?;
    let patterns = filter.pathspecs();
    let shared = repo.clone().into_sync();
    let tracked = Tracked::new(&index, ignore_case(&repo));
    let rules = rules(&repo, &tracked);

    let (staged, worktree, untracked) = thread::scope(|scope| {
        let staged = scope.spawn(|| staged(&shared.to_thread_local(), &index, filter, memory));
        let untracked = scope.spawn(|| walk::walk(root, filter, &rules, memory, |rel| !tracked.visit(rel)).map(|walked| walked.files));
        let worktree = worktree(&repo, &index, &patterns, filter);

        (join(staged), worktree, join(untracked))
    });

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
pub(crate) fn all(root: &Path, filter: &Arc<Filter>, memory: Option<&Memory>) -> Result<Walked, DiscoverError> {
    let repo = open(root)?;
    let index = repo.index_or_empty().map_err(git)?;
    let tracked = Tracked::new(&index, ignore_case(&repo));

    if let Some(prefixes) = filter.prefixes()
        && prefixes.iter().all(|rel| tracked.contains(rel))
    {
        let files = prefixes.iter().filter_map(|rel| filter.source_file(rel)).filter(|file| is_regular(&file.abs)).collect();

        return Ok(Walked { files, modules: Vec::new() });
    }

    let mut walked = walk::walk(root, filter, &rules(&repo, &tracked), memory, |rel| {
        tracked.visit(rel);

        true
    })?;

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
fn rules(repo: &gix::Repository, tracked: &Tracked<'_>) -> Rules {
    Rules::Git { ignore_case: tracked.ignore_case, exclude: repo.common_dir().join("info").join("exclude") }
}

/// Whether git compares paths case-insensitively here (`core.ignoreCase`).
fn ignore_case(repo: &gix::Repository) -> bool {
    repo.filesystem_options().is_ok_and(|caps| caps.ignore_case)
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
) -> Result<(Vec<SourceFile>, HashSet<String>), DiscoverError> {
    if patterns.is_empty()
        && let Some(suspects) = repo.workdir().and_then(|workdir| suspects(workdir, index))
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
fn suspects<'i>(workdir: &Path, index: &'i gix::index::State) -> Option<Vec<&'i BStr>> {
    let entries = index.entries();
    let chunk = entries.len().div_ceil(STATTERS).max(1);

    let suspects = thread::scope(|scope| {
        let parts: Vec<_> = entries
            .chunks(chunk)
            .map(|part| scope.spawn(move || part.iter().filter(|entry| !vouched(workdir, index, entry)).map(|entry| entry.path(index)).collect::<Vec<_>>()))
            .collect();

        parts.into_iter().flat_map(join).collect()
    });

    cfg!(unix).then_some(suspects)
}

/// Whether `entry` needs no closer look: git does not report it here, or its
/// file's stat matches the index exactly and the entry is not racy (written
/// in the same second as the index, so a later change could share its
/// timestamp).
#[cfg(unix)]
fn vouched(workdir: &Path, index: &gix::index::State, entry: &gix::index::Entry) -> bool {
    use std::os::unix::fs::MetadataExt;

    use gix::index::entry::Flags;

    if !is_file(entry.mode) || entry.flags.intersects(Flags::SKIP_WORKTREE | Flags::ASSUME_VALID) {
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
