//! `fmtkit_ts::format_source` over arbitrary UTF-8, with the script language
//! picked by the first byte.
//!
//! A syntax error is fine; `TsError::Invariant` means a pipeline step broke a
//! valid program and is a bug, except when the step is oxfmt (see below).
//! Formatting the output again must change nothing and report no applied
//! steps.

#![no_main]

use fmtkit_config::TsFormat;
use fmtkit_core::Lang;
use fmtkit_ts::{TsError, format_source};
use libfuzzer_sys::fuzz_target;

const LANGS: [(Lang, &str); 8] = [
    (Lang::Ts, "ts"),
    (Lang::Tsx, "tsx"),
    (Lang::Mts, "mts"),
    (Lang::Cts, "cts"),
    (Lang::Js, "js"),
    (Lang::Jsx, "jsx"),
    (Lang::Mjs, "mjs"),
    (Lang::Cjs, "cjs"),
];

fuzz_target!(|data: &[u8]| {
    let Some((&selector, rest)) = data.split_first() else {
        return;
    };

    let Ok(source) = std::str::from_utf8(rest) else {
        return;
    };

    let (lang, ext) = LANGS[usize::from(selector % 8)];
    // Bit 3 turns a TypeScript dialect into a declaration file, which is
    // validated and scored but never rewritten.
    let declaration = selector & 0x08 != 0 && matches!(lang, Lang::Ts | Lang::Mts | Lang::Cts);
    let rel = if declaration { format!("fuzz.d.{ext}") } else { format!("fuzz.{ext}") };
    let score = selector & 0x10 != 0;
    let options = TsFormat::default();

    let once = match format_source(&rel, lang, source, &options, score) {
        Ok(once) => once,
        Err(TsError::Syntax { .. }) => return,
        // oxc_formatter itself sometimes prints a different program (it merges
        // line comments, adds parentheses that turn `a < b > c` into a generic
        // call or `await x` into a call of `await`). The invariant check refuses
        // that output and the file stays as written, which is the intended
        // behaviour; docs/known-issues.md lists each bug with its minimized
        // input. Every other step is ours, so its invariant is still a bug.
        Err(TsError::Invariant { step: "oxfmt", .. }) => return,
        Err(TsError::Invariant { step, message }) => panic!("{rel}: invariant broken by step {step}: {message}"),
    };

    let twice = match format_source(&rel, lang, &once.output, &options, score) {
        Ok(twice) => twice,
        Err(e) => panic!("{rel}: formatted output fails to format again: {e}\n--- output ---\n{}", once.output),
    };

    assert_eq!(twice.output, once.output, "{rel}: not idempotent");
    assert!(twice.applied.is_empty(), "{rel}: second run applied {:?}", twice.applied);
});
