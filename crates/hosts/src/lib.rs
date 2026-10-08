//! Host documents: Vue single-file components and HTML through markup_fmt,
//! Markdown through oxc_formatter_markdown, and CSS inside them through
//! oxc_formatter_css. Embedded scripts go through [`fmtkit_ts::format_embedded`].
//!
//! The whole document is reformatted. Line endings are normalised to `\n`
//! and a non-empty document always ends with one. A document that changed is
//! formatted again until it stops changing (see [`format_host`]).
//!
//! Failure policy, carried over from v1 (where Vue and HTML were hard
//! validated and Markdown fences were best effort):
//! - a Vue or HTML document whose markup does not parse, or whose `<script>`
//!   block fails in the script pipeline, is an error and stays untouched;
//! - a Markdown fence whose code fails is left as written;
//! - a `<style>` block or style fence that oxc_formatter_css rejects is left
//!   as written;
//! - a Vue template expression that fails is left as written;
//! - Vue custom blocks, `<template lang="pug">` and scripts or styles in a
//!   language fmtkit does not format (CoffeeScript, Sass, Stylus) are kept
//!   verbatim apart from surrounding blank lines.

mod css;
mod embed;
mod markdown;
mod markup;
mod raw;

use std::borrow::Cow;

use fmtkit_config::TsFormat;
use fmtkit_core::{Lang, LineIndex};
use fmtkit_ts::TsError;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HostError {
    /// The document does not parse. `line` and `column` are 1-based.
    #[error("{line}:{column}: {message}")]
    Syntax { line: u32, column: u32, message: String },
    /// A Vue or HTML `<script>` block failed in the script pipeline. `line` is
    /// the 1-based line of the failure in the host document.
    #[error("embedded {lang:?} block at line {line}: {message}")]
    Embedded { lang: Lang, line: u32, message: String },
    /// Formatting did not reach a fixed point; `step` is `idempotency`.
    #[error("{step} failed: {message}")]
    Invariant { step: &'static str, message: String },
}

/// The most rounds that may follow a round that changed the document before
/// [`format_host`] gives up on reaching a fixed point.
const EXTRA_ROUNDS: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Formatted {
    pub output: String,
    /// The steps that changed the text: `embedded` (a script), `css` (a
    /// style), then the document step (`markup` or `markdown`) whenever the
    /// document changed at all.
    pub applied: Vec<&'static str>,
}

/// Formats one embedded script; [`fmtkit_ts::format_embedded`] outside tests.
pub(crate) type ScriptFormatter<'f> = &'f dyn Fn(Lang, &str, &TsFormat) -> Result<String, TsError>;

/// Format a whole host document. markup_fmt, oxc_formatter_markdown, and the
/// embedded formatters do not always reach a fixed point in one round, so a
/// document that changed is formatted again, at most [`EXTRA_ROUNDS`] more
/// times, until a round leaves it as it is. A document that does not change
/// costs one round. `applied` is the union of every round's steps in order.
pub fn format_host(rel: &str, lang: Lang, source: &str, options: &TsFormat) -> Result<Formatted, HostError> {
    let _ = rel;

    format_with(lang, source, options, &fmtkit_ts::format_embedded)
}

/// [`format_host`] with the embedded script formatter supplied by the caller.
pub(crate) fn format_with(lang: Lang, source: &str, options: &TsFormat, scripts: ScriptFormatter<'_>) -> Result<Formatted, HostError> {
    let mut formatted = round(lang, source, options, scripts)?;

    if formatted.output == source {
        return Ok(formatted);
    }

    for _ in 0..EXTRA_ROUNDS {
        let next = round(lang, &formatted.output, options, scripts)?;

        if next.output == formatted.output {
            return Ok(formatted);
        }

        for step in next.applied {
            if !formatted.applied.contains(&step) {
                formatted.applied.push(step);
            }
        }

        formatted.output = next.output;
    }

    Err(HostError::Invariant { step: "idempotency", message: format!("the document still changes after {EXTRA_ROUNDS} extra rounds") })
}

/// One formatting round over the whole document.
fn round(lang: Lang, source: &str, options: &TsFormat, scripts: ScriptFormatter<'_>) -> Result<Formatted, HostError> {
    if !lang.is_host() {
        return Ok(Formatted { output: source.to_owned(), applied: Vec::new() });
    }

    let text = normalize_newlines(source);
    let mut applied = Vec::new();

    let output = match lang {
        _ if text.trim().is_empty() => String::new(),
        Lang::Vue | Lang::Html => markup::format(lang, &text, options, scripts, &mut applied)?,
        _ => markdown::format(&text, options, scripts, &mut applied)?,
    };

    let step = if lang == Lang::Markdown { "markdown" } else { "markup" };

    if output != source && !applied.contains(&step) {
        applied.push(step);
    }

    Ok(Formatted { output, applied })
}

/// `\r\n` and lone `\r` become `\n`, as Prettier's `endOfLine: "lf"` does.
fn normalize_newlines(source: &str) -> Cow<'_, str> {
    if source.contains('\r') { Cow::Owned(source.replace("\r\n", "\n").replace('\r', "\n")) } else { Cow::Borrowed(source) }
}

/// Whether two code blocks differ only in their common indentation and in
/// surrounding blank lines, which markup_fmt re-applies when it splices a
/// block back.
fn same_code(a: &str, b: &str) -> bool {
    dedent(a) == dedent(b)
}

fn dedent(code: &str) -> Vec<&str> {
    let lines: Vec<&str> = code.lines().map(str::trim_end).collect();
    let indent = lines.iter().filter(|line| !line.is_empty()).map(|line| line.len() - line.trim_start().len()).min().unwrap_or(0);
    let first = lines.iter().position(|line| !line.is_empty()).unwrap_or(lines.len());
    let last = lines.iter().rposition(|line| !line.is_empty()).map_or(first, |i| i + 1);

    lines[first..last].iter().map(|line| line.get(indent..).unwrap_or_default()).collect()
}

/// A [`HostError::Syntax`] at byte `offset` of `text`.
fn syntax_error(text: &str, offset: usize, message: String) -> HostError {
    let index = LineIndex::new(text);
    let offset = u32::try_from(offset.min(text.len())).unwrap_or(u32::MAX);
    let (line, column) = index.line_col(offset);

    HostError::Syntax { line, column, message }
}

#[cfg(test)]
mod tests;
