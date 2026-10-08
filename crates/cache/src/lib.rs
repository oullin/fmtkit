//! The outcome cache.
//!
//! A file whose bytes, path, fmtkit version, outcome-relevant configuration,
//! and mode match a stored entry reuses the stored [`FileOutcome`] without
//! being parsed. A hit never needs the formatter: format-mode outcomes are
//! stored only when there was nothing to write, and check mode writes nothing,
//! so its outcomes are stored whether or not formatting would change the file.
//! Failed outcomes are never stored. Entries live in the user cache directory, one
//! store per repository root. Every failure degrades to a miss.
//!
//! A store also keeps a few marks: opaque keys a caller sets when a check
//! that is not tied to one file passed (`go vet` over a module), so a later
//! run with the same inputs can skip it. Only the most recently used
//! [`MARKS`] survive a flush.
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
use std::sync::{Mutex, OnceLock, PoisonError, RwLock};
use std::time::{Duration, SystemTime};

use fmtkit_core::{FileOutcome, Mode};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

/// Names a directory that holds the stores instead of `<cache dir>/fmtkit/v2`.
pub const CACHE_DIR_ENV: &str = "FMTKIT_CACHE_DIR";

/// Past this many entries, a flush keeps only the entries this run used.
pub const BUDGET: usize = 200_000;

/// How much older than the run a file's modification time must be before its
/// stamp is trusted; coarse file systems keep two-second timestamps.
pub const SETTLE: Duration = Duration::from_secs(2);

/// Leads every store file; bump it when the layout changes.
const MAGIC: &[u8; 8] = b"fmtkit\x02\x03";

const SHARDS: usize = 16;

/// How many marks a store keeps.
pub const MARKS: usize = 64;

/// The digest a lookup is keyed by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Key(pub [u8; 32]);

pub struct Cache {
    store: Option<Store>,
}

/// What a file's metadata says about its content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stamp {
    dev: u64,
    ino: u64,
    size: u64,
    /// Nanoseconds since the Unix epoch.
    mtime: i64,
    ctime: i64,
}

/// A file that was read because no stored outcome matched its stamp.
pub struct Fresh {
    pub bytes: Vec<u8>,
    pub key: Key,
    mode: Mode,
    /// The path's key and the stamp taken before the read.
    stamp: Option<(Key, Stamp)>,
}

pub enum Lookup {
    Hit(FileOutcome),
    Miss(Fresh),
}

struct Store {
    path: PathBuf,
    config_hash: [u8; 32],
    /// Stamps modified at or after this many nanoseconds are not remembered.
    settled: i64,
    tables: OnceLock<Tables>,
    /// Whether a put added or replaced an entry since the store was read.
    dirty: AtomicBool,
}

struct Tables {
    outcomes: Shards<Slot>,
    /// Path key to the stamp the path had and the content key it held then.
    stamps: Shards<(Stamp, Key)>,
    /// Least recently used first.
    marks: Mutex<Vec<[u8; 32]>>,
}

struct Shards<V>([RwLock<FxHashMap<[u8; 32], V>>; SHARDS]);

/// An outcome kept encoded: a run decodes only the outcomes it hits, on the
/// worker that hits them, and a store of thousands of findings loads and
/// frees as a few thousand buffers.
struct Slot {
    encoded: Box<[u8]>,
    /// Hit or put during this run, so kept when the store is pruned.
    used: AtomicBool,
}

#[derive(Serialize, Deserialize)]
struct Stored<'a> {
    version: &'a str,
    /// Each outcome as `postcard`-encoded [`wire::Outcome`].
    #[serde(borrow)]
    entries: Vec<([u8; 32], &'a [u8])>,
    stamps: Vec<([u8; 32], Stamp, [u8; 32])>,
    marks: Vec<[u8; 32]>,
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

        let settled = SystemTime::now().checked_sub(SETTLE).map_or(0, nanos_since_epoch);

        Self { store: Some(Store { path, config_hash, settled, tables: OnceLock::new(), dirty: AtomicBool::new(false) }) }
    }

    /// Read the store now rather than at the first lookup, so the read can
    /// overlap other work.
    #[must_use]
    pub fn loaded(self) -> Self {
        if let Some(store) = &self.store {
            store.tables();
        }

        self
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

        let mut hasher = blake3::Hasher::new();

        hasher.update(&(fmtkit_core::VERSION.len() as u64).to_le_bytes());
        hasher.update(fmtkit_core::VERSION.as_bytes());
        hasher.update(&store.config_hash);
        hasher.update(&[mode_byte(mode)]);
        hasher.update(&(rel.len() as u64).to_le_bytes());
        hasher.update(rel.as_bytes());
        hasher.update(content);

        Key(*hasher.finalize().as_bytes())
    }

    /// Look up the file at `path`, known as `rel`. It is opened only when its
    /// stamp does not answer the lookup; a content hit then remembers the
    /// stamp for the next run. Thread-safe.
    pub fn read(&self, mode: Mode, rel: &str, path: &Path) -> io::Result<Lookup> {
        let stamp = self.store.as_ref().and_then(|store| Some((store.path_key(mode, rel), Stamp::of(path)?)));

        if let Some((path_key, stamp)) = &stamp
            && let Some(outcome) = self.get_stamped(path_key, stamp)
        {
            return Ok(Lookup::Hit(outcome));
        }

        let bytes = fs::read(path)?;
        let fresh = Fresh { key: self.key(mode, rel, &bytes), bytes, mode, stamp };

        match self.get(&fresh.key) {
            Some(outcome) => {
                self.remember(&fresh);

                Ok(Lookup::Hit(outcome))
            }
            None => Ok(Lookup::Miss(fresh)),
        }
    }

    /// Thread-safe.
    pub fn get(&self, key: &Key) -> Option<FileOutcome> {
        let store = self.store.as_ref()?;
        let shard = store.tables().outcomes.shard(key).read().unwrap_or_else(PoisonError::into_inner);
        let slot = shard.get(&key.0)?;
        let outcome = postcard::from_bytes::<wire::Outcome>(&slot.encoded).ok()?;

        slot.used.store(true, Ordering::Relaxed);

        Some(outcome.into())
    }

    fn get_stamped(&self, path_key: &Key, stamp: &Stamp) -> Option<FileOutcome> {
        let store = self.store.as_ref()?;
        let key = {
            let shard = store.tables().stamps.shard(path_key).read().unwrap_or_else(PoisonError::into_inner);
            let (stored, key) = shard.get(&path_key.0)?;

            (stored == stamp).then_some(*key)?
        };

        self.get(&key)
    }

    /// Store the outcome of a file [`Cache::read`] returned, and remember its
    /// stamp. In check mode an outcome that would change the file is stored
    /// too; failed outcomes never are. Thread-safe.
    pub fn put_fresh(&self, fresh: &Fresh, outcome: &FileOutcome) {
        if outcome.error.is_none() && (!outcome.changed || fresh.mode == Mode::Check) {
            self.insert(fresh.key, outcome);
            self.remember(fresh);
        }
    }

    /// Point the path's stamp at the content key it was read with, unless the
    /// file changed too recently for the stamp to be trusted.
    fn remember(&self, fresh: &Fresh) {
        let (Some(store), Some((path_key, stamp))) = (&self.store, fresh.stamp) else {
            return;
        };

        if stamp.mtime >= store.settled {
            return;
        }

        let mut shard = store.tables().stamps.shard(&path_key).write().unwrap_or_else(PoisonError::into_inner);

        if shard.get(&path_key.0) == Some(&(stamp, fresh.key)) {
            return;
        }

        shard.insert(path_key.0, (stamp, fresh.key));
        store.dirty.store(true, Ordering::Relaxed);
    }

    /// Thread-safe. Outcomes that changed the file or failed are ignored.
    pub fn put(&self, key: Key, outcome: &FileOutcome) {
        if !outcome.changed && outcome.error.is_none() {
            self.insert(key, outcome);
        }
    }

    fn insert(&self, key: Key, outcome: &FileOutcome) {
        let Some(store) = &self.store else {
            return;
        };

        let Ok(encoded) = postcard::to_stdvec(&wire::Outcome::from(outcome)) else {
            return;
        };

        let mut shard = store.tables().outcomes.shard(&key).write().unwrap_or_else(PoisonError::into_inner);

        if let Some(slot) = shard.get(&key.0)
            && *slot.encoded == *encoded
        {
            slot.used.store(true, Ordering::Relaxed);

            return;
        }

        shard.insert(key.0, Slot { encoded: encoded.into_boxed_slice(), used: AtomicBool::new(true) });
        store.dirty.store(true, Ordering::Relaxed);
    }

    /// Whether `key` was marked, by this run or an earlier one. Thread-safe.
    pub fn marked(&self, key: &Key) -> bool {
        let Some(store) = &self.store else {
            return false;
        };

        let mut marks = store.tables().marks.lock().unwrap_or_else(PoisonError::into_inner);

        let Some(at) = marks.iter().position(|mark| *mark == key.0) else {
            return false;
        };

        let mark = marks.remove(at);

        marks.push(mark);

        true
    }

    /// Set a mark, forgetting the least recently used one past [`MARKS`].
    /// Thread-safe.
    pub fn mark(&self, key: Key) {
        let Some(store) = &self.store else {
            return;
        };

        if self.marked(&key) {
            return;
        }

        let mut marks = store.tables().marks.lock().unwrap_or_else(PoisonError::into_inner);

        marks.push(key.0);

        let excess = marks.len().saturating_sub(MARKS);

        marks.drain(..excess);
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

        let (Some(tables), true) = (store.tables.into_inner(), store.dirty.into_inner()) else {
            return Ok(());
        };

        let mut entries = tables.outcomes.into_vec();

        if entries.len() > budget {
            entries.retain(|(_, slot)| slot.used.load(Ordering::Relaxed));
        }

        entries.sort_unstable_by_key(|entry| entry.0);

        let kept = |key: &Key| entries.binary_search_by_key(&key.0, |entry| entry.0).is_ok();
        let mut stamps: Vec<([u8; 32], Stamp, [u8; 32])> =
            tables.stamps.into_vec().into_iter().filter(|(_, (_, key))| kept(key)).map(|(path_key, (stamp, key))| (path_key, stamp, key.0)).collect();

        stamps.sort_unstable_by_key(|entry| entry.0);

        let marks = tables.marks.into_inner().unwrap_or_else(PoisonError::into_inner);
        let stored = Stored { version: fmtkit_core::VERSION, entries: entries.iter().map(|(key, slot)| (*key, &*slot.encoded)).collect(), stamps, marks };
        let mut bytes = MAGIC.to_vec();

        bytes.extend(postcard::to_stdvec(&stored).map_err(io::Error::other)?);

        write_atomic(&store.path, &bytes)
    }
}

impl Store {
    fn tables(&self) -> &Tables {
        self.tables.get_or_init(|| Tables::load(&self.path))
    }

    /// The key a path's stamp is stored under.
    fn path_key(&self, mode: Mode, rel: &str) -> Key {
        let mut hasher = blake3::Hasher::new();

        hasher.update(&self.config_hash);
        hasher.update(&[mode_byte(mode)]);
        hasher.update(&(rel.len() as u64).to_le_bytes());
        hasher.update(rel.as_bytes());

        Key(*hasher.finalize().as_bytes())
    }
}

impl Tables {
    /// Read a store file. A missing, foreign, corrupt, or other-version file
    /// reads as empty.
    fn load(path: &Path) -> Self {
        let mut tables = Self { outcomes: Shards::empty(), stamps: Shards::empty(), marks: Mutex::default() };

        let Ok(bytes) = fs::read(path) else {
            return tables;
        };

        let Some(body) = bytes.strip_prefix(MAGIC) else {
            return tables;
        };

        let Ok(stored) = postcard::from_bytes::<Stored>(body) else {
            return tables;
        };

        if stored.version != fmtkit_core::VERSION {
            return tables;
        }

        for (key, encoded) in stored.entries {
            tables.outcomes.insert(key, Slot { encoded: encoded.into(), used: AtomicBool::new(false) });
        }

        for (path_key, stamp, key) in stored.stamps {
            tables.stamps.insert(path_key, (stamp, Key(key)));
        }

        *tables.marks.get_mut().unwrap_or_else(PoisonError::into_inner) = stored.marks;

        tables
    }
}

impl<V> Shards<V> {
    fn empty() -> Self {
        Self(std::array::from_fn(|_| RwLock::default()))
    }

    fn insert(&mut self, key: [u8; 32], value: V) {
        self.0[index(&Key(key))].get_mut().unwrap_or_else(PoisonError::into_inner).insert(key, value);
    }

    fn shard(&self, key: &Key) -> &RwLock<FxHashMap<[u8; 32], V>> {
        &self.0[index(key)]
    }

    fn into_vec(self) -> Vec<([u8; 32], V)> {
        self.0.into_iter().flat_map(|shard| shard.into_inner().unwrap_or_else(PoisonError::into_inner)).collect()
    }
}

impl Stamp {
    /// The stamp of the regular file at `path`, or `None` when it cannot be
    /// read or the platform has no inode numbers.
    #[cfg(unix)]
    fn of(path: &Path) -> Option<Self> {
        use std::os::unix::fs::MetadataExt;

        let meta = fs::metadata(path).ok()?;

        meta.is_file().then(|| Self {
            dev: meta.dev(),
            ino: meta.ino(),
            size: meta.size(),
            mtime: meta.mtime().saturating_mul(1_000_000_000).saturating_add(meta.mtime_nsec()),
            ctime: meta.ctime().saturating_mul(1_000_000_000).saturating_add(meta.ctime_nsec()),
        })
    }

    #[cfg(not(unix))]
    fn of(_path: &Path) -> Option<Self> {
        None
    }
}

fn mode_byte(mode: Mode) -> u8 {
    match mode {
        Mode::Format => 0,
        Mode::Check => 1,
    }
}

fn nanos_since_epoch(time: SystemTime) -> i64 {
    time.duration_since(SystemTime::UNIX_EPOCH).map_or(0, |since| i64::try_from(since.as_nanos()).unwrap_or(i64::MAX))
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

    /// Write `content` to `path`, dated an hour ago so its stamp is settled.
    fn settled_file(path: &Path, content: &[u8]) {
        let hour_ago = SystemTime::now() - Duration::from_secs(3600);

        fs::write(path, content).unwrap();
        fs::File::options().write(true).open(path).unwrap().set_modified(hour_ago).unwrap();
    }

    fn miss(cache: &Cache, path: &Path) -> Fresh {
        match cache.read(Mode::Check, "a.ts", path).unwrap() {
            Lookup::Miss(fresh) => fresh,
            Lookup::Hit(_) => panic!("expected a miss"),
        }
    }

    fn stamp_hit(cache: &Cache, path: &Path) -> Option<FileOutcome> {
        let store = cache.store.as_ref().unwrap();

        cache.get_stamped(&store.path_key(Mode::Check, "a.ts"), &Stamp::of(path)?)
    }

    #[test]
    fn settled_stamps_answer_without_reading() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.ts");
        let store = dir.path().join("store");
        let cache = Cache::open_in(&store, Path::new("/repo"), [0; 32]);

        settled_file(&file, b"let a = 1;");

        let fresh = miss(&cache, &file);

        assert_eq!(fresh.bytes, b"let a = 1;");
        cache.put_fresh(&fresh, &FileOutcome::new("a.ts", None));
        cache.flush().unwrap();

        let cache = Cache::open_in(&store, Path::new("/repo"), [0; 32]);

        assert_eq!(stamp_hit(&cache, &file), Some(FileOutcome::new("a.ts", None)));
        assert!(matches!(cache.read(Mode::Check, "a.ts", &file).unwrap(), Lookup::Hit(_)));

        // The same size and modification time, but the change time moved on.
        settled_file(&file, b"let b = 2;");

        assert_eq!(stamp_hit(&cache, &file), None);
        assert_eq!(miss(&cache, &file).bytes, b"let b = 2;");

        // Another configuration never sees this one's stamps.
        assert_eq!(stamp_hit(&Cache::open_in(&store, Path::new("/repo"), [1; 32]), &file), None);
    }

    #[test]
    fn recent_stamps_are_not_remembered() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.ts");
        let store = dir.path().join("store");
        let cache = Cache::open_in(&store, Path::new("/repo"), [0; 32]);

        fs::write(&file, b"let a = 1;").unwrap();
        cache.put_fresh(&miss(&cache, &file), &FileOutcome::new("a.ts", None));
        cache.flush().unwrap();

        let cache = Cache::open_in(&store, Path::new("/repo"), [0; 32]);

        assert_eq!(stamp_hit(&cache, &file), None);

        // The content still hits, and the stamp is remembered once settled.
        settled_file(&file, b"let a = 1;");

        assert!(matches!(cache.read(Mode::Check, "a.ts", &file).unwrap(), Lookup::Hit(_)));
        assert!(stamp_hit(&cache, &file).is_some());
    }

    #[test]
    fn stamps_follow_their_outcomes_out_of_the_store() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.ts");
        let store = dir.path().join("store");
        let cache = Cache::open_in(&store, Path::new("/repo"), [0; 32]);

        settled_file(&file, b"old");
        cache.put_fresh(&miss(&cache, &file), &FileOutcome::new("a.ts", None));
        cache.put(cache.key(Mode::Check, "b.ts", b""), &FileOutcome::new("b.ts", None));
        cache.flush().unwrap();

        // Only b.ts is used, so pruning drops a.ts and the stamp pointing at it.
        let cache = Cache::open_in(&store, Path::new("/repo"), [0; 32]);

        assert!(cache.get(&cache.key(Mode::Check, "b.ts", b"")).is_some());
        cache.put(cache.key(Mode::Check, "c.ts", b""), &FileOutcome::new("c.ts", None));
        cache.flush_with_budget(1).unwrap();

        let cache = Cache::open_in(&store, Path::new("/repo"), [0; 32]);

        assert!(cache.store.as_ref().unwrap().tables().stamps.0.iter().all(|shard| shard.read().unwrap().is_empty()));
        assert_eq!(stamp_hit(&cache, &file), None);
    }

    #[test]
    fn check_mode_keeps_outcomes_that_would_change_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.ts");
        let cache = Cache::open_in(&dir.path().join("store"), Path::new("/repo"), [0; 32]);
        let changed = FileOutcome { changed: true, ..FileOutcome::new("a.ts", None) };

        fs::write(&file, b"let  a").unwrap();

        let Lookup::Miss(format) = cache.read(Mode::Format, "a.ts", &file).unwrap() else { panic!("expected a miss") };

        cache.put_fresh(&format, &changed);

        assert_eq!(cache.get(&format.key), None);

        let check = miss(&cache, &file);

        cache.put_fresh(&check, &FileOutcome { error: Some("parse".into()), ..changed.clone() });

        assert_eq!(cache.get(&check.key), None);

        cache.put_fresh(&check, &changed);

        assert_eq!(cache.get(&check.key), Some(changed));
    }

    #[test]
    fn disabled_caches_still_read() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.ts");

        fs::write(&file, b"x").unwrap();

        let Lookup::Miss(fresh) = Cache::disabled().read(Mode::Format, "a.ts", &file).unwrap() else { panic!("expected a miss") };

        assert_eq!((fresh.bytes.as_slice(), fresh.key), (b"x".as_slice(), Key([0; 32])));
        assert!(Cache::disabled().read(Mode::Format, "a.ts", &dir.path().join("gone.ts")).is_err());
    }

    #[test]
    fn keeps_the_most_recently_used_marks() {
        let dir = tempfile::tempdir().unwrap();
        let root = Path::new("/repo");
        let key = |n: usize| Key(blake3::hash(&n.to_le_bytes()).into());
        let cache = Cache::open_in(dir.path(), root, [0; 32]);

        assert!(!cache.marked(&key(0)));

        for n in 0..MARKS {
            cache.mark(key(n));
        }

        // Using the oldest mark makes the second oldest the one to go.
        assert!(cache.marked(&key(0)));

        cache.mark(key(MARKS));
        cache.flush().unwrap();

        let cache = Cache::open_in(dir.path(), root, [0; 32]);

        assert!(cache.marked(&key(0)));
        assert!(!cache.marked(&key(1)));
        assert!((2..=MARKS).all(|n| cache.marked(&key(n))));
        assert!(!Cache::disabled().marked(&key(0)));

        Cache::disabled().mark(key(0));
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
