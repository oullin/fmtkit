//! `fmtkit_hosts::format_host` over arbitrary UTF-8 as Vue, HTML, or Markdown
//! (picked by the first byte): errors, never panics, and is idempotent.

#![no_main]

use fmtkit_config::TsFormat;
use fmtkit_core::Lang;
use fmtkit_hosts::format_host;
use libfuzzer_sys::fuzz_target;

const LANGS: [(Lang, &str); 3] = [(Lang::Vue, "fuzz.vue"), (Lang::Html, "fuzz.html"), (Lang::Markdown, "fuzz.md")];

fuzz_target!(|data: &[u8]| {
    let Some((&selector, rest)) = data.split_first() else {
        return;
    };

    let Ok(source) = std::str::from_utf8(rest) else {
        return;
    };

    let (lang, rel) = LANGS[usize::from(selector) % LANGS.len()];
    let options = TsFormat::default();

    let Ok(once) = format_host(rel, lang, source, &options) else {
        return;
    };

    let twice = match format_host(rel, lang, &once.output, &options) {
        Ok(twice) => twice,
        Err(e) => panic!("{rel}: formatted output fails to format again: {e}\n--- output ---\n{}", once.output),
    };

    assert_eq!(twice.output, once.output, "{rel}: not idempotent");
});
