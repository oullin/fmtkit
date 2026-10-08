//! Which files a run covers.
//!
//! Inside a git repository the default scope is the changed set: tracked files
//! modified in the worktree or the index against `HEAD`, plus untracked files
//! that are not ignored. `--all`, or naming paths, widens it to every tracked
//! and untracked, non-ignored file (under those paths). Outside git every file
//! under the root that `.gitignore` files and `[files] exclude` do not exclude
//! is in scope, changed or not.
//!
//! Whatever the source, `.git`, `node_modules`, and `vendor` directories are
//! never entered, symbolic links are never followed or listed, and `.d.ts`
//! family declaration files are left out.

mod filter;
mod generated;
mod git;
mod walk;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{env, io};

use fmtkit_core::{Lane, Lang};

pub use generated::is_generated;

use filter::Filter;

/// What the user asked to cover.
#[derive(Debug, Clone, Default)]
pub struct Scope {
    /// Cover every file, not only the changed set.
    pub all: bool,
    /// Lanes to keep; empty keeps both.
    pub lanes: Vec<Lane>,
    /// Cover these paths (files or directories), relative to the current
    /// directory or absolute, changed or not. Empty covers the changed set, or
    /// the whole repository under `all`.
    pub paths: Vec<PathBuf>,
}

/// One file in scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    /// Repository-relative, forward slashes.
    pub rel: String,
    pub abs: PathBuf,
    pub lang: Lang,
}

#[derive(Debug, Clone, Default)]
pub struct Discovery {
    /// Sorted by `rel`, deduplicated.
    pub files: Vec<SourceFile>,
    /// Scope paths that do not exist, as given.
    pub missing: Vec<String>,
    /// Whether the root is a git work tree.
    pub git: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum DiscoverError {
    #[error("git: {0}")]
    Git(String),
    #[error("walk {path}: {message}")]
    Walk { path: PathBuf, message: String },
    #[error("[files] exclude: {0}")]
    Pattern(String),
    /// A scope path that exists but lies outside the root.
    #[error("{} is outside the repository root {}", path.display(), root.display())]
    Outside { path: PathBuf, root: PathBuf },
}

/// The repository root for `cwd`: the enclosing git work tree, else `cwd`.
///
/// The work tree is found by walking up from `cwd` (a linked worktree's root
/// is its own directory). A bare repository, or no repository at all, yields
/// `cwd`. A relative `cwd` yields a relative root.
pub fn find_root(cwd: &Path) -> PathBuf {
    let Ok((path, _trust)) = gix::discover::upwards(cwd) else {
        return cwd.to_path_buf();
    };

    match path.into_repository_and_work_tree_directories().1 {
        Some(work_tree) if work_tree.is_absolute() || cwd.is_relative() => work_tree,
        Some(work_tree) => cwd.join(work_tree),
        None => cwd.to_path_buf(),
    }
}

/// Resolve `scope` under `root` into the files to process.
///
/// `root` is a git work tree when it holds a `.git` entry; anything else is
/// walked as plain files.
pub fn discover(root: &Path, scope: &Scope, files: &fmtkit_config::Files) -> Result<Discovery, DiscoverError> {
    let git = root.join(".git").exists();
    let (prefixes, missing) = resolve_paths(root, &scope.paths)?;

    if prefixes.as_ref().is_some_and(Vec::is_empty) {
        return Ok(Discovery { files: Vec::new(), missing, git });
    }

    let filter = Arc::new(Filter::new(root, &files.exclude, &scope.lanes, prefixes)?);

    // A named path is covered whether or not it changed, as `check` did in 0.x.
    let every = scope.all || !scope.paths.is_empty();

    let mut files = match (git, every) {
        (true, false) => git::changed(root, &filter)?,
        (true, true) => git::all(root, &filter)?,
        (false, _) => walk::all(root, &filter)?,
    };

    files.sort_unstable_by(|a, b| a.rel.cmp(&b.rel));
    files.dedup_by(|a, b| a.rel == b.rel);

    Ok(Discovery { files, missing, git })
}

/// Turn scope paths into root-relative prefixes. `None` covers the whole root;
/// `Some(empty)` means every given path was missing.
fn resolve_paths(root: &Path, paths: &[PathBuf]) -> Result<(Option<Vec<String>>, Vec<String>), DiscoverError> {
    if paths.is_empty() {
        return Ok((None, Vec::new()));
    }

    let cwd = env::current_dir().map_err(|err| DiscoverError::Walk { path: PathBuf::from("."), message: err.to_string() })?;
    let resolved_root = root.canonicalize().unwrap_or_else(|_| cwd.join(root));
    let mut prefixes = Vec::with_capacity(paths.len());
    let mut missing = Vec::new();
    let mut whole = false;

    for path in paths {
        let resolved = match cwd.join(path).canonicalize() {
            Ok(resolved) => resolved,
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                missing.push(path.display().to_string());

                continue;
            }
            Err(err) => return Err(DiscoverError::Walk { path: path.clone(), message: err.to_string() }),
        };

        let Ok(rel) = resolved.strip_prefix(&resolved_root) else {
            return Err(DiscoverError::Outside { path: path.clone(), root: root.to_path_buf() });
        };

        let rel = to_slash(rel);

        whole |= rel.is_empty();
        prefixes.push(rel);
    }

    if whole {
        return Ok((None, missing));
    }

    prefixes.sort_unstable();
    prefixes.dedup();

    Ok((Some(prefixes), missing))
}

fn to_slash(path: &Path) -> String {
    let mut out = String::new();

    for component in path.components() {
        if !out.is_empty() {
            out.push('/');
        }

        out.push_str(&component.as_os_str().to_string_lossy());
    }

    out
}
