//! `fmtkit serve`: a process that keeps one repository's [`Session`] warm and
//! runs the `format` and `check` calls other fmtkit processes hand it.
//!
//! The server listens on [`fmtkit_cache::socket`] for its repository root,
//! readable and writable only by its owner, and runs one call at a time.
//! Each message is a frame: a little-endian `u32` length, then a `postcard`
//! value. A caller sends one [`Request`]; the server answers with progress
//! while the run goes, then the exit code and the bytes to print, and writes
//! what the run learned only after answering.
//!
//! The server refuses a call from another fmtkit version or executable, or
//! made with different values of the variables a run reads, and the caller
//! runs the call itself. Once its own executable has changed on disk, the
//! server refuses every call and stops. The server logs each call it
//! answers or refuses on stderr.

use std::ffi::OsString;
use std::fs::{self, Permissions};
use std::io::{self, Read, Write};
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant, SystemTime};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use fmtkit_core::VERSION;
use fmtkit_engine::{Progress, Session};

use crate::{Call, EXIT_INTERNAL, EXIT_USAGE, execute, fail, progress_bar};

/// Changes whenever [`Request`] or [`Reply`] does.
const PROTO: u32 = 1;

/// Larger frames are refused rather than allocated.
const MAX_FRAME: usize = 1 << 30;

/// How long the server waits for a caller to send its request, or to take
/// a reply, before moving on to the next caller.
const READ_TIMEOUT: Duration = Duration::from_secs(10);
const WRITE_TIMEOUT: Duration = Duration::from_secs(60);

/// The variables a run reads, directly or through git, go, the C toolchain,
/// and the cache and configuration lookups: every variable with one of
/// these prefixes, and these names.
const PREFIXES: [&str; 4] = ["FMTKIT_", "GIT_", "GO", "CGO_"];
const NAMES: [&str; 9] = ["AR", "CC", "CXX", "FC", "HOME", "PATH", "PKG_CONFIG", "XDG_CACHE_HOME", "XDG_CONFIG_HOME"];

#[derive(Serialize, Deserialize)]
struct Request<C> {
    proto: u32,
    version: String,
    /// The caller's executable, canonical.
    exe: Option<PathBuf>,
    /// [`environment`] in the caller.
    env: Vec<(OsString, OsString)>,
    call: C,
}

#[derive(Serialize, Deserialize)]
enum Reply {
    Progress { done: usize, total: usize },
    Done { code: u8, stdout: Vec<u8>, stderr: Vec<u8> },
    Refused(String),
}

/// What identifies an executable on disk: a rebuilt or upgraded binary
/// differs in at least one field.
#[derive(PartialEq, Eq)]
struct Executable {
    path: PathBuf,
    len: u64,
    modified: Option<SystemTime>,
}

/// Serve the repository that contains `cwd` until stopped, or until this
/// executable changes.
pub(crate) fn serve(cwd: &Path) -> ExitCode {
    let root = fmtkit_discover::find_root(cwd);

    let Some(socket) = fmtkit_cache::socket(&root) else {
        return fail(EXIT_USAGE, "serve needs a cache directory to hold its socket");
    };

    let Some(me) = executable() else {
        return fail(EXIT_INTERNAL, "serve cannot find its own executable");
    };

    let listener = match listen(&socket) {
        Ok(listener) => listener,
        Err(e) => return fail(EXIT_USAGE, &format!("{}: {e}", socket.display())),
    };

    let env = environment();
    let mut session = Session::default();

    eprintln!("fmtkit: serving {} on {}", root.display(), socket.display());

    for stream in listener.incoming() {
        let Ok(stream) = stream else {
            continue;
        };

        let started = Instant::now();
        let changed = executable().as_ref() != Some(&me);
        let answered = answer(&mut session, &stream, &me.path, &env, changed);

        drop(stream);

        match answered {
            Ok(code) => eprintln!("fmtkit: answered in {:.1} ms with exit {code}", started.elapsed().as_secs_f64() * 1000.0),
            Err(reason) => eprintln!("fmtkit: refused a call: {reason}"),
        }

        session.persist();

        if changed {
            eprintln!("fmtkit: the fmtkit executable changed; stopping");

            break;
        }
    }

    let _ = fs::remove_file(&socket);

    ExitCode::SUCCESS
}

/// Hand `call` to the server for its repository; the exit code, after
/// printing what the server answered. `None` when no server is listening or
/// the server refused, and the caller should run `call` itself.
pub(crate) fn ask(call: &Call) -> Option<u8> {
    let stream = UnixStream::connect(fmtkit_cache::socket(&call.root)?).ok()?;
    let request = Request { proto: PROTO, version: VERSION.to_owned(), exe: executable().map(|exe| exe.path), env: environment(), call };

    write(&stream, &request).ok()?;

    let bar = call.interactive.then(progress_bar);
    let reply = loop {
        match read::<Reply>(&stream) {
            Ok(Reply::Progress { done, total }) => {
                if let Some(bar) = &bar {
                    bar(done, total);
                }
            }
            Ok(Reply::Done { code, stdout, stderr }) => break Some((code, stdout, stderr)),
            Ok(Reply::Refused(_)) | Err(_) => break None,
        }
    };

    let Some((code, stdout, stderr)) = reply else {
        if let Some(bar) = &bar {
            bar(usize::MAX, 0);
        }

        return None;
    };

    let mut out = io::stdout().lock();

    if let Err(e) = out.write_all(&stdout).and_then(|()| out.flush())
        && e.kind() != io::ErrorKind::BrokenPipe
    {
        eprintln!("fmtkit: write report: {e}");

        return Some(EXIT_INTERNAL);
    }

    let _ = io::stderr().lock().write_all(&stderr);

    Some(code)
}

/// Bind `socket`, replacing one that nothing answers on. The socket is bound
/// in a directory only its owner can enter, made private there, and then
/// moved into place, so no one else can connect to it in between.
fn listen(socket: &Path) -> io::Result<UnixListener> {
    if let Some(dir) = socket.parent() {
        fs::create_dir_all(dir)?;
    }

    if UnixStream::connect(socket).is_ok() {
        return Err(io::Error::new(io::ErrorKind::AddrInUse, "another fmtkit serve is running for this repository"));
    }

    match fs::remove_file(socket) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
        _ => {}
    }

    let staging = socket.with_extension(format!("{}.d", std::process::id()));
    let staged = staging.join("s");

    fs::DirBuilder::new().mode(0o700).create(&staging)?;

    let bound = UnixListener::bind(&staged)
        .and_then(|listener| fs::set_permissions(&staged, Permissions::from_mode(0o600)).map(|()| listener))
        .and_then(|listener| fs::rename(&staged, socket).map(|()| listener));

    let _ = fs::remove_file(&staged);
    let _ = fs::remove_dir(&staging);

    bound
}

/// Run one caller's request in `session`, or refuse it: the exit code it
/// was answered with, or why it was refused.
fn answer(session: &mut Session, stream: &UnixStream, exe: &Path, env: &[(OsString, OsString)], stopping: bool) -> Result<u8, &'static str> {
    let _ = stream.set_read_timeout(Some(READ_TIMEOUT));
    let _ = stream.set_write_timeout(Some(WRITE_TIMEOUT));

    let refusal = match read::<Request<Call>>(stream) {
        Err(_) => Err("the request could not be read"),
        Ok(_) if stopping => Err("the server's executable changed"),
        Ok(request) if request.proto != PROTO || request.version != VERSION => Err("the server runs another fmtkit version"),
        Ok(request) if request.exe.as_deref() != Some(exe) => Err("the server runs another fmtkit executable"),
        Ok(request) if request.env != env => Err("the server runs with another environment"),
        Ok(request) => Ok(request.call),
    };

    let call = match refusal {
        Ok(call) => call,
        Err(reason) => {
            let _ = write(stream, &Reply::Refused(reason.to_owned()));

            return Err(reason);
        }
    };

    let progress = match stream.try_clone() {
        Ok(sink) if call.interactive => forward(sink),
        _ => Progress::default(),
    };

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = execute(session, &call, &progress, &mut stdout, &mut stderr);

    let _ = write(stream, &Reply::Done { code, stdout, stderr });

    Ok(code)
}

/// Progress that is sent to the caller, at most once per percent.
fn forward(sink: UnixStream) -> Progress {
    let sink = Mutex::new(sink);
    let last = AtomicUsize::new(usize::MAX);

    Progress::new(move |done, total| {
        if done != usize::MAX && (total == 0 || last.swap(done * 100 / total, Ordering::Relaxed) == done * 100 / total) {
            return;
        }

        let _ = write(&sink.lock().unwrap_or_else(PoisonError::into_inner), &Reply::Progress { done, total });
    })
}

/// This process's executable, if it is still on disk.
fn executable() -> Option<Executable> {
    let path = std::env::current_exe().ok()?.canonicalize().ok()?;
    let meta = fs::metadata(&path).ok()?;

    Some(Executable { path, len: meta.len(), modified: meta.modified().ok() })
}

/// The variables a run reads, sorted by name.
fn environment() -> Vec<(OsString, OsString)> {
    let mut vars: Vec<(OsString, OsString)> = std::env::vars_os()
        .filter(|(name, _)| {
            let name = name.to_string_lossy();

            PREFIXES.iter().any(|prefix| name.starts_with(prefix)) || NAMES.contains(&name.as_ref())
        })
        .collect();

    vars.sort();
    vars
}

fn write<T: Serialize>(mut stream: &UnixStream, value: &T) -> io::Result<()> {
    let mut frame = postcard::to_extend(value, vec![0; 4]).map_err(io::Error::other)?;
    let len = u32::try_from(frame.len() - 4).map_err(io::Error::other)?;

    frame[..4].copy_from_slice(&len.to_le_bytes());
    stream.write_all(&frame)
}

fn read<T: DeserializeOwned>(mut stream: &UnixStream) -> io::Result<T> {
    let mut len = [0; 4];

    stream.read_exact(&mut len)?;

    let len = usize::try_from(u32::from_le_bytes(len)).map_err(io::Error::other)?;

    if len > MAX_FRAME {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "frame too large"));
    }

    let mut frame = vec![0; len];

    stream.read_exact(&mut frame)?;
    postcard::from_bytes(&frame).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}
