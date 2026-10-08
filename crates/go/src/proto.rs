//! The wire format shared with `fmtkit-go-helper`, specified in
//! `go/helper/proto/PROTOCOL.md`.
//!
//! A frame is `u32 length | u8 kind | u32 id | payload`, little-endian, where
//! `length` counts everything after itself. Payloads are fixed field sequences
//! of `u32`s, `u8` bools, and `u32`-length-prefixed bytes and lists. Decoding
//! never panics and never allocates more than the input justifies.

use std::fmt;
use std::io::{self, Read};
use std::path::PathBuf;

use fmtkit_core::ComplexityScore;

use crate::{Request, Steps};

/// The protocol version exchanged in the handshake.
pub const PROTOCOL_VERSION: u32 = 1;

/// The largest accepted value of a frame's length field (256 MiB).
pub const MAX_FRAME: u32 = 256 << 20;

/// The kind byte and the id that follow the length field.
const HEADER_LEN: u32 = 5;

/// How much of a frame [`read_frame`] reserves before the bytes arrive.
const INITIAL_RESERVE: u32 = 64 << 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Hello = 1,
    Process = 2,
    Reply = 3,
    Shutdown = 4,
}

impl Kind {
    fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            1 => Some(Self::Hello),
            2 => Some(Self::Process),
            3 => Some(Self::Reply),
            4 => Some(Self::Shutdown),
            _ => None,
        }
    }
}

/// The first frame in each direction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hello {
    pub proto: u32,
    /// The fmtkit release, or `dev` for an unstamped helper build.
    pub version: String,
}

/// One spacing finding as the helper sends it, before it is tied to a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireViolation {
    pub rule: String,
    pub line: u32,
    pub column: u32,
    pub message: String,
}

/// A `Reply` payload as it travels.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WireReply {
    /// Empty when the file was processed.
    pub error: String,
    pub output: Vec<u8>,
    pub applied: Vec<String>,
    pub violations: Vec<WireViolation>,
    pub complexity: Vec<ComplexityScore>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    Hello(Hello),
    Process(Request),
    Reply(WireReply),
    Shutdown,
}

impl Message {
    pub fn kind(&self) -> Kind {
        match self {
            Self::Hello(_) => Kind::Hello,
            Self::Process(_) => Kind::Process,
            Self::Reply(_) => Kind::Reply,
            Self::Shutdown => Kind::Shutdown,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub id: u32,
    pub message: Message,
}

/// Why bytes could not be decoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodeError(String);

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for DecodeError {}

#[derive(Debug, thiserror::Error)]
pub enum ReadError {
    #[error("read: {0}")]
    Io(#[from] io::Error),
    #[error(transparent)]
    Malformed(#[from] DecodeError),
}

fn malformed(message: impl Into<String>) -> DecodeError {
    DecodeError(message.into())
}

/// Append one encoded frame to `out`.
pub fn encode(frame: &Frame, out: &mut Vec<u8>) {
    let start = out.len();

    out.extend_from_slice(&[0; 4]);
    out.push(frame.message.kind() as u8);
    out.extend_from_slice(&frame.id.to_le_bytes());

    match &frame.message {
        Message::Hello(hello) => {
            put_u32(out, hello.proto);
            put_bytes(out, hello.version.as_bytes());
        }
        Message::Process(request) => encode_request(request, out),
        Message::Reply(reply) => encode_reply(reply, out),
        Message::Shutdown => {}
    }

    let length = len_u32(out.len() - start - 4);

    out[start..start + 4].copy_from_slice(&length.to_le_bytes());
}

/// The encoded size of a frame carrying `request`, without encoding it.
pub fn request_frame_len(request: &Request) -> usize {
    4 + HEADER_LEN as usize + 3 * 4 + request.rel.len() + abs_bytes(&request.abs).len() + request.source.len() + 5
}

fn encode_request(request: &Request, out: &mut Vec<u8>) {
    let Steps { spacing, gofmt, goimports, resolve_imports, complexity } = request.steps;

    put_bytes(out, request.rel.as_bytes());
    put_bytes(out, &abs_bytes(&request.abs));
    put_bytes(out, &request.source);

    for flag in [spacing, gofmt, goimports, resolve_imports, complexity] {
        out.push(u8::from(flag));
    }
}

fn encode_reply(reply: &WireReply, out: &mut Vec<u8>) {
    put_bytes(out, reply.error.as_bytes());
    put_bytes(out, &reply.output);
    put_u32(out, len_u32(reply.applied.len()));

    for name in &reply.applied {
        put_bytes(out, name.as_bytes());
    }

    put_u32(out, len_u32(reply.violations.len()));

    for v in &reply.violations {
        put_bytes(out, v.rule.as_bytes());
        put_u32(out, v.line);
        put_u32(out, v.column);
        put_bytes(out, v.message.as_bytes());
    }

    put_u32(out, len_u32(reply.complexity.len()));

    for s in &reply.complexity {
        put_bytes(out, s.key.as_bytes());
        put_bytes(out, s.name.as_bytes());
        put_u32(out, s.line);
        put_u32(out, s.cyclomatic);
        put_u32(out, s.cognitive);
    }
}

#[cfg(unix)]
fn abs_bytes(path: &std::path::Path) -> std::borrow::Cow<'_, [u8]> {
    use std::os::unix::ffi::OsStrExt;

    std::borrow::Cow::Borrowed(path.as_os_str().as_bytes())
}

#[cfg(not(unix))]
fn abs_bytes(path: &std::path::Path) -> std::borrow::Cow<'_, [u8]> {
    match path.to_string_lossy() {
        std::borrow::Cow::Borrowed(text) => std::borrow::Cow::Borrowed(text.as_bytes()),
        std::borrow::Cow::Owned(text) => std::borrow::Cow::Owned(text.into_bytes()),
    }
}

#[cfg(unix)]
fn path_from_bytes(bytes: &[u8]) -> PathBuf {
    use std::os::unix::ffi::OsStrExt;

    PathBuf::from(std::ffi::OsStr::from_bytes(bytes))
}

#[cfg(not(unix))]
fn path_from_bytes(bytes: &[u8]) -> PathBuf {
    PathBuf::from(String::from_utf8_lossy(bytes).into_owned())
}

fn put_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    put_u32(out, len_u32(bytes.len()));
    out.extend_from_slice(bytes);
}

/// Lengths past `u32::MAX` cannot be framed; callers reject such inputs first
/// (see [`request_frame_len`]), so saturating here only guards the arithmetic.
fn len_u32(len: usize) -> u32 {
    u32::try_from(len).unwrap_or(u32::MAX)
}

/// Decode the first frame in `buf`, returning it and the number of bytes it
/// used. Bytes after the frame are left alone; a frame that is not complete
/// yet is an error.
pub fn decode(buf: &[u8]) -> Result<(Frame, usize), DecodeError> {
    let Some((prefix, rest)) = buf.split_first_chunk::<4>() else {
        return Err(malformed(format!("truncated length: {} bytes", buf.len())));
    };

    let length = check_length(u32::from_le_bytes(*prefix))?;
    let Some(body) = rest.get(..length as usize) else {
        return Err(malformed(format!("truncated frame: {} of {length} bytes", rest.len())));
    };

    Ok((decode_body(body)?, 4 + length as usize))
}

/// Read one frame. `Ok(None)` is a clean end of stream between frames.
pub fn read_frame(reader: &mut impl Read) -> Result<Option<Frame>, ReadError> {
    let mut prefix = [0; 4];
    let mut filled = 0;

    while filled < prefix.len() {
        match reader.read(&mut prefix[filled..]) {
            Ok(0) if filled == 0 => return Ok(None),
            Ok(0) => return Err(malformed(format!("truncated length: {filled} bytes")).into()),
            Ok(n) => filled += n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e.into()),
        }
    }

    let length = check_length(u32::from_le_bytes(prefix))?;
    let mut body = Vec::with_capacity(length.min(INITIAL_RESERVE) as usize);

    reader.by_ref().take(u64::from(length)).read_to_end(&mut body)?;

    if body.len() != length as usize {
        return Err(malformed(format!("truncated frame: {} of {length} bytes", body.len())).into());
    }

    Ok(Some(decode_body(&body)?))
}

fn check_length(length: u32) -> Result<u32, DecodeError> {
    if length < HEADER_LEN {
        return Err(malformed(format!("frame length {length} is shorter than its header")));
    }

    if length > MAX_FRAME {
        return Err(malformed(format!("frame length {length} exceeds {MAX_FRAME}")));
    }

    Ok(length)
}

/// Decode a frame without its length prefix.
fn decode_body(body: &[u8]) -> Result<Frame, DecodeError> {
    let mut cursor = Cursor { buf: body };
    let kind_byte = cursor.byte()?;
    let id = cursor.u32()?;
    let kind = Kind::from_byte(kind_byte).ok_or_else(|| malformed(format!("unknown frame kind {kind_byte}")))?;

    let message = match kind {
        Kind::Hello => Message::Hello(Hello { proto: cursor.u32()?, version: cursor.string()? }),
        Kind::Process => Message::Process(decode_request(&mut cursor)?),
        Kind::Reply => Message::Reply(decode_reply(&mut cursor)?),
        Kind::Shutdown => Message::Shutdown,
    };

    if !cursor.buf.is_empty() {
        return Err(malformed(format!("{} trailing bytes after a {kind:?} payload", cursor.buf.len())));
    }

    Ok(Frame { id, message })
}

fn decode_request(cursor: &mut Cursor<'_>) -> Result<Request, DecodeError> {
    let rel = cursor.string()?;
    let abs = path_from_bytes(cursor.bytes()?);
    let source = cursor.bytes()?.to_vec();

    let steps =
        Steps { spacing: cursor.bool()?, gofmt: cursor.bool()?, goimports: cursor.bool()?, resolve_imports: cursor.bool()?, complexity: cursor.bool()? };

    Ok(Request { rel, abs, source, steps })
}

fn decode_reply(cursor: &mut Cursor<'_>) -> Result<WireReply, DecodeError> {
    let error = cursor.string()?;
    let output = cursor.bytes()?.to_vec();
    let count = cursor.count(4)?;
    let mut applied = Vec::with_capacity(count);

    for _ in 0..count {
        applied.push(cursor.string()?);
    }

    let count = cursor.count(16)?;
    let mut violations = Vec::with_capacity(count);

    for _ in 0..count {
        violations.push(WireViolation { rule: cursor.string()?, line: cursor.u32()?, column: cursor.u32()?, message: cursor.string()? });
    }

    let count = cursor.count(20)?;
    let mut complexity = Vec::with_capacity(count);

    for _ in 0..count {
        complexity.push(ComplexityScore {
            key: cursor.string()?,
            name: cursor.string()?,
            line: cursor.u32()?,
            cyclomatic: cursor.u32()?,
            cognitive: cursor.u32()?,
        });
    }

    Ok(WireReply { error, output, applied, violations, complexity })
}

struct Cursor<'a> {
    buf: &'a [u8],
}

impl<'a> Cursor<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], DecodeError> {
        if n > self.buf.len() {
            return Err(malformed(format!("need {n} bytes, have {}", self.buf.len())));
        }

        let (head, tail) = self.buf.split_at(n);

        self.buf = tail;

        Ok(head)
    }

    fn byte(&mut self) -> Result<u8, DecodeError> {
        Ok(self.take(1)?[0])
    }

    fn u32(&mut self) -> Result<u32, DecodeError> {
        let Some((head, tail)) = self.buf.split_first_chunk::<4>() else {
            return Err(malformed(format!("need 4 bytes, have {}", self.buf.len())));
        };

        self.buf = tail;

        Ok(u32::from_le_bytes(*head))
    }

    fn bool(&mut self) -> Result<bool, DecodeError> {
        match self.byte()? {
            0 => Ok(false),
            1 => Ok(true),
            other => Err(malformed(format!("bool byte {other}"))),
        }
    }

    fn bytes(&mut self) -> Result<&'a [u8], DecodeError> {
        let n = self.u32()?;

        self.take(n as usize)
    }

    /// Text fields are UTF-8 by convention; anything else is decoded lossily.
    fn string(&mut self) -> Result<String, DecodeError> {
        Ok(String::from_utf8_lossy(self.bytes()?).into_owned())
    }

    /// A list length, rejected when the remaining bytes cannot hold that many
    /// elements of at least `min_size` bytes each.
    fn count(&mut self, min_size: usize) -> Result<usize, DecodeError> {
        let n = self.u32()? as usize;

        if n.saturating_mul(min_size) > self.buf.len() {
            return Err(malformed(format!("list of {n} elements does not fit in {} bytes", self.buf.len())));
        }

        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use fmtkit_core::ComplexityScore;
    use proptest::prelude::*;

    use super::{Frame, Hello, Message, WireReply, WireViolation, decode, encode, read_frame, request_frame_len};
    use crate::{Request, Steps};

    fn fixture(name: &str) -> Vec<u8> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../go/helper/proto/testdata").join(format!("{name}.bin"));

        std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
    }

    fn score(key: &str, name: &str, line: u32, cyclomatic: u32, cognitive: u32) -> ComplexityScore {
        ComplexityScore { key: key.into(), name: name.into(), line, cyclomatic, cognitive }
    }

    /// The values behind `go/helper/proto/testdata`, as `proto_test.go` spells them.
    fn golden() -> Vec<(&'static str, Frame)> {
        let request = Request {
            rel: "pkg/a.go".into(),
            abs: PathBuf::from("/repo/pkg/a.go"),
            source: b"package a\n".to_vec(),
            steps: Steps { spacing: true, gofmt: true, goimports: false, resolve_imports: false, complexity: true },
        };

        let reply = WireReply {
            error: String::new(),
            output: b"package a\n\nfunc f() {}\n".to_vec(),
            applied: vec!["spacing".into(), "gofmt".into()],
            violations: vec![
                WireViolation { rule: "spacing".into(), line: 3, column: 0, message: "missing blank line".into() },
                WireViolation { rule: "spacing".into(), line: 10, column: 2, message: "ünïcode ✓".into() },
            ],
            complexity: vec![score("pkg/a.go#f", "f", 3, 1, 0), score("pkg/a.go#(*T).M", "(*T).M", 5, 4, 6)],
        };

        let error = WireReply { error: "gofmt: 1:1: expected 'package', found 'EOF'".into(), ..WireReply::default() };

        vec![
            ("hello", Frame { id: 0, message: Message::Hello(Hello { proto: 1, version: "2.0.0".into() }) }),
            ("process", Frame { id: 7, message: Message::Process(request) }),
            ("reply", Frame { id: 7, message: Message::Reply(reply) }),
            ("reply_error", Frame { id: u32::MAX, message: Message::Reply(error) }),
            ("shutdown", Frame { id: 0, message: Message::Shutdown }),
        ]
    }

    #[test]
    fn encodes_the_golden_frames_byte_for_byte() {
        for (name, frame) in golden() {
            let mut encoded = Vec::new();

            encode(&frame, &mut encoded);

            assert_eq!(encoded, fixture(name), "{name}");
        }
    }

    #[test]
    fn decodes_the_golden_frames() {
        for (name, frame) in golden() {
            let bytes = fixture(name);

            assert_eq!(decode(&bytes), Ok((frame.clone(), bytes.len())), "{name}");
            assert_eq!(read_frame(&mut bytes.as_slice()).unwrap(), Some(frame), "{name}");
        }
    }

    #[test]
    fn predicts_the_request_frame_length() {
        for (_, frame) in golden() {
            if let Message::Process(request) = &frame.message {
                let mut encoded = Vec::new();

                encode(&frame, &mut encoded);

                assert_eq!(request_frame_len(request), encoded.len());
            }
        }
    }

    #[test]
    fn reads_frames_back_to_back_and_stops_cleanly() {
        let mut stream = Vec::new();

        for (_, frame) in golden() {
            encode(&frame, &mut stream);
        }

        let mut reader = stream.as_slice();
        let mut count = 0;

        while let Some(_frame) = read_frame(&mut reader).unwrap() {
            count += 1;
        }

        assert_eq!(count, golden().len());
    }

    #[test]
    fn rejects_malformed_frames() {
        let hello = fixture("hello");
        let mut trailing = hello.clone();

        trailing[0] += 1;
        trailing.push(0);

        let mut bad_bool = fixture("process");
        let last = bad_bool.len() - 1;

        bad_bool[last] = 2;

        let cases: Vec<(&str, Vec<u8>)> = vec![
            ("empty", vec![]),
            ("truncated length", vec![1, 0]),
            ("short header", vec![4, 0, 0, 0, 1, 0, 0, 0]),
            ("oversized", vec![0xFF, 0xFF, 0xFF, 0xFF, 1]),
            ("truncated payload", hello[..hello.len() - 1].to_vec()),
            ("unknown kind", vec![5, 0, 0, 0, 9, 0, 0, 0, 0]),
            ("trailing bytes", trailing),
            ("bad bool", bad_bool),
            ("huge list", [&[17, 0, 0, 0, 3, 1, 0, 0, 0][..], &[0; 8], &[0xFF; 4]].concat()),
            ("string past the end", vec![14, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 9, 0, 0, 0, b'x']),
        ];

        for (name, bytes) in cases {
            assert!(decode(&bytes).is_err(), "{name} decoded");

            let read = read_frame(&mut bytes.as_slice());

            assert!(read.is_err() || (bytes.is_empty() && matches!(read, Ok(None))), "{name} read: {read:?}");
        }
    }

    #[test]
    fn reports_a_truncated_stream_as_malformed_not_io() {
        let hello = fixture("hello");
        let err = read_frame(&mut &hello[..6]).unwrap_err();

        assert!(matches!(err, super::ReadError::Malformed(_)), "{err}");
    }

    fn arb_string() -> impl Strategy<Value = String> {
        prop::string::string_regex(".{0,12}").unwrap()
    }

    fn arb_reply() -> impl Strategy<Value = WireReply> {
        let violation =
            (arb_string(), any::<u32>(), any::<u32>(), arb_string()).prop_map(|(rule, line, column, message)| WireViolation { rule, line, column, message });
        let score = (arb_string(), arb_string(), any::<u32>(), any::<u32>(), any::<u32>())
            .prop_map(|(key, name, line, cyclomatic, cognitive)| ComplexityScore { key, name, line, cyclomatic, cognitive });

        (
            arb_string(),
            prop::collection::vec(any::<u8>(), 0..64),
            prop::collection::vec(arb_string(), 0..4),
            prop::collection::vec(violation, 0..4),
            prop::collection::vec(score, 0..4),
        )
            .prop_map(|(error, output, applied, violations, complexity)| WireReply { error, output, applied, violations, complexity })
    }

    fn arb_request() -> impl Strategy<Value = Request> {
        (arb_string(), "/[a-z/]{0,12}", prop::collection::vec(any::<u8>(), 0..64), any::<[bool; 5]>()).prop_map(|(rel, abs, source, flags)| Request {
            rel,
            abs: PathBuf::from(abs),
            source,
            steps: Steps { spacing: flags[0], gofmt: flags[1], goimports: flags[2], resolve_imports: flags[3], complexity: flags[4] },
        })
    }

    fn arb_frame() -> impl Strategy<Value = Frame> {
        let message = prop_oneof![
            (any::<u32>(), arb_string()).prop_map(|(proto, version)| Message::Hello(Hello { proto, version })),
            arb_request().prop_map(Message::Process),
            arb_reply().prop_map(Message::Reply),
            Just(Message::Shutdown),
        ];

        (any::<u32>(), message).prop_map(|(id, message)| Frame { id, message })
    }

    proptest! {
        #[test]
        fn round_trips(frame in arb_frame()) {
            let mut encoded = Vec::new();

            encode(&frame, &mut encoded);

            prop_assert_eq!(decode(&encoded), Ok((frame.clone(), encoded.len())));
            prop_assert_eq!(read_frame(&mut encoded.as_slice()).unwrap(), Some(frame));
        }

        #[test]
        fn never_panics_on_arbitrary_bytes(bytes in prop::collection::vec(any::<u8>(), 0..256)) {
            let _ = decode(&bytes);
            let _ = read_frame(&mut bytes.as_slice());
        }

        #[test]
        fn never_panics_on_corrupted_frames(frame in arb_frame(), flips in prop::collection::vec((any::<prop::sample::Index>(), any::<u8>()), 1..4)) {
            let mut encoded = Vec::new();

            encode(&frame, &mut encoded);

            for (index, byte) in flips {
                let at = index.index(encoded.len());

                encoded[at] = byte;
            }

            let _ = decode(&encoded);
            let _ = read_frame(&mut encoded.as_slice());
        }
    }
}
