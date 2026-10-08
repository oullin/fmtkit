//! The outcome cache.
//!
//! A file whose bytes, path, fmtkit version, outcome-relevant configuration,
//! and mode match a stored entry reuses the stored [`FileOutcome`] without
//! being parsed. Only outcomes with nothing to write are stored, so a hit
//! never needs the formatter. Entries live in the user cache directory, one
//! store per repository root. Every failure degrades to a miss.
//!
//! A store is one `postcard` file at `<cache dir>/fmtkit/v2/<blake3(root)>.bin`,
//! where the cache directory is the platform's (`~/Library/Caches` on macOS,
//! `$XDG_CACHE_HOME` or `~/.cache` on Linux). `FMTKIT_CACHE_DIR` replaces the
//! whole `<cache dir>/fmtkit/v2` prefix. The file is read once, on first use,
//! and rewritten atomically by [`Cache::flush`] when the run stored something
//! new.

mod wire;

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{OnceLock, PoisonError, RwLock};

use fmtkit_core::{FileOutcome, Mode};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

/// Names a directory that holds the stores instead of `<cache dir>/fmtkit/v2`.
pub const CACHE_DIR_ENV: &str = "FMTKIT_CACHE_DIR";

/// Past this many entries, a flush keeps only the entries this run used.
pub const BUDGET: usize = 200_000;

/// Leads every store file; bump it when the layout changes.
const MAGIC: &[u8; 8] = b"fmtkit\x02\x00";

const SHARDS: usize = 16;

/// The digest a lookup is keyed by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Key(pub [u8; 32]);

pub struct Cache {
    store: Option<Store>,
}

struct Store {
    path: PathBuf,
    config_hash: [u8; 32],
    entries: OnceLock<Shards>,
    /// Whether a put added or replaced an entry since the store was read.
    dirty: AtomicBool,
}

type Shard = RwLock<FxHashMap<[u8; 32], Slot>>;

struct Shards([Shard; SHARDS]);

struct Slot {
    outcome: FileOutcome,
    /// Hit or put during this run, so kept when the store is pruned.
    used: AtomicBool,
}

#[derive(Serialize, Deserialize)]
struct Stored {
    version: String,
    entries: Vec<([u8; 32], wire::Outcome)>,
}

impl Cache {
    /// Open the store for `root`. A disabled cache misses every lookup and
    /// stores nothing. With no usable cache directory the cache is disabled.
    pub fn open(root: &Path, config_hash: [u8; 32], enabled: bool) -> Self {
        let dir = std::env::var_os(CACHE_DIR_ENV)
            .filter(|dir| !dir.is_empty())
            .map(PathBuf::from)
            .or_else(|| dirs::cache_dir().map(|dir| dir.join("fmtkit").join("v2")));

        match dir {
            Some(dir) if enabled => Self::open_in(&dir, root, config_hash),
            _ => Self::disabled(),
        }
    }

    /// Open the store for `root` inside `dir`, ignoring the environment.
    pub fn open_in(dir: &Path, root: &Path, config_hash: [u8; 32]) -> Self {
        let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
        let name = blake3::hash(root.as_os_str().as_encoded_bytes()).to_hex();
        let path = dir.join(format!("{name}.bin"));

        Self { store: Some(Store { path, config_hash, entries: OnceLock::new(), dirty: AtomicBool::new(false) }) }
    }

    /// A store that is never read or written.
    pub fn disabled() -> Self {
        Self { store: None }
    }

    pub fn is_enabled(&self) -> bool {
        self.store.is_some()
    }

    /// The store file, if the cache is enabled.
    pub fn path(&self) -> Option<&Path> {
        self.store.as_ref().map(|store| store.path.as_path())
    }

    /// The lookup key of `content` at `rel` in `mode`. A disabled cache skips
    /// the hashing and returns an all-zero key.
    pub fn key(&self, mode: Mode, rel: &str, content: &[u8]) -> Key {
        let Some(store) = &self.store else {
            return Key([0; 32]);
        };

        let mode: u8 = match mode {
            Mode::Format => 0,
            Mode::Check => 1,
        };

        let mut hasher = blake3::Hasher::new();

        hasher.update(&(fmtkit_core::VERSION.len() as u64).to_le_bytes());
        hasher.update(fmtkit_core::VERSION.as_bytes());
        hasher.update(&store.config_hash);
        hasher.update(&[mode]);
        hasher.update(&(rel.len() as u64).to_le_bytes());
        hasher.update(rel.as_bytes());
        hasher.update(content);

        Key(*hasher.finalize().as_bytes())
    }

    /// Thread-safe.
    pub fn get(&self, key: &Key) -> Option<FileOutcome> {
        let store = self.store.as_ref()?;
        let shard = store.entries().shard(key).read().unwrap_or_else(PoisonError::into_inner);
        let slot = shard.get(&key.0)?;

        slot.used.store(true, Ordering::Relaxed);

        Some(slot.outcome.clone())
    }

    /// Thread-safe. Outcomes that changed the file or failed are ignored.
    pub fn put(&self, key: Key, outcome: &FileOutcome) {
        let Some(store) = &self.store else {
            return;
        };

        if outcome.changed || outcome.error.is_some() {
            return;
        }

        let mut shard = store.entries().shard(&key).write().unwrap_or_else(PoisonError::into_inner);

        if let Some(slot) = shard.get(&key.0)
            && slot.outcome == *outcome
        {
            slot.used.store(true, Ordering::Relaxed);

            return;
        }

        shard.insert(key.0, Slot { outcome: outcome.clone(), used: AtomicBool::new(true) });
        store.dirty.store(true, Ordering::Relaxed);
    }

    /// Persist new entries, dropping entries not used by this run when the store
    /// grows past its budget.
    pub fn flush(self) -> io::Result<()> {
        self.flush_with_budget(BUDGET)
    }

    fn flush_with_budget(self, budget: usize) -> io::Result<()> {
        let Some(store) = self.store else {
            return Ok(());
        };

        let (Some(shards), true) = (store.entries.into_inner(), store.dirty.into_inner()) else {
            return Ok(());
        };

        let mut entries: Vec<([u8; 32], Slot)> = shards.0.into_iter().flat_map(|shard| shard.into_inner().unwrap_or_else(PoisonError::into_inner)).collect();

        if entries.len() > budget {
            entries.retain(|(_, slot)| slot.used.load(Ordering::Relaxed));
        }

        entries.sort_unstable_by_key(|entry| entry.0);

        let stored = Stored {
            version: fmtkit_core::VERSION.to_owned(),
            entries: entries.iter().map(|(key, slot)| (*key, wire::Outcome::from(&slot.outcome))).collect(),
        };
        let mut bytes = MAGIC.to_vec();

        bytes.extend(postcard::to_stdvec(&stored).map_err(io::Error::other)?);

        write_atomic(&store.path, &bytes)
    }
}

impl Store {
    fn entries(&self) -> &Shards {
        self.entries.get_or_init(|| Shards::load(&self.path))
    }
}

impl Shards {
    fn empty() -> Self {
        Self(std::array::from_fn(|_| RwLock::default()))
    }

    /// Read a store file. A missing, foreign, corrupt, or other-version file
    /// reads as empty.
    fn load(path: &Path) -> Self {
        let mut shards = Self::empty();

        let Ok(bytes) = fs::read(path) else {
            return shards;
        };

        let Some(body) = bytes.strip_prefix(MAGIC) else {
            return shards;
        };

        let Ok(stored) = postcard::from_bytes::<Stored>(body) else {
            return shards;
        };

        if stored.version != fmtkit_core::VERSION {
            return shards;
        }

        for (key, outcome) in stored.entries {
            let shard = shards.0[index(&Key(key))].get_mut().unwrap_or_else(PoisonError::into_inner);

            shard.insert(key, Slot { outcome: outcome.into(), used: AtomicBool::new(false) });
        }

        shards
    }

    fn shard(&self, key: &Key) -> &Shard {
        &self.0[index(key)]
    }
}

fn index(key: &Key) -> usize {
    usize::from(key.0[0]) % SHARDS
}

/// Write `bytes` to a sibling temporary file, then rename it over `path`.
fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let dir = path.parent().unwrap_or(Path::new("."));

    fs::create_dir_all(dir)?;

    let tmp = path.with_extension(format!("{}.tmp", std::process::id()));
    let written = fs::File::create(&tmp).and_then(|mut file| file.write_all(bytes)).and_then(|()| fs::rename(&tmp, path));

    if written.is_err() {
        let _ = fs::remove_file(&tmp);
    }

    written
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prunes_unused_entries_past_the_budget() {
        let dir = tempfile::tempdir().unwrap();
        let root = Path::new("/repo");
        let cache = Cache::open_in(dir.path(), root, [0; 32]);
        let keys: Vec<Key> = (0..4u8).map(|n| cache.key(Mode::Check, "a.ts", &[n])).collect();

        for (n, key) in keys.iter().enumerate() {
            cache.put(*key, &FileOutcome::new(format!("{n}.ts"), None));
        }

        cache.flush_with_budget(2).unwrap();

        let cache = Cache::open_in(dir.path(), root, [0; 32]);

        assert!(cache.get(&keys[0]).is_some());

        cache.put(cache.key(Mode::Check, "new.ts", b""), &FileOutcome::new("new.ts", None));
        cache.flush_with_budget(2).unwrap();

        let cache = Cache::open_in(dir.path(), root, [0; 32]);
        let kept: Vec<bool> = keys.iter().map(|key| cache.get(key).is_some()).collect();

        assert_eq!(kept, [true, false, false, false]);
        assert!(cache.get(&cache.key(Mode::Check, "new.ts", b"")).is_some());
    }

    #[test]
    fn under_budget_keeps_everything() {
        let dir = tempfile::tempdir().unwrap();
        let root = Path::new("/repo");
        let cache = Cache::open_in(dir.path(), root, [0; 32]);
        let old = cache.key(Mode::Format, "old.ts", b"");

        cache.put(old, &FileOutcome::new("old.ts", None));
        cache.flush().unwrap();

        let cache = Cache::open_in(dir.path(), root, [0; 32]);

        cache.put(cache.key(Mode::Format, "new.ts", b""), &FileOutcome::new("new.ts", None));
        cache.flush().unwrap();

        assert!(Cache::open_in(dir.path(), root, [0; 32]).get(&old).is_some());
    }
}
