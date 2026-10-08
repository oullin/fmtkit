//! The watcher on Linux: inotify, with a watch per directory that reports
//! changes to the directory's entries as well as its own.

use std::collections::HashMap;
use std::io;
use std::mem::MaybeUninit;
use std::os::fd::OwnedFd;
use std::path::Path;

use rustix::fs::inotify::{self, CreateFlags, ReadFlags, WatchFlags};
use rustix::io::Errno;

use super::{Kind, REPORTS, Touched, absolute, is_under};

const MASK: WatchFlags = WatchFlags::MODIFY
    .union(WatchFlags::ATTRIB)
    .union(WatchFlags::CLOSE_WRITE)
    .union(WatchFlags::CREATE)
    .union(WatchFlags::DELETE)
    .union(WatchFlags::DELETE_SELF)
    .union(WatchFlags::MOVE_SELF)
    .union(WatchFlags::MOVED_FROM)
    .union(WatchFlags::MOVED_TO)
    .union(WatchFlags::ONLYDIR)
    .union(WatchFlags::DONT_FOLLOW)
    .union(WatchFlags::EXCL_UNLINK);

/// Reports that add or remove a directory entry.
const ENTRIES: ReadFlags = ReadFlags::CREATE.union(ReadFlags::DELETE).union(ReadFlags::MOVED_FROM).union(ReadFlags::MOVED_TO);

pub(crate) struct Watcher {
    fd: OwnedFd,
    /// Watched directories, with their watch and the run that added it.
    dirs: HashMap<String, (i32, u64)>,
    wds: HashMap<i32, String>,
    /// Counts drains; a watch added during a run carries that run's count.
    run: u64,
    buf: Vec<MaybeUninit<u8>>,
}

impl Watcher {
    pub(crate) fn new() -> io::Result<Self> {
        let fd = inotify::init(CreateFlags::CLOEXEC | CreateFlags::NONBLOCK)?;

        Ok(Self { fd, dirs: HashMap::new(), wds: HashMap::new(), run: 0, buf: vec![MaybeUninit::uninit(); 64 * 1024] })
    }

    /// Watch each directory among `paths` not watched yet; a file is watched
    /// through its directory. A path that cannot be watched (gone, or not a
    /// directory) is not.
    pub(crate) fn add<'r>(&mut self, root: &Path, paths: impl IntoIterator<Item = (&'r str, Kind)>) -> io::Result<()> {
        for (rel, kind) in paths {
            if kind != Kind::Dir || self.dirs.contains_key(rel) {
                continue;
            }

            match inotify::add_watch(&self.fd, absolute(root, rel), MASK) {
                // A path naming a directory watched under another path (a bind
                // mount) would be reported under the other: it is not watched.
                Ok(wd) if self.wds.contains_key(&wd) => {}
                Ok(wd) => {
                    self.wds.insert(wd, rel.to_owned());
                    self.dirs.insert(rel.to_owned(), (wd, self.run));
                }
                Err(Errno::NOSPC) => return Err(io::Error::other("the inotify watch limit (fs.inotify.max_user_watches) is reached")),
                Err(Errno::NOMEM) => return Err(Errno::NOMEM.into()),
                Err(_) => {}
            }
        }

        Ok(())
    }

    pub(crate) fn watches(&self, rel: &str, kind: Kind) -> bool {
        self.dirs.contains_key(directory(rel, kind))
    }

    /// Whether `rel` has been watched since before this run.
    pub(crate) fn watched_before(&self, rel: &str, kind: Kind) -> bool {
        self.dirs.get(directory(rel, kind)).is_some_and(|(_, since)| *since < self.run)
    }

    /// Files are watched through their directories.
    #[allow(clippy::unused_self)]
    pub(crate) fn retain_files(&mut self, _keep: impl Fn(&str) -> bool) {}

    /// What was reported since the last drain, or `None` when the kernel
    /// dropped reports or there were too many to sort out. A directory
    /// removed or renamed is no longer watched.
    pub(crate) fn drain(&mut self) -> Option<Touched> {
        self.run += 1;

        let mut touched = Touched::default();
        let mut gone: Vec<String> = Vec::new();
        let mut reader = inotify::Reader::new(&self.fd, &mut self.buf);
        let mut reports = 0;

        loop {
            let event = match reader.next() {
                Ok(event) => event,
                Err(Errno::AGAIN) => break,
                Err(_) => return None,
            };

            reports += 1;

            let flags = event.events();

            if reports > REPORTS || flags.intersects(ReadFlags::QUEUE_OVERFLOW | ReadFlags::UNMOUNT) {
                return None;
            }

            let Some(dir) = self.wds.get(&event.wd()) else {
                continue;
            };

            match event.file_name().map(|name| name.to_str()) {
                Some(Ok(name)) => {
                    let child = if dir.is_empty() { name.to_owned() } else { format!("{dir}/{name}") };

                    if flags.intersects(ENTRIES) {
                        touched.paths.insert(dir.clone());

                        if flags.contains(ReadFlags::ISDIR) {
                            touched.subtrees.push(child.clone());
                            gone.push(child.clone());
                        }
                    }

                    touched.paths.insert(child);
                }
                // Not UTF-8: neither listed nor tracked.
                Some(Err(_)) => {}
                None if flags.intersects(ReadFlags::DELETE_SELF | ReadFlags::MOVE_SELF | ReadFlags::IGNORED) => {
                    touched.subtrees.push(dir.clone());
                    gone.push(dir.clone());
                }
                None => {
                    touched.paths.insert(dir.clone());
                }
            }
        }

        for rel in gone {
            self.forget_under(&rel);
        }

        Some(touched)
    }

    /// Stop watching the directory `rel` and every directory below it.
    fn forget_under(&mut self, rel: &str) {
        let dropped: Vec<String> = self.dirs.keys().filter(|path| is_under(path, rel)).cloned().collect();

        for path in dropped {
            if let Some((wd, _)) = self.dirs.remove(&path) {
                // Already gone when the kernel dropped it.
                let _ = inotify::remove_watch(&self.fd, wd);
                self.wds.remove(&wd);
            }
        }
    }
}

/// The directory whose watch covers `rel`.
fn directory(rel: &str, kind: Kind) -> &str {
    match kind {
        Kind::Dir => rel,
        Kind::File => rel.rsplit_once('/').map_or("", |(dir, _)| dir),
    }
}
