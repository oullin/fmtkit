//! The headless protocol oxlint speaks to its type-aware backend, which
//! `tsrs --lint headless` serves: a version 2 JSON payload on stdin, then
//! length-prefixed JSON frames on stdout.

use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;

use serde_json::{Map, Value, json};

use super::{Finding, Request, TypeAwareError};

const FRAME_ERROR: u8 = 0;
const FRAME_DIAGNOSTIC: u8 = 1;

/// How much of tsrs's standard error a failure quotes.
const STDERR_TAIL: usize = 2000;

/// Run `request` with the `tsrs` at `tsrs`. Every finding comes back, in the
/// order tsrs reported it; nothing is filtered here.
pub fn run(tsrs: &Path, request: &Request<'_>) -> Result<Vec<Finding>, TypeAwareError> {
    let payload = payload(request).to_string();
    let spawn_error = |source| TypeAwareError::Spawn { path: tsrs.to_path_buf(), source };
    let mut child = Command::new(tsrs)
        .args(["--lint", "headless"])
        .current_dir(request.root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(spawn_error)?;

    let mut stdin = child.stdin.take().expect("stdin is piped");
    let mut stdout = child.stdout.take().expect("stdout is piped");
    let mut stderr = child.stderr.take().expect("stderr is piped");

    // stdin is written and stderr drained on their own threads, so a full pipe
    // on any side cannot stall the others.
    let (written, bytes, errors) = thread::scope(|s| {
        let writer = s.spawn(move || {
            let written = stdin.write_all(payload.as_bytes());

            drop(stdin);
            written
        });
        let drainer = s.spawn(move || {
            let mut text = Vec::new();
            let _ = stderr.read_to_end(&mut text);

            text
        });
        let mut bytes = Vec::new();
        let read = stdout.read_to_end(&mut bytes).map(|_| bytes);

        (writer.join().unwrap_or_else(|panic| std::panic::resume_unwind(panic)), read, drainer.join().unwrap_or_default())
    });

    let status = child.wait().map_err(spawn_error)?;
    let errors = tail(&String::from_utf8_lossy(&errors));
    let bytes = bytes.map_err(|e| TypeAwareError::Protocol { path: tsrs.to_path_buf(), message: e.to_string() })?;
    let frames = frames(&bytes).map_err(|message| TypeAwareError::Protocol { path: tsrs.to_path_buf(), message })?;

    if let Some(message) = frames.iter().find_map(Frame::error) {
        return Err(TypeAwareError::Backend { path: tsrs.to_path_buf(), message });
    }

    if !status.success() {
        return Err(TypeAwareError::Exit { path: tsrs.to_path_buf(), status, stderr: errors });
    }

    // tsrs read everything it needed before it stopped reading; a pipe it
    // closed early is reported through its exit status instead.
    if let Err(error) = written
        && error.kind() != std::io::ErrorKind::BrokenPipe
    {
        return Err(spawn_error(error));
    }

    Ok(frames.into_iter().filter(|frame| frame.kind == FRAME_DIAGNOSTIC).map(|frame| finding(&frame.body, request.root)).collect())
}

fn tail(text: &str) -> String {
    let text = text.trim();
    let start = text.len().saturating_sub(STDERR_TAIL);
    let start = (start..text.len()).find(|at| text.is_char_boundary(*at)).unwrap_or(text.len());

    text[start..].to_owned()
}

/// The version 2 payload: one config group per rule set, absolute paths.
pub(super) fn payload(request: &Request<'_>) -> Value {
    let absolute = |rel: &str| request.root.join(rel).to_string_lossy().into_owned();
    let configs: Vec<Value> = request
        .groups
        .iter()
        .map(|group| {
            let rules: Vec<Value> = group
                .rules
                .iter()
                .map(|rule| {
                    let mut entry = Map::from_iter([("name".to_owned(), json!(rule.name))]);

                    if let Some(options) = &rule.options {
                        entry.insert("options".into(), options.clone());
                    }

                    Value::Object(entry)
                })
                .collect();

            json!({ "file_paths": group.files.iter().map(|f| absolute(f)).collect::<Vec<_>>(), "rules": rules })
        })
        .collect();
    let overrides = if request.overrides.is_empty() {
        Value::Null
    } else {
        Value::Object(request.overrides.iter().map(|(rel, text)| (absolute(rel), json!(text))).collect())
    };

    json!({
        "version": 2,
        "configs": configs,
        "source_overrides": overrides,
        "report_syntactic": false,
        "report_semantic": false,
    })
}

/// `| size: u32 LE | type: u8 | JSON |`, as oxc_linter's tsgolint.rs reads it.
#[derive(Debug, PartialEq)]
pub(super) struct Frame {
    pub kind: u8,
    pub body: Value,
}

impl Frame {
    fn error(&self) -> Option<String> {
        (self.kind == FRAME_ERROR).then(|| self.body.get("error").and_then(Value::as_str).unwrap_or("unknown error").to_owned())
    }
}

pub(super) fn frames(mut bytes: &[u8]) -> Result<Vec<Frame>, String> {
    let mut out = Vec::new();

    while !bytes.is_empty() {
        let (Some(size), Some(&kind)) = (bytes.get(..4), bytes.get(4)) else {
            return Err("truncated frame header".into());
        };
        let size = u32::from_le_bytes(size.try_into().expect("four bytes")) as usize;
        let body = bytes.get(5..5 + size).ok_or("truncated frame")?;
        let body = serde_json::from_slice(body).map_err(|e| format!("frame body: {e}"))?;

        out.push(Frame { kind, body });
        bytes = &bytes[5 + size..];
    }

    Ok(out)
}

/// A diagnostic frame. Rule diagnostics (`kind` 0) name their rule; program
/// diagnostics (`kind` 1: tsconfig errors) do not. Ranges are UTF-8 byte
/// offsets, as oxc spans are; an unspanned range falls back to the first
/// labelled range, as oxlint does.
fn finding(body: &Value, root: &Path) -> Finding {
    let offset = |value: Option<&Value>| value.and_then(Value::as_u64).map_or(0, |p| u32::try_from(p).unwrap_or(u32::MAX));
    let range = |value: &Value| (offset(value.get("pos")), offset(value.get("end")));
    let mut span = body.get("range").map_or((0, 0), range);

    if span == (0, 0)
        && let Some(label) = body.pointer("/labeled_ranges/0/range")
    {
        span = range(label);
    }

    Finding {
        rule: body.get("rule").and_then(Value::as_str).filter(|_| body.get("kind").and_then(Value::as_u64) != Some(1)).map(str::to_owned),
        file: body.get("file_path").and_then(Value::as_str).map(|path| relative(root, path)).unwrap_or_default(),
        start: span.0,
        end: span.1,
        message: body.pointer("/message/description").and_then(Value::as_str).unwrap_or_default().to_owned(),
    }
}

/// A path tsrs reported, relative to the root it was given (or to that root
/// with symlinks resolved, should tsrs have resolved them).
fn relative(root: &Path, path: &str) -> String {
    let path = Path::new(path);
    let slashed = |path: &Path| path.to_string_lossy().replace('\\', "/");

    if let Ok(rel) = path.strip_prefix(root) {
        return slashed(rel);
    }

    match root.canonicalize() {
        Ok(canonical) => path.strip_prefix(&canonical).map_or_else(|_| slashed(path), slashed),
        Err(_) => slashed(path),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use serde_json::json;

    use super::{FRAME_DIAGNOSTIC, FRAME_ERROR, Frame, finding, frames, payload, tail};
    use crate::type_aware::{Group, Request, TsRule};

    fn frame(kind: u8, body: &serde_json::Value) -> Vec<u8> {
        let text = body.to_string();
        let mut bytes = u32::try_from(text.len()).unwrap().to_le_bytes().to_vec();

        bytes.push(kind);
        bytes.extend_from_slice(text.as_bytes());
        bytes
    }

    #[test]
    fn decodes_frames_in_order() {
        let mut bytes = frame(FRAME_DIAGNOSTIC, &json!({ "rule": "no-for-in-array" }));

        bytes.extend(frame(FRAME_ERROR, &json!({ "error": "boom" })));
        bytes.extend(frame(2, &json!({ "rules": [] })));

        let decoded = frames(&bytes).unwrap();

        assert_eq!(decoded.len(), 3);
        assert_eq!(decoded[0], Frame { kind: FRAME_DIAGNOSTIC, body: json!({ "rule": "no-for-in-array" }) });
        assert_eq!(decoded[1].error().as_deref(), Some("boom"));
        assert_eq!(decoded[2].error(), None);
        assert_eq!(frames(&[]).unwrap(), []);
    }

    #[test]
    fn rejects_truncated_and_malformed_frames() {
        let mut bytes = frame(FRAME_DIAGNOSTIC, &json!({}));

        bytes.pop();

        assert!(frames(&bytes).is_err());
        assert!(frames(&[1, 0]).is_err());

        let mut garbage = 3u32.to_le_bytes().to_vec();

        garbage.extend([FRAME_DIAGNOSTIC, b'{', b'{', b'{']);

        assert!(frames(&garbage).is_err());
    }

    #[test]
    fn builds_a_version_2_payload_per_rule_set() {
        let all = vec![
            TsRule { name: "await-thenable".into(), options: Some(json!({ "allowMixedPromiseArrays": true })) },
            TsRule { name: "no-floating-promises".into(), options: None },
            TsRule { name: "return-await".into(), options: Some(json!("error-handling-correctness-only")) },
        ];
        let groups = [
            Group { files: vec!["src/a.ts".into(), "src/b.tsx".into()], rules: all.clone() },
            Group { files: vec!["src/c.js".into()], rules: vec![all[0].clone()] },
        ];
        let request = Request { root: Path::new("/repo"), groups: &groups, overrides: &[] };
        let built = payload(&request);

        assert_eq!(built["version"], 2);
        assert_eq!(built["source_overrides"], serde_json::Value::Null);
        assert_eq!(built["report_syntactic"], false);
        assert_eq!(built["report_semantic"], false);
        assert_eq!(built["configs"][0]["file_paths"], json!(["/repo/src/a.ts", "/repo/src/b.tsx"]));
        assert_eq!(built["configs"][0]["rules"][0], json!({ "name": "await-thenable", "options": { "allowMixedPromiseArrays": true } }));
        assert_eq!(built["configs"][0]["rules"][1], json!({ "name": "no-floating-promises" }));
        assert_eq!(built["configs"][0]["rules"][2], json!({ "name": "return-await", "options": "error-handling-correctness-only" }));
        assert_eq!(built["configs"][1]["file_paths"], json!(["/repo/src/c.js"]));

        let overrides = [("src/a.ts".to_owned(), "let x = 1;".to_owned())];
        let request = Request { overrides: &overrides, ..request };

        assert_eq!(payload(&request)["source_overrides"], json!({ "/repo/src/a.ts": "let x = 1;" }));
    }

    #[test]
    fn maps_a_diagnostic_frame_to_a_finding() {
        let body = json!({
            "kind": 0,
            "rule": "no-floating-promises",
            "file_path": "/repo/src/a.ts",
            "range": { "pos": 12, "end": 20 },
            "message": { "id": "floating", "description": "Promises must be awaited.", "help": "Add await." },
        });
        let found = finding(&body, Path::new("/repo"));

        assert_eq!((found.rule.as_deref(), found.file.as_str(), found.start, found.end), (Some("no-floating-promises"), "src/a.ts", 12, 20));
        assert_eq!(found.message, "Promises must be awaited.");

        let unspanned = json!({ "kind": 0, "rule": "r", "file_path": "/repo/a.ts", "range": { "pos": 0, "end": 0 }, "labeled_ranges": [{ "range": { "pos": 4, "end": 9 } }] });

        assert_eq!((finding(&unspanned, Path::new("/repo")).start, finding(&unspanned, Path::new("/repo")).end), (4, 9));

        let program = json!({ "kind": 1, "file_path": "/repo/tsconfig.json", "message": { "description": "bad option" } });

        assert_eq!(finding(&program, Path::new("/repo")).rule, None);
    }

    #[test]
    fn quotes_the_end_of_standard_error() {
        assert_eq!(tail("  short\n"), "short");
        assert_eq!(tail(&"é".repeat(3000)).len(), 2000);
    }
}
