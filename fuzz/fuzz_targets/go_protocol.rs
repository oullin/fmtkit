//! The Go helper wire codec over arbitrary bytes.
//!
//! `decode` and `read_frame` must fail cleanly, never panic, and never
//! allocate more than the input justifies; they must agree with each other;
//! and anything that decodes must survive an encode/decode round trip.

#![no_main]

use std::io::{self, Read};

use fmtkit_go::compatible;
use fmtkit_go::proto::{Frame, Message, decode, encode, read_frame};
use libfuzzer_sys::fuzz_target;

/// Hands out the stream in small pieces, with the odd `Interrupted`, so the
/// length-prefix loop in `read_frame` sees short reads.
struct Chunked<'a> {
    data: &'a [u8],
    chunk: usize,
    interrupt: bool,
    calls: usize,
}

impl Read for Chunked<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.calls += 1;

        if self.interrupt && self.calls.is_multiple_of(3) {
            return Err(io::ErrorKind::Interrupted.into());
        }

        let n = self.chunk.min(buf.len()).min(self.data.len());

        buf[..n].copy_from_slice(&self.data[..n]);
        self.data = &self.data[n..];

        Ok(n)
    }
}

fn check_round_trip(frame: &Frame) {
    if let Message::Hello(hello) = &frame.message {
        let _ = compatible(hello, false);
        let _ = compatible(hello, true);
    }

    let mut encoded = Vec::new();

    encode(frame, &mut encoded);
    assert_eq!(decode(&encoded), Ok((frame.clone(), encoded.len())), "a decoded frame does not round-trip");
}

fuzz_target!(|data: &[u8]| {
    let Some((&mode, stream)) = data.split_first() else {
        return;
    };

    let decoded = decode(stream);

    if let Ok((frame, used)) = &decoded {
        assert!(*used <= stream.len());
        check_round_trip(frame);
    }

    // The same bytes through the streaming reader give the same first frame.
    let mut reader = Chunked { data: stream, chunk: usize::from(mode % 8) + 1, interrupt: mode & 0x80 != 0, calls: 0 };

    match (read_frame(&mut reader), &decoded) {
        (Ok(Some(read)), Ok((frame, _))) => assert_eq!(&read, frame),
        (Ok(Some(read)), Err(e)) => panic!("read_frame accepted {read:?} that decode rejected: {e}"),
        (Ok(None), _) => assert!(stream.is_empty(), "read_frame saw a clean end of a non-empty stream"),
        (Err(e), Ok((frame, _))) => panic!("decode accepted {frame:?} that read_frame rejected: {e}"),
        (Err(_), Err(_)) => {}
    }

    // Keep reading frames back to back until the stream ends or breaks.
    let mut rest = stream;

    while let Ok((frame, used)) = decode(rest) {
        check_round_trip(&frame);
        rest = &rest[used..];
    }

    let mut reader = stream;

    while let Ok(Some(_)) = read_frame(&mut reader) {}
});
