//! The helper process: spawn, handshake, request multiplexing, and failure.
//!
//! Three threads serve one helper. A writer drains an unbounded queue of
//! encoded frames into the helper's stdin, so [`Helper::submit`] never blocks
//! on the pipe. A reader decodes replies from stdout and hands each to the
//! [`Ticket`] registered under its id. A third thread keeps the tail of stderr
//! for the crash message. Once the helper dies or misbehaves, every pending and
//! later request fails with the same error.

use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender};
use fmtkit_core::{Diagnostic, Severity, VERSION};
use rustc_hash::FxHashMap;

use crate::proto::{self, Frame, Hello, MAX_FRAME, Message, PROTOCOL_VERSION, ReadError, WireReply};
use crate::{GoError, Reply, Request, locate};

/// The version an unstamped helper build reports.
pub const DEV_VERSION: &str = "dev";

/// How long the helper may take to answer the handshake.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(30);

/// How much of the helper's stderr a crash message quotes.
const STDERR_TAIL: usize = 4 << 10;

const PIPE_BUFFER: usize = 64 << 10;

type Answer = Result<Reply, GoError>;

/// A running `fmtkit-go-helper`. Share it by reference across threads.
pub struct Helper {
    shared: Arc<Shared>,
    outbox: Option<Sender<Vec<u8>>>,
    writer: Option<JoinHandle<()>>,
    reader: Option<JoinHandle<()>>,
    pid: u32,
    path: PathBuf,
    version: String,
    finished: bool,
}

/// The pending answer to one [`Helper::submit`].
#[must_use = "a ticket does nothing until it is waited on"]
pub struct Ticket {
    answer: Receiver<Answer>,
}

struct Shared {
    state: Mutex<State>,
    child: Mutex<Child>,
    stderr: Mutex<Vec<u8>>,
    /// Set once fmtkit is stopping the helper on purpose.
    closing: AtomicBool,
}

#[derive(Default)]
struct State {
    next_id: u32,
    pending: FxHashMap<u32, Pending>,
    failure: Option<Failure>,
}

struct Pending {
    rel: String,
    answer: Sender<Answer>,
}

/// Why the helper can take no more requests.
#[derive(Debug, Clone)]
enum Failure {
    Crashed(String),
    Protocol(String),
    /// Stopped by [`Helper::shutdown`]; not an error by itself.
    Closed,
}

impl Failure {
    fn error(&self) -> GoError {
        match self {
            Self::Crashed(message) => GoError::Crashed(message.clone()),
            Self::Protocol(message) => GoError::Protocol(message.clone()),
            Self::Closed => GoError::Crashed("the helper was shut down".into()),
        }
    }
}

impl Helper {
    /// Locate, start, and handshake with the helper.
    ///
    /// The lookup order is `explicit`, `FMTKIT_GO_HELPER`, beside the running
    /// executable (as invoked, then with symlinks resolved), Homebrew's
    /// `../share/fmtkit/`, then `PATH`. See [`compatible`] for the version rule.
    pub fn spawn(explicit: Option<&Path>) -> Result<Self, GoError> {
        let path = locate::helper(explicit)?;

        Self::start(path)
    }

    fn start(path: PathBuf) -> Result<Self, GoError> {
        let mut child = Command::new(&path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| GoError::Spawn(format!("cannot start {}: {e}", path.display())))?;

        let (Some(stdin), Some(stdout), Some(stderr)) = (child.stdin.take(), child.stdout.take(), child.stderr.take()) else {
            let _ = child.kill();
            let _ = child.wait();

            return Err(GoError::Spawn("the helper's pipes were not opened".into()));
        };

        let pid = child.id();

        let shared = Arc::new(Shared {
            state: Mutex::new(State { next_id: 1, ..State::default() }),
            child: Mutex::new(child),
            stderr: Mutex::new(Vec::new()),
            closing: AtomicBool::new(false),
        });

        let stderr_done = match spawn_stderr(stderr, Arc::clone(&shared)) {
            Ok(done) => done,
            Err(e) => {
                let mut child = lock(&shared.child);
                let _ = child.kill();
                let _ = child.wait();

                return Err(e);
            }
        };

        let (outbox, inbox) = crossbeam_channel::unbounded();
        let (hello_tx, hello_rx) = crossbeam_channel::bounded(1);
        let mut hello = Vec::new();

        proto::encode(&Frame { id: 0, message: Message::Hello(Hello { proto: PROTOCOL_VERSION, version: VERSION.into() }) }, &mut hello);

        let _ = outbox.send(hello);

        let mut helper = Self { shared, outbox: Some(outbox), writer: None, reader: None, pid, path, version: String::new(), finished: false };

        helper.writer = Some(spawn_thread("fmtkit-go-writer", {
            let shared = Arc::clone(&helper.shared);

            move || write_loop(&stdin, &inbox, &shared)
        })?);

        helper.reader = Some(spawn_thread("fmtkit-go-reader", {
            let shared = Arc::clone(&helper.shared);

            move || read_loop(stdout, &shared, &hello_tx, &stderr_done)
        })?);

        let hello = match hello_rx.recv_timeout(HANDSHAKE_TIMEOUT) {
            Ok(Ok(hello)) => hello,
            Ok(Err(e)) => return Err(e),
            Err(_) => return Err(GoError::Spawn(format!("{} did not answer the handshake within {HANDSHAKE_TIMEOUT:?}", helper.path.display()))),
        };

        if !compatible(&hello, cfg!(debug_assertions)) {
            return Err(GoError::Version {
                found: format!("{} (fmtkit-go-helper {})", hello.proto, hello.version),
                expected: format!("{PROTOCOL_VERSION} (fmtkit-go-helper {VERSION})"),
                version: VERSION.into(),
            });
        }

        helper.version = hello.version;

        Ok(helper)
    }

    /// Queue a request. Safe to call from any thread; never waits for the
    /// helper. Fails at once when the helper has already died, so callers see
    /// the crash at the first opportunity; a request queued before the crash
    /// fails through its ticket instead.
    pub fn submit(&self, request: Request) -> Result<Ticket, GoError> {
        let (answer, ticket) = crossbeam_channel::bounded(1);

        if proto::request_frame_len(&request) > MAX_FRAME as usize {
            let error = format!("{} bytes is too large for the go helper", request.source.len());
            let _ = answer.send(Ok(Reply { output: request.source, error: Some(error), ..Reply::default() }));

            return Ok(Ticket { answer: ticket });
        }

        let id = {
            let mut state = lock(&self.shared.state);

            if let Some(failure) = &state.failure {
                return Err(failure.error());
            }

            let id = state.allocate_id();

            state.pending.insert(id, Pending { rel: request.rel.clone(), answer });

            id
        };

        let mut frame = Vec::with_capacity(proto::request_frame_len(&request));

        proto::encode(&Frame { id, message: Message::Process(request) }, &mut frame);

        // A closed queue means the writer saw the helper die; the reader fails
        // this request along with the others.
        if let Some(outbox) = &self.outbox {
            let _ = outbox.send(frame);
        }

        Ok(Ticket { answer: ticket })
    }

    /// Ask the helper to exit and wait for it. Requests already submitted are
    /// answered first.
    pub fn shutdown(mut self) -> Result<(), GoError> {
        self.finished = true;
        self.shared.closing.store(true, Ordering::SeqCst);

        if let Some(outbox) = self.outbox.take() {
            let mut frame = Vec::new();

            proto::encode(&Frame { id: 0, message: Message::Shutdown }, &mut frame);

            let _ = outbox.send(frame);
        }

        // The reader ends when the helper closes stdout on exit; only then is
        // the child waited on, so the reader never stalls behind that lock.
        join(self.writer.take());
        join(self.reader.take());

        let status = lock(&self.shared.child).wait();

        if let Some(failure @ (Failure::Crashed(_) | Failure::Protocol(_))) = &lock(&self.shared.state).failure {
            return Err(failure.error());
        }

        match status {
            Ok(status) if status.success() => Ok(()),
            Ok(status) => Err(GoError::Crashed(with_stderr(&format!("exited with {status} during shutdown"), &self.shared))),
            Err(e) => Err(GoError::Crashed(format!("waiting for the helper: {e}"))),
        }
    }

    /// The helper's process id.
    pub fn id(&self) -> u32 {
        self.pid
    }

    /// The executable that was started.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The fmtkit version the helper reported in the handshake.
    pub fn version(&self) -> &str {
        &self.version
    }
}

impl Drop for Helper {
    /// A helper dropped without [`Helper::shutdown`] is killed; its pending
    /// tickets fail.
    fn drop(&mut self) {
        if self.finished {
            return;
        }

        self.shared.closing.store(true, Ordering::SeqCst);
        self.shared.fail(Failure::Crashed("the helper was stopped before it answered".into()));

        {
            let mut child = lock(&self.shared.child);
            let _ = child.kill();
            let _ = child.wait();
        }

        self.outbox = None;

        join(self.writer.take());
        join(self.reader.take());
    }
}

impl Ticket {
    /// Block until the helper answers this request or fails.
    pub fn wait(self) -> Result<Reply, GoError> {
        self.answer.recv().unwrap_or_else(|_| Err(GoError::Crashed("the helper client went away".into())))
    }
}

/// Whether fmtkit may talk to a helper that sent `hello`. The protocol versions
/// must be equal. The fmtkit versions must be equal too, unless either side is
/// a development build: a helper reporting [`DEV_VERSION`] (built without
/// `-ldflags "-X main.version=..."`), or a debug build of fmtkit (`dev_build`).
pub fn compatible(hello: &Hello, dev_build: bool) -> bool {
    hello.proto == PROTOCOL_VERSION && (hello.version == VERSION || hello.version == DEV_VERSION || dev_build)
}

impl State {
    /// The next free id, skipping 0 (reserved for Hello and Shutdown) and any id
    /// still pending after the counter wraps.
    fn allocate_id(&mut self) -> u32 {
        loop {
            let id = self.next_id;

            self.next_id = self.next_id.wrapping_add(1);

            if id != 0 && !self.pending.contains_key(&id) {
                return id;
            }
        }
    }
}

impl Shared {
    /// Record the first failure and fail every pending request with it.
    fn fail(&self, failure: Failure) {
        let (failure, pending): (Failure, Vec<Pending>) = {
            let mut state = lock(&self.state);
            let failure = state.failure.get_or_insert(failure).clone();

            (failure, state.pending.drain().map(|(_, pending)| pending).collect())
        };

        for pending in pending {
            let _ = pending.answer.send(Err(failure.error()));
        }
    }

    fn deliver(&self, id: u32, reply: WireReply) -> Result<(), Failure> {
        let Some(pending) = lock(&self.state).pending.remove(&id) else {
            return Err(Failure::Protocol(format!("reply for unknown request id {id}")));
        };

        let _ = pending.answer.send(Ok(into_reply(&pending.rel, reply)));

        Ok(())
    }

    fn kill(&self) {
        let _ = lock(&self.child).kill();
    }

    /// Describe how the helper ended: wait briefly for it to exit by itself,
    /// then kill it, so the message carries its real exit status when it has one.
    fn exit_message(&self, stderr_done: &Receiver<()>) -> String {
        let mut status = None;

        for _ in 0..25 {
            match lock(&self.child).try_wait() {
                Ok(Some(exited)) => {
                    status = Some(exited);

                    break;
                }
                Ok(None) => thread::sleep(Duration::from_millis(10)),
                Err(_) => break,
            }
        }

        let status = status.or_else(|| {
            let mut child = lock(&self.child);
            let _ = child.kill();

            child.wait().ok()
        });

        let _ = stderr_done.recv_timeout(Duration::from_millis(500));
        let head = status.map_or_else(|| "exited".to_owned(), |status| format!("exited with {status}"));

        with_stderr(&head, self)
    }
}

fn into_reply(rel: &str, wire: WireReply) -> Reply {
    let violations = wire
        .violations
        .into_iter()
        .map(|v| Diagnostic { rule: v.rule, file: rel.to_owned(), line: v.line, column: v.column, message: v.message, severity: Severity::Error })
        .collect();

    Reply { output: wire.output, applied: wire.applied, violations, complexity: wire.complexity, error: (!wire.error.is_empty()).then_some(wire.error) }
}

fn write_loop(stdin: &ChildStdin, inbox: &Receiver<Vec<u8>>, shared: &Shared) {
    let mut writer = BufWriter::with_capacity(PIPE_BUFFER, stdin);

    while let Ok(frame) = inbox.recv() {
        let flushed = writer.write_all(&frame).and_then(|()| if inbox.is_empty() { writer.flush() } else { Ok(()) });

        if flushed.is_err() {
            // The helper stopped reading. Killing it makes the reader see the
            // end of stdout and report the crash with the exit status.
            if !shared.closing.load(Ordering::SeqCst) {
                shared.kill();
            }

            return;
        }
    }

    let _ = writer.flush();
}

fn read_loop(stdout: ChildStdout, shared: &Shared, hello: &Sender<Result<Hello, GoError>>, stderr_done: &Receiver<()>) {
    let mut reader = BufReader::with_capacity(PIPE_BUFFER, stdout);

    let failure = match proto::read_frame(&mut reader) {
        Ok(Some(Frame { message: Message::Hello(h), .. })) => {
            let _ = hello.try_send(Ok(h));

            serve_replies(&mut reader, shared, stderr_done)
        }
        Ok(Some(frame)) => Failure::Protocol(format!("expected hello, got a {:?} frame", frame.message.kind())),
        other => ended(other.map(|_| ()), shared, stderr_done),
    };

    if matches!(failure, Failure::Protocol(_)) {
        shared.kill();
    }

    // Only reaches spawn when the handshake itself failed; never blocks.
    let _ = hello.try_send(Err(failure.error()));

    shared.fail(failure);
}

/// Dispatch replies until the stream ends or breaks; returns why it stopped.
fn serve_replies(reader: &mut impl Read, shared: &Shared, stderr_done: &Receiver<()>) -> Failure {
    loop {
        match proto::read_frame(reader) {
            Ok(Some(Frame { id, message: Message::Reply(reply) })) => {
                if let Err(failure) = shared.deliver(id, reply) {
                    return failure;
                }
            }
            Ok(Some(frame)) => return Failure::Protocol(format!("unexpected {:?} frame from the helper", frame.message.kind())),
            other => return ended(other.map(|_| ()), shared, stderr_done),
        }
    }
}

/// Classify the end of the reply stream.
fn ended(result: Result<(), ReadError>, shared: &Shared, stderr_done: &Receiver<()>) -> Failure {
    match result {
        Err(ReadError::Malformed(e)) => Failure::Protocol(e.to_string()),
        _ if shared.closing.load(Ordering::SeqCst) => Failure::Closed,
        Ok(()) => Failure::Crashed(shared.exit_message(stderr_done)),
        Err(ReadError::Io(e)) => Failure::Crashed(format!("reading replies: {e}; {}", shared.exit_message(stderr_done))),
    }
}

/// Keep the last [`STDERR_TAIL`] bytes of the helper's stderr. The returned
/// channel disconnects when stderr closes.
fn spawn_stderr(mut stderr: ChildStderr, shared: Arc<Shared>) -> Result<Receiver<()>, GoError> {
    let (done_tx, done_rx) = crossbeam_channel::bounded::<()>(0);

    spawn_thread("fmtkit-go-stderr", move || {
        let _done = done_tx;
        let mut chunk = [0; 4096];

        while let Ok(n @ 1..) = stderr.read(&mut chunk) {
            let mut tail = lock(&shared.stderr);

            tail.extend_from_slice(&chunk[..n]);

            if tail.len() > STDERR_TAIL {
                let excess = tail.len() - STDERR_TAIL;

                tail.drain(..excess);
            }
        }
    })?;

    Ok(done_rx)
}

fn with_stderr(head: &str, shared: &Shared) -> String {
    let tail = lock(&shared.stderr);
    let text = String::from_utf8_lossy(&tail);
    let text = text.trim();

    if text.is_empty() { head.to_owned() } else { format!("{head}; stderr: {text}") }
}

fn spawn_thread(name: &str, body: impl FnOnce() + Send + 'static) -> Result<JoinHandle<()>, GoError> {
    thread::Builder::new().name(name.into()).spawn(body).map_err(|e| GoError::Spawn(format!("cannot start the {name} thread: {e}")))
}

fn join(handle: Option<JoinHandle<()>>) {
    if let Some(handle) = handle {
        let _ = handle.join();
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::{DEV_VERSION, Failure, State, compatible, into_reply};
    use crate::proto::{Hello, PROTOCOL_VERSION, WireReply, WireViolation};
    use fmtkit_core::VERSION;

    fn hello(proto: u32, version: &str) -> Hello {
        Hello { proto, version: version.into() }
    }

    #[test]
    fn requires_the_same_protocol_and_release() {
        assert!(compatible(&hello(PROTOCOL_VERSION, VERSION), false));
        assert!(!compatible(&hello(PROTOCOL_VERSION, "1.9.0"), false));
        assert!(!compatible(&hello(PROTOCOL_VERSION + 1, VERSION), false));
    }

    #[test]
    fn lets_development_builds_skip_the_release_check() {
        assert!(compatible(&hello(PROTOCOL_VERSION, DEV_VERSION), false));
        assert!(compatible(&hello(PROTOCOL_VERSION, "1.9.0"), true));
        assert!(!compatible(&hello(PROTOCOL_VERSION + 1, DEV_VERSION), true));
    }

    #[test]
    fn allocates_ids_from_one_and_skips_zero_and_pending_ids() {
        let mut state = State { next_id: u32::MAX, ..State::default() };

        assert_eq!(state.allocate_id(), u32::MAX);

        let (answer, _ticket) = crossbeam_channel::bounded(1);

        state.pending.insert(1, super::Pending { rel: String::new(), answer });

        assert_eq!(state.allocate_id(), 2);
    }

    #[test]
    fn ties_violations_to_the_requested_file() {
        let wire = WireReply {
            output: b"x".to_vec(),
            violations: vec![WireViolation { rule: "spacing".into(), line: 4, column: 0, message: "m".into() }],
            ..WireReply::default()
        };

        let reply = into_reply("pkg/a.go", wire);

        assert_eq!(reply.error, None);
        assert_eq!(reply.violations[0].file, "pkg/a.go");
        assert_eq!(reply.violations[0].line, 4);

        let failed = into_reply("pkg/a.go", WireReply { error: "gofmt: boom".into(), ..WireReply::default() });

        assert_eq!(failed.error.as_deref(), Some("gofmt: boom"));
    }

    #[test]
    fn maps_failures_to_errors() {
        assert!(matches!(Failure::Crashed("x".into()).error(), crate::GoError::Crashed(m) if m == "x"));
        assert!(matches!(Failure::Protocol("y".into()).error(), crate::GoError::Protocol(m) if m == "y"));
        assert!(matches!(Failure::Closed.error(), crate::GoError::Crashed(_)));
    }
}
