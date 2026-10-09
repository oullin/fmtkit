//! The watcher on macOS: kqueue, with a descriptor per watched file and
//! directory.

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io;
use std::os::fd::{AsRawFd, RawFd};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::Path;

use kqueue::{EventData, EventFilter, FilterFlag, Ident, Vnode};
use rustix::process::{Resource, Rlimit, getrlimit, setrlimit};

use super::{Kind, REPORTS, Touched, absolute, is_under};

/// Open for events only, so the descriptor keeps no volume from being
/// unmounted; never block on a FIFO; never follow a symbolic link. Darwin's
/// `O_EVTONLY`, `O_NONBLOCK`, and `O_NOFOLLOW`.
const OPEN: i32 = 0x8000 | 0x0004 | 0x0100;

/// Descriptors left for everything else the process opens.
const SPARE: u64 = 1024;

pub(crate) struct Watcher {
    queue: kqueue::Watcher,
    paths: HashMap<String, Watched>,
    fds: HashMap<RawFd, String>,
    /// Counts drains; a watch added during a run carries that run's count.
    run: u64,
    /// How many descriptors the process may hold.
    limit: u64,
}

struct Watched {
    file: File,
    kind: Kind,
    since: u64,
}

impl Watcher {
    pub(crate) fn new() -> io::Result<Self> {
        Ok(Self { queue: kqueue::Watcher::new()?, paths: HashMap::new(), fds: HashMap::new(), run: 0, limit: raise_limit() })
    }

    /// Watch each of `paths` not watched yet. A watch whose descriptor does
    /// not hold what its path names once the watch is in place (the path was
    /// replaced in between) is dropped, as is a path that cannot be opened:
    /// it is not watched.
    pub(crate) fn add<'r>(&mut self, root: &Path, paths: impl IntoIterator<Item = (&'r str, Kind)>) -> io::Result<()> {
        let mut fresh = Vec::new();

        for (rel, kind) in paths {
            if self.paths.contains_key(rel) {
                continue;
            }

            if self.paths.len() as u64 + SPARE >= self.limit {
                return Err(io::Error::other(format!("more files and directories to watch than the limit of {} open files allows", self.limit)));
            }

            let file = match OpenOptions::new().read(true).custom_flags(OPEN).open(absolute(root, rel)) {
                Ok(file) => file,
                Err(err) if exhausted(&err) => return Err(err),
                Err(_) => continue,
            };
            let fd = file.as_raw_fd();

            self.queue.add_fd(fd, EventFilter::EVFILT_VNODE, flags())?;
            self.fds.insert(fd, rel.to_owned());
            self.paths.insert(rel.to_owned(), Watched { file, kind, since: self.run });
            fresh.push(rel);
        }

        if fresh.is_empty() {
            return Ok(());
        }

        self.queue.watch()?;

        for rel in fresh {
            let held = &self.paths[rel];
            let same = held.file.metadata().ok().zip(fs::symlink_metadata(absolute(root, rel)).ok()).is_some_and(|(held_meta, now)| {
                (held_meta.dev(), held_meta.ino()) == (now.dev(), now.ino()) && if held.kind == Kind::Dir { now.is_dir() } else { now.is_file() }
            });

            if !same {
                self.forget(rel);
            }
        }

        Ok(())
    }

    pub(crate) fn watches(&self, rel: &str, kind: Kind) -> bool {
        self.paths.get(rel).is_some_and(|watched| watched.kind == kind)
    }

    /// Whether `rel` has been watched since before this run.
    pub(crate) fn watched_before(&self, rel: &str, kind: Kind) -> bool {
        self.paths.get(rel).is_some_and(|watched| watched.kind == kind && watched.since < self.run)
    }

    pub(crate) fn retain_files(&mut self, keep: impl Fn(&str) -> bool) {
        let dropped: Vec<String> = self.paths.iter().filter(|(rel, watched)| watched.kind == Kind::File && !keep(rel)).map(|(rel, _)| rel.clone()).collect();

        for rel in dropped {
            self.forget(&rel);
        }
    }

    /// What was reported since the last drain, or `None` when there was too
    /// much to sort out. A path removed or renamed is no longer watched.
    pub(crate) fn drain(&mut self) -> Option<Touched> {
        self.run += 1;

        let mut touched = Touched::default();
        let mut gone = Vec::new();
        let mut reports = 0;

        while let Some(event) = self.queue.poll(None) {
            reports += 1;

            let (Ident::Fd(fd), EventData::Vnode(vnode)) = (&event.ident, &event.data) else {
                return None;
            };

            if reports > REPORTS {
                return None;
            }

            let Some(rel) = self.fds.get(fd) else {
                continue;
            };

            let replaced = matches!(vnode, Vnode::Delete | Vnode::Rename | Vnode::Revoke);

            if replaced {
                gone.push(rel.clone());
            }

            if replaced && self.paths[rel].kind == Kind::Dir {
                touched.subtrees.push(rel.clone());
            } else {
                touched.paths.insert(rel.clone());
            }
        }

        for rel in gone {
            self.forget_under(&rel);
        }

        Some(touched)
    }

    /// Stop watching `rel` and everything below it.
    fn forget_under(&mut self, rel: &str) {
        let dropped: Vec<String> = self.paths.keys().filter(|path| is_under(path, rel)).cloned().collect();

        for path in dropped {
            self.forget(&path);
        }
    }

    fn forget(&mut self, rel: &str) {
        if let Some(watched) = self.paths.remove(rel) {
            let fd = watched.file.as_raw_fd();

            // Unregistered before the descriptor closes, as `kqueue` needs.
            let _ = self.queue.remove_fd(fd, EventFilter::EVFILT_VNODE);
            self.fds.remove(&fd);
        }
    }
}

fn flags() -> FilterFlag {
    FilterFlag::NOTE_DELETE
        | FilterFlag::NOTE_WRITE
        | FilterFlag::NOTE_EXTEND
        | FilterFlag::NOTE_ATTRIB
        | FilterFlag::NOTE_LINK
        | FilterFlag::NOTE_RENAME
        | FilterFlag::NOTE_REVOKE
}

/// Whether opening failed for want of descriptors.
fn exhausted(err: &io::Error) -> bool {
    // EMFILE and ENFILE.
    matches!(err.raw_os_error(), Some(23 | 24))
}

/// Raise the limit on open files as far as the system lets this process;
/// the limit, after. A watch holds a descriptor per file and directory.
fn raise_limit() -> u64 {
    let Rlimit { current, maximum } = getrlimit(Resource::Nofile);
    let current = current.unwrap_or(u64::MAX);

    for wanted in [maximum, Some(1 << 20), Some(1 << 18), Some(1 << 16), Some(10_240)].into_iter().flatten() {
        if wanted <= current {
            break;
        }

        if maximum.is_none_or(|maximum| wanted <= maximum) && setrlimit(Resource::Nofile, Rlimit { current: Some(wanted), maximum }).is_ok() {
            return wanted;
        }
    }

    current
}
