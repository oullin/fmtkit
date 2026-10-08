//! The walk under git's rules, on a small rayon pool.
//!
//! A path is ignored by the first decision, deepest first, among the
//! `.gitignore` files of its directory and their parents up to the root,
//! then `.git/info/exclude`, then the global excludes file (`core.excludesFile`,
//! else `$XDG_CONFIG_HOME/git/ignore`). An ignored directory is not entered, so
//! nothing below it can be re-included, as in git. Each directory is one
//! rayon task; idle threads sleep until work arrives rather than polling.

use std::fs;
use std::io;
use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};

use ignore::Match;
use ignore::gitignore::{Gitignore, GitignoreBuilder};

use crate::filter::Filter;
use crate::{DiscoverError, Walked};

/// One directory's `.gitignore`, chained to the nearest parent that has one.
struct Layer {
    matcher: Gitignore,
    parent: Option<Arc<Layer>>,
}

struct Walk<'a, K> {
    root: &'a Path,
    filter: &'a Filter,
    keep: &'a K,
    ignore_case: bool,
    exclude: Gitignore,
    global: Gitignore,
    found: Mutex<Walked>,
    failure: Mutex<Option<DiscoverError>>,
}

pub(super) fn walk<K>(root: &Path, filter: &Filter, ignore_case: bool, exclude: &Path, threads: usize, keep: &K) -> Result<Walked, DiscoverError>
where
    K: Fn(&str) -> bool + Sync,
{
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .map_err(|err| DiscoverError::Walk { path: root.to_path_buf(), message: err.to_string() })?;

    let walk = Walk {
        root,
        filter,
        keep,
        ignore_case,
        exclude: matcher(root, ignore_case, |builder| builder.add(exclude)),
        global: global(root, ignore_case),
        found: Mutex::new(Walked::default()),
        failure: Mutex::new(None),
    };

    pool.scope(|scope| walk.visit(scope, root, "", None));

    if let Some(err) = walk.failure.into_inner().unwrap_or_else(PoisonError::into_inner) {
        return Err(err);
    }

    Ok(walk.found.into_inner().unwrap_or_else(PoisonError::into_inner))
}

impl<K> Walk<'_, K>
where
    K: Fn(&str) -> bool + Sync,
{
    /// Record the files in `dir` (known as `rel`) and queue its subdirectories.
    fn visit<'s>(&'s self, scope: &rayon::Scope<'s>, dir: &Path, rel: &str, parent: Option<Arc<Layer>>) {
        let entries = match fs::read_dir(dir).and_then(Iterator::collect::<io::Result<Vec<_>>>) {
            Ok(entries) => entries,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return,
            Err(err) => return self.fail(dir, &err),
        };

        // A nested repository or submodule belongs to its own repository.
        if !rel.is_empty() && entries.iter().any(|entry| entry.file_name() == ".git") {
            return;
        }

        let layers = if entries.iter().any(|entry| entry.file_name() == ".gitignore") {
            let matcher = matcher(dir, self.ignore_case, |builder| builder.add(dir.join(".gitignore")));

            Some(Arc::new(Layer { matcher, parent }))
        } else {
            parent
        };

        let mut walked = Walked::default();

        for entry in entries {
            let (Ok(kind), Some(name)) = (entry.file_type(), entry.file_name().to_str().map(str::to_owned)) else {
                continue;
            };

            let path = entry.path();
            let child = if rel.is_empty() { name } else { format!("{rel}/{name}") };

            if self.ignored(layers.as_deref(), &path, kind.is_dir()) {
                continue;
            }

            if kind.is_dir() {
                if self.filter.enters(&child) {
                    let layers = layers.clone();

                    scope.spawn(move |scope| self.visit(scope, &path, &child, layers));
                }
            } else if kind.is_file() && (self.keep)(&child) {
                walked.see(self.filter, &child);
            }
        }

        if !walked.files.is_empty() || !walked.modules.is_empty() {
            let mut found = self.found.lock().unwrap_or_else(PoisonError::into_inner);

            found.files.append(&mut walked.files);
            found.modules.append(&mut walked.modules);
        }
    }

    fn ignored(&self, mut layer: Option<&Layer>, path: &Path, is_dir: bool) -> bool {
        while let Some(current) = layer {
            match current.matcher.matched(path, is_dir) {
                Match::None => layer = current.parent.as_deref(),
                decided => return decided.is_ignore(),
            }
        }

        match self.exclude.matched(path, is_dir) {
            Match::None => self.global.matched(path, is_dir).is_ignore(),
            decided => decided.is_ignore(),
        }
    }

    fn fail(&self, dir: &Path, err: &io::Error) {
        let path = dir.strip_prefix(self.root).unwrap_or(dir);

        self.failure
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get_or_insert_with(|| DiscoverError::Walk { path: self.root.to_path_buf(), message: format!("{}: {err}", path.display()) });
    }
}

/// The global excludes file, rooted at the repository root as git roots it.
fn global(root: &Path, ignore_case: bool) -> Gitignore {
    let mut builder = GitignoreBuilder::new(root);
    let _ = builder.case_insensitive(ignore_case);

    builder.build_global().0
}

/// A matcher rooted at `dir`. An unreadable or malformed ignore file
/// contributes what could be read, as in git and the `ignore` crate.
fn matcher(dir: &Path, ignore_case: bool, add: impl FnOnce(&mut GitignoreBuilder) -> Option<ignore::Error>) -> Gitignore {
    let mut builder = GitignoreBuilder::new(dir);
    let _ = builder.case_insensitive(ignore_case);
    let _ = add(&mut builder);

    builder.build().unwrap_or_else(|_| Gitignore::empty())
}
