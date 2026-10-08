//! What discovery remembers between runs, so an unchanged tree is not listed
//! again.
//!
//! Each directory the git walk lists is stored with its stamp (device, inode,
//! size, and modification and change times) and what the listing yielded:
//! the regular files and subdirectories git's rules do not ignore, whether it
//! holds a nested repository, and the stamp of its `.gitignore`. A later walk
//! that finds the same stamp reuses the listing; adding, removing, or renaming
//! an entry changes the directory's stamp. A listing is trusted only while
//! every ignore rule it was filtered by is unchanged: a changed `.gitignore`
//! re-lists its directory and everything below it, and a changed
//! `info/exclude`, global excludes file, or `core.ignoreCase` setting drops
//! every listing.
//!
//! It also keeps the paths that differ between `HEAD` and the index, keyed by
//! the `HEAD` tree and the index checksum. Both name content, so the result
//! holds for as long as they are unchanged.
//!
//! A directory or ignore file modified within [`SETTLE`] of the run is never
//! stored, since a coarse timestamp could miss a second change in the same
//! tick. The store is one file, read once and rewritten atomically when a run
//! listed something new. A process that runs many times keeps one `Memory`,
//! calls [`Memory::next_run`] before each run and [`Memory::save`] after it.

use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock, PoisonError};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

/// How much older than the run a directory or ignore file must be before its
/// stamp is trusted; coarse file systems keep two-second timestamps.
pub const SETTLE: Duration = Duration::from_secs(2);

/// Past this many listings, a save keeps only the ones this run used.
const BUDGET: usize = 100_000;

/// Leads every store file; bump it when the layout changes.
const MAGIC: &[u8; 8] = b"fmtkit\x03\x02";

/// Directory listings kept between runs, in a file of their own.
pub struct Memory {
    path: PathBuf,
    /// Nanoseconds since the epoch before which a stamp is settled.
    settled: i64,
    stored: OnceLock<Stored>,
    fresh: Mutex<HashMap<String, Listing>>,
    fresh_staged: OnceLock<Staged>,
    /// The rules this run's listings were filtered by, once the walk knows them.
    context: OnceLock<Context>,
    /// Whether this run's context matches the stored one.
    trusted: AtomicBool,
}

/// The identity of a file or directory at the moment it was read.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Stamp {
    dev: u64,
    ino: u64,
    size: u64,
    mtime: i64,
    ctime: i64,
}

/// What listing one directory under git's rules yielded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Listing {
    pub stamp: Stamp,
    /// The stamp of the directory's `.gitignore`, when it has one.
    pub rules: Option<Stamp>,
    /// Whether the directory holds a `.git` entry (a nested repository).
    pub nested: bool,
    /// Regular files the rules keep, by name.
    pub files: Vec<String>,
    /// Subdirectories the rules keep, by name.
    pub dirs: Vec<String>,
}

/// The rules outside the tree that every listing was filtered by.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub(crate) struct Context {
    pub version: String,
    pub ignore_case: bool,
    /// `info/exclude` and the global excludes file, with their stamps.
    pub files: Vec<(PathBuf, Option<Stamp>)>,
}

/// The paths of the regular files that differ between a `HEAD` tree and an
/// index, both named by their hashes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Staged {
    pub tree: Vec<u8>,
    pub index: Vec<u8>,
    pub paths: Vec<String>,
}

#[derive(Default, Serialize, Deserialize)]
struct Stored {
    context: Context,
    listings: HashMap<String, Listing>,
    staged: Option<Staged>,
    #[serde(skip)]
    used: HashMap<String, AtomicBool>,
}

impl Memory {
    /// The memory kept in `path`. Nothing is read until the first lookup.
    pub fn open(path: impl Into<PathBuf>) -> Self {
        Self::open_as_of(path, SystemTime::now())
    }

    /// [`Memory::open`] for a run taking place at `now`, which decides what
    /// has settled. Tests use a later `now` to store what they just wrote.
    pub fn open_as_of(path: impl Into<PathBuf>, now: SystemTime) -> Self {
        Self {
            path: path.into(),
            settled: settled_before(now),
            stored: OnceLock::new(),
            fresh: Mutex::new(HashMap::new()),
            fresh_staged: OnceLock::new(),
            context: OnceLock::new(),
            trusted: AtomicBool::new(false),
        }
    }

    /// Read the store now rather than at the first lookup, so the read can
    /// overlap other work.
    pub fn load(&self) {
        self.stored();
    }

    /// Begin another run taking place at `now`.
    pub fn next_run(&mut self, now: SystemTime) {
        self.settled = settled_before(now);
    }

    /// Settle the rules this run filters by. Listings stored under other
    /// rules are not used, and are dropped by the next save.
    pub(crate) fn begin(&self, context: Context) -> bool {
        let trusted = self.stored().context == context && context.files.iter().all(|(_, stamp)| stamp.is_none_or(|stamp| self.is_settled(&stamp)));

        let _ = self.context.set(context);

        self.trusted.store(trusted, Ordering::Relaxed);

        trusted
    }

    /// The stored listing of `rel` when it still has `stamp`.
    pub(crate) fn listing(&self, rel: &str, stamp: &Stamp) -> Option<&Listing> {
        if !self.trusted.load(Ordering::Relaxed) {
            return None;
        }

        let stored = self.stored();
        let listing = stored.listings.get(rel).filter(|listing| listing.stamp == *stamp)?;

        if let Some(used) = stored.used.get(rel) {
            used.store(true, Ordering::Relaxed);
        }

        Some(listing)
    }

    /// The stored listing of `rel`, whatever its stamp.
    pub(crate) fn previous(&self, rel: &str) -> Option<&Listing> {
        self.trusted.load(Ordering::Relaxed).then(|| self.stored().listings.get(rel)).flatten()
    }

    /// Remember `listing` for `rel` when it and its ignore file have settled.
    pub(crate) fn remember(&self, rel: &str, listing: Listing) {
        if self.is_settled(&listing.stamp) && listing.rules.is_none_or(|rules| self.is_settled(&rules)) {
            self.fresh.lock().unwrap_or_else(PoisonError::into_inner).insert(rel.to_owned(), listing);
        }
    }

    /// The paths that differ between the tree and the index with these
    /// hashes, when a run has compared them before.
    pub(crate) fn staged(&self, tree: &[u8], index: &[u8]) -> Option<&[String]> {
        self.stored().staged.as_ref().filter(|staged| staged.tree == tree && staged.index == index).map(|staged| staged.paths.as_slice())
    }

    /// Remember what comparing a tree with an index found.
    pub(crate) fn remember_staged(&self, staged: Staged) {
        let _ = self.fresh_staged.set(staged);
    }

    /// Fold this run's listings into the store, and write it when the run
    /// found something new. Past the budget, only the listings this run used
    /// are kept.
    pub fn save(&mut self) -> io::Result<()> {
        let fresh = std::mem::take(self.fresh.get_mut().unwrap_or_else(PoisonError::into_inner));
        let fresh_staged = self.fresh_staged.take();
        let trusted = std::mem::take(self.trusted.get_mut());

        // A run that never walked under git's rules leaves the store as it was.
        let Some(context) = self.context.take() else {
            return Ok(());
        };

        // Rules files that changed too recently may change again unseen, so
        // nothing filtered by them is kept, and the next run lists afresh.
        if context.files.iter().any(|(_, stamp)| stamp.is_some_and(|stamp| stamp.mtime >= self.settled)) {
            self.stored = OnceLock::from(Stored::default());

            return Ok(());
        }

        let changed = !fresh.is_empty() || fresh_staged.is_some() || !trusted;
        let Stored { listings, used, staged, .. } = self.stored.take().unwrap_or_default();

        let mut listings = match trusted {
            true if listings.len() + fresh.len() > BUDGET => {
                listings.into_iter().filter(|(rel, _)| used.get(rel).is_some_and(|used| used.load(Ordering::Relaxed))).collect()
            }
            true => listings,
            false => HashMap::new(),
        };

        listings.extend(fresh);

        let mut stored = Stored { context, listings, staged: fresh_staged.or(staged), used: HashMap::new() };
        let written = if changed { stored.encode().and_then(|bytes| write_atomic(&self.path, &bytes)) } else { Ok(()) };

        stored.used = unused(&stored.listings);
        self.stored = OnceLock::from(stored);

        written
    }

    fn stored(&self) -> &Stored {
        self.stored.get_or_init(|| {
            let mut stored = Stored::read(&self.path).unwrap_or_default();

            stored.used = unused(&stored.listings);
            stored
        })
    }

    fn is_settled(&self, stamp: &Stamp) -> bool {
        stamp.is_settled(self.settled)
    }
}

impl Stored {
    fn read(path: &Path) -> Option<Self> {
        let bytes = fs::read(path).ok()?;
        let body = bytes.strip_prefix(MAGIC)?;

        postcard::from_bytes(body).ok()
    }

    fn encode(&self) -> io::Result<Vec<u8>> {
        let mut bytes = MAGIC.to_vec();

        bytes.extend(postcard::to_stdvec(self).map_err(io::Error::other)?);

        Ok(bytes)
    }
}

/// A flag per listing, set when a run uses it.
fn unused(listings: &HashMap<String, Listing>) -> HashMap<String, AtomicBool> {
    listings.keys().map(|rel| (rel.clone(), AtomicBool::new(false))).collect()
}

impl Context {
    pub(crate) fn new(ignore_case: bool, files: impl IntoIterator<Item = PathBuf>) -> Self {
        let files = files.into_iter().map(|path| (Stamp::of(&path), path)).map(|(stamp, path)| (path, stamp)).collect();

        Self { version: fmtkit_core::VERSION.to_owned(), ignore_case, files }
    }
}

impl Stamp {
    /// The stamp of whatever `path` names, following a symbolic link, or
    /// `None` when it cannot be read or the platform has no inode numbers.
    pub(crate) fn of(path: &Path) -> Option<Self> {
        fs::metadata(path).ok().and_then(|meta| Self::from_metadata(&meta))
    }

    // `None` on platforms without inode numbers.
    #[cfg(unix)]
    #[allow(clippy::unnecessary_wraps)]
    pub(crate) fn from_metadata(meta: &fs::Metadata) -> Option<Self> {
        use std::os::unix::fs::MetadataExt;

        Some(Self {
            dev: meta.dev(),
            ino: meta.ino(),
            size: meta.size(),
            mtime: meta.mtime().saturating_mul(1_000_000_000).saturating_add(meta.mtime_nsec()),
            ctime: meta.ctime().saturating_mul(1_000_000_000).saturating_add(meta.ctime_nsec()),
        })
    }

    #[cfg(not(unix))]
    pub(crate) fn from_metadata(_meta: &fs::Metadata) -> Option<Self> {
        None
    }

    /// Whether the file was last written and changed before `before`,
    /// nanoseconds since the epoch.
    pub(crate) fn is_settled(&self, before: i64) -> bool {
        self.mtime < before && self.ctime < before
    }
}

/// Nanoseconds since the epoch at [`SETTLE`] before `now`.
pub(crate) fn settled_before(now: SystemTime) -> i64 {
    now.checked_sub(SETTLE).map_or(0, nanos_since_epoch)
}

fn nanos_since_epoch(time: SystemTime) -> i64 {
    time.duration_since(SystemTime::UNIX_EPOCH).map_or(0, |since| i64::try_from(since.as_nanos()).unwrap_or(i64::MAX))
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
