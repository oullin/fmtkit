use std::collections::HashSet;
use std::ops::ControlFlow;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::thread;

use gix::bstr::{BStr, ByteSlice};
use gix::diff::index::ChangeRef;
use gix::dir::EntryRef;
use gix::dir::entry::{Kind, Status};
use gix::dir::walk::{Action, Delegate, EmissionMode, ForDeletionMode};
use gix::index::entry::Mode;
use gix::status::UntrackedFiles;
use gix::status::index_worktree::Item;
use gix::status::plumbing::index_as_worktree::{Change, EntryStatus};
use gix::status::tree_index::TrackRenames;

use crate::filter::Filter;
use crate::{DiscoverError, SourceFile};

/// Tracked files that differ from `HEAD` in the index or the worktree, plus
/// untracked files that are not ignored. Files deleted from the worktree are
/// left out; a rename contributes its new path.
///
/// The three comparisons (`HEAD` against the index, the index against the
/// worktree, and the untracked walk) run on their own threads.
pub(crate) fn changed(root: &Path, filter: &Filter) -> Result<Vec<SourceFile>, DiscoverError> {
    let repo = open(root)?;
    let index = repo.index_or_empty().map_err(git)?;
    let patterns = filter.pathspecs();
    let shared = repo.clone().into_sync();

    let (staged, worktree, untracked) = thread::scope(|scope| {
        let staged = scope.spawn(|| staged(&shared.to_thread_local(), &index, filter));
        let untracked = scope.spawn(|| walk(&shared.to_thread_local(), &index, &patterns, filter, false));
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
/// is not ignored, from one directory walk.
pub(crate) fn all(root: &Path, filter: &Filter) -> Result<Vec<SourceFile>, DiscoverError> {
    let repo = open(root)?;
    let index = repo.index_or_empty().map_err(git)?;

    walk(&repo, &index, &filter.pathspecs(), filter, true)
}

fn open(root: &Path) -> Result<gix::Repository, DiscoverError> {
    gix::open(root).map_err(git)
}

#[allow(clippy::needless_pass_by_value)]
fn git(err: impl ToString) -> DiscoverError {
    DiscoverError::Git(err.to_string())
}

fn join<T>(handle: thread::ScopedJoinHandle<'_, T>) -> T {
    handle.join().unwrap_or_else(|payload| std::panic::resume_unwind(payload))
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

/// Entries added or modified in the index against `HEAD`.
fn staged(repo: &gix::Repository, index: &gix::index::State, filter: &Filter) -> Result<Vec<SourceFile>, DiscoverError> {
    let tree = repo.head_tree_id_or_empty().map_err(git)?.detach();
    let mut files = Vec::new();

    repo.tree_index_status(&tree, index, None, TrackRenames::Disabled, |change, _, _| {
        match change {
            ChangeRef::Addition { location, entry_mode, .. } | ChangeRef::Modification { location, entry_mode, .. } if is_file(entry_mode) => {
                push(&mut files, filter, location.as_ref());
            }
            ChangeRef::Rewrite { location, entry_mode, .. } if is_file(entry_mode) => push(&mut files, filter, location.as_ref()),
            _ => {}
        }

        Ok(ControlFlow::Continue(()))
    })
    .map_err(git)?;

    Ok(files)
}

/// Tracked files whose worktree copy differs from the index, and the paths
/// that no longer exist as files in the worktree.
fn worktree(
    repo: &gix::Repository,
    index: &gix::worktree::Index,
    patterns: &[gix::bstr::BString],
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

/// Walk the worktree for untracked, non-ignored files, and tracked files too
/// when `tracked` is set. Directories the filter rejects are not entered.
fn walk(
    repo: &gix::Repository,
    index: &gix::index::State,
    patterns: &[gix::bstr::BString],
    filter: &Filter,
    tracked: bool,
) -> Result<Vec<SourceFile>, DiscoverError> {
    let options = repo
        .dirwalk_options()
        .map_err(git)?
        .emit_tracked(tracked)
        .emit_untracked(EmissionMode::Matching)
        .emit_ignored(None)
        .emit_pruned(false)
        .emit_empty_directories(false)
        .recurse_repositories(false)
        .empty_patterns_match_prefix(false);

    let mut collect = Collect { filter, tracked, files: Vec::new() };

    repo.dirwalk(index, patterns, &AtomicBool::new(false), options, &mut collect).map_err(git)?;

    Ok(collect.files)
}

struct Collect<'a> {
    filter: &'a Filter,
    tracked: bool,
    files: Vec<SourceFile>,
}

impl Delegate for Collect<'_> {
    fn emit(&mut self, entry: EntryRef<'_>, _collapsed_directory_status: Option<Status>) -> Action {
        let wanted = match entry.status {
            Status::Untracked => true,
            Status::Tracked => self.tracked,
            Status::Pruned | Status::Ignored(_) => false,
        };

        if wanted && entry.disk_kind == Some(Kind::File) {
            push(&mut self.files, self.filter, entry.rela_path.as_ref());
        }

        ControlFlow::Continue(())
    }

    fn can_recurse(&mut self, entry: EntryRef<'_>, for_deletion: Option<ForDeletionMode>, worktree_root_is_repository: bool) -> bool {
        entry.status.can_recurse(entry.disk_kind, entry.pathspec_match, for_deletion, worktree_root_is_repository)
            && entry.rela_path.to_str().is_ok_and(|rel| self.filter.enters(rel))
    }
}
