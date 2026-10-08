//! The walk under git's rules, on a small rayon pool.
//!
//! A path is ignored by the first decision, deepest first, among the
//! `.gitignore` files of its directory and their parents up to the root,
//! then `.git/info/exclude`, then the global excludes file (`core.excludesFile`,
//! else `$XDG_CONFIG_HOME/git/ignore`). An ignored directory is not entered, so
//! nothing below it can be re-included, as in git. Each directory is one
//! rayon task; idle threads sleep until work arrives rather than polling.
//!
//! With a [`Memory`], a directory whose stamp is unchanged is not listed: its
//! stored listing is replayed, and ignore files are read only for the
//! directories that are listed.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use ignore::Match;
use ignore::gitignore::{Gitignore, GitignoreBuilder};

use crate::filter::Filter;
use crate::memory::{Context, Listing, Memory, Stamp};
use crate::{DiscoverError, Walked};

/// One directory's `.gitignore`, chained to the nearest parent that has one.
/// The file is read the first time a path below it is matched.
struct Layer {
    dir: PathBuf,
    matcher: OnceLock<Gitignore>,
    parent: Option<Arc<Layer>>,
}

struct Walk<'a, K> {
    root: &'a Path,
    filter: &'a Filter,
    keep: &'a K,
    ignore_case: bool,
    exclude: &'a Path,
    global: Option<PathBuf>,
    exclude_matcher: OnceLock<Gitignore>,
    global_matcher: OnceLock<Gitignore>,
    memory: Option<&'a Memory>,
    found: Mutex<Walked>,
    failure: Mutex<Option<DiscoverError>>,
}

pub(super) fn walk<K>(
    root: &Path,
    filter: &Filter,
    ignore_case: bool,
    exclude: &Path,
    memory: Option<&Memory>,
    threads: usize,
    keep: &K,
) -> Result<Walked, DiscoverError>
where
    K: Fn(&str) -> bool + Sync,
{
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .map_err(|err| DiscoverError::Walk { path: root.to_path_buf(), message: err.to_string() })?;

    let global = ignore::gitignore::gitconfig_excludes_path();
    let trusted = memory.is_some_and(|memory| memory.begin(Context::new(ignore_case, [exclude.to_path_buf()].into_iter().chain(global.clone()))));

    let walk = Walk {
        root,
        filter,
        keep,
        ignore_case,
        exclude,
        global,
        exclude_matcher: OnceLock::new(),
        global_matcher: OnceLock::new(),
        memory,
        found: Mutex::new(Walked::default()),
        failure: Mutex::new(None),
    };

    pool.scope(|scope| walk.visit(scope, root, "", None, trusted));

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
    /// A `trusted` directory's stored listing may be replayed: every ignore
    /// file above it is unchanged since the listing was stored.
    fn visit<'s>(&'s self, scope: &rayon::Scope<'s>, dir: &Path, rel: &str, parent: Option<Arc<Layer>>, trusted: bool) {
        let stamp = match fs::metadata(dir) {
            Ok(meta) => Stamp::from_metadata(&meta),
            Err(err) if err.kind() == io::ErrorKind::NotFound => return,
            Err(err) => return self.fail(dir, &err),
        };

        let stored = self.memory.filter(|_| trusted).zip(stamp).and_then(|(memory, stamp)| memory.listing(rel, &stamp));
        let replayed = stored.filter(|listing| listing.rules.is_none_or(|rules| Stamp::of(&dir.join(".gitignore")) == Some(rules)));

        if let Some(listing) = replayed {
            let layers = if listing.rules.is_some() { Some(layer(dir, parent)) } else { parent };

            return self.replay(scope, dir, rel, listing, layers.as_ref(), trusted);
        }

        let Some((listing, layers)) = self.list(dir, rel, parent, stamp) else {
            return;
        };

        // Listings below keep only while this directory's rules are the ones
        // they were filtered by.
        let trusted = trusted && self.memory.and_then(|memory| memory.previous(rel)).is_some_and(|previous| previous.rules == listing.rules);

        self.replay(scope, dir, rel, &listing, layers.as_ref(), trusted);

        if let (Some(memory), Some(_)) = (self.memory, stamp) {
            memory.remember(rel, listing);
        }
    }

    /// Record a listing's files and queue its subdirectories, which `layers`
    /// (this directory's rules and its parents') apply to.
    fn replay<'s>(&'s self, scope: &rayon::Scope<'s>, dir: &Path, rel: &str, listing: &Listing, layers: Option<&Arc<Layer>>, trusted: bool) {
        // A nested repository or submodule belongs to its own repository.
        if listing.nested && !rel.is_empty() {
            return;
        }

        for name in &listing.dirs {
            let child = join(rel, name);

            if self.filter.enters(&child) {
                let layers = layers.cloned();
                let path = dir.join(name);

                scope.spawn(move |scope| self.visit(scope, &path, &child, layers, trusted));
            }
        }

        let mut walked = Walked::default();

        for name in &listing.files {
            let child = join(rel, name);

            if (self.keep)(&child) {
                walked.see(self.filter, &child);
            }
        }

        if !walked.files.is_empty() || !walked.modules.is_empty() {
            let mut found = self.found.lock().unwrap_or_else(PoisonError::into_inner);

            found.files.append(&mut walked.files);
            found.modules.append(&mut walked.modules);
        }
    }

    /// List `dir` and keep what git's rules do not ignore, returning the
    /// listing and the rules that apply below it. `stamp` was taken before
    /// the listing, so a change made during it changes the stamp.
    fn list(&self, dir: &Path, rel: &str, parent: Option<Arc<Layer>>, stamp: Option<Stamp>) -> Option<(Listing, Option<Arc<Layer>>)> {
        let entries = match fs::read_dir(dir).and_then(Iterator::collect::<io::Result<Vec<_>>>) {
            Ok(entries) => entries,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return None,
            Err(err) => {
                self.fail(dir, &err);

                return None;
            }
        };

        let mut listing = Listing {
            stamp: stamp.unwrap_or_default(),
            rules: None,
            nested: entries.iter().any(|entry| entry.file_name() == ".git"),
            files: Vec::new(),
            dirs: Vec::new(),
        };

        if listing.nested && !rel.is_empty() {
            return Some((listing, parent));
        }

        let layers = if entries.iter().any(|entry| entry.file_name() == ".gitignore") {
            // Stamped before it is read, as the directory is.
            listing.rules = Some(Stamp::of(&dir.join(".gitignore")).unwrap_or_default());

            Some(layer(dir, parent))
        } else {
            parent
        };

        for entry in entries {
            let (Ok(kind), Some(name)) = (entry.file_type(), entry.file_name().to_str().map(str::to_owned)) else {
                continue;
            };

            if !(kind.is_dir() || kind.is_file()) || self.ignored(layers.as_deref(), &entry.path(), kind.is_dir()) {
                continue;
            }

            if kind.is_dir() { listing.dirs.push(name) } else { listing.files.push(name) }
        }

        Some((listing, layers))
    }

    /// The decision of `layer` alone on `path`, if it has one.
    fn decided(&self, layer: &Layer, path: &Path, is_dir: bool) -> Option<bool> {
        match self.layer_matcher(layer).matched(path, is_dir) {
            Match::None => None,
            decided => Some(decided.is_ignore()),
        }
    }

    fn ignored(&self, mut layer: Option<&Layer>, path: &Path, is_dir: bool) -> bool {
        while let Some(current) = layer {
            match self.decided(current, path, is_dir) {
                None => layer = current.parent.as_deref(),
                Some(ignore) => return ignore,
            }
        }

        let exclude = self.exclude_matcher.get_or_init(|| matcher(self.root, self.ignore_case, |builder| builder.add(self.exclude)));

        match exclude.matched(path, is_dir) {
            Match::None => self.global_matcher.get_or_init(|| self.global()).matched(path, is_dir).is_ignore(),
            decided => decided.is_ignore(),
        }
    }

    fn layer_matcher<'l>(&self, layer: &'l Layer) -> &'l Gitignore {
        layer.matcher.get_or_init(|| matcher(&layer.dir, self.ignore_case, |builder| builder.add(layer.dir.join(".gitignore"))))
    }

    /// The global excludes file, rooted at the repository root as git roots it.
    fn global(&self) -> Gitignore {
        match &self.global {
            Some(path) if path.is_file() => matcher(self.root, self.ignore_case, |builder| builder.add(path)),
            _ => Gitignore::empty(),
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

fn layer(dir: &Path, parent: Option<Arc<Layer>>) -> Arc<Layer> {
    Arc::new(Layer { dir: dir.to_path_buf(), matcher: OnceLock::new(), parent })
}

fn join(rel: &str, name: &str) -> String {
    if rel.is_empty() { name.to_owned() } else { format!("{rel}/{name}") }
}

/// A matcher rooted at `dir`. An unreadable or malformed ignore file
/// contributes what could be read, as in git and the `ignore` crate.
fn matcher(dir: &Path, ignore_case: bool, add: impl FnOnce(&mut GitignoreBuilder) -> Option<ignore::Error>) -> Gitignore {
    let mut builder = GitignoreBuilder::new(dir);
    let _ = builder.case_insensitive(ignore_case);
    let _ = add(&mut builder);

    builder.build().unwrap_or_else(|_| Gitignore::empty())
}
