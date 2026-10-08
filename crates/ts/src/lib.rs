//! The TypeScript and JavaScript lane: one in-memory pipeline per file.
//!
//! parse → segment passes (BodyWrap until stable, ClassReorder,
//! DeclarationReorder, BlankLine) → oxc_formatter → fluent passes (FluentChain,
//! DrizzleQuery, ExpandedCall) → segment passes → validate → complexity.
//! Lint fixes are applied by the caller before this pipeline runs.

pub mod complexity;

use fmtkit_config::TsFormat;
use fmtkit_core::{ComplexityScore, Lang};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TsError {
    /// The input does not parse. `line` and `column` are 1-based.
    #[error("{line}:{column}: {message}")]
    Syntax { line: u32, column: u32, message: String },
    /// A step produced text that does not parse or that parses to a different
    /// program; the file is left untouched.
    #[error("{step} produced invalid output: {message}")]
    Invariant { step: &'static str, message: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Formatted {
    pub output: String,
    /// The steps that changed the text, in order.
    pub applied: Vec<&'static str>,
    /// Scores for the output when scoring was requested.
    pub complexity: Vec<ComplexityScore>,
}

/// Format one script. `rel` is the repository-relative path used in
/// complexity keys; `score` asks for complexity scores of the output.
pub fn format_source(rel: &str, lang: Lang, source: &str, options: &TsFormat, score: bool) -> Result<Formatted, TsError> {
    let _ = (rel, lang, options, score);

    Ok(Formatted { output: source.to_owned(), ..Formatted::default() })
}

/// Format a script embedded in a host document (a Vue `<script>`, an HTML
/// `<script>`, a Markdown fence). `lang` picks the dialect.
pub fn format_embedded(lang: Lang, source: &str, options: &TsFormat) -> Result<String, TsError> {
    format_source("", lang, source, options, false).map(|f| f.output)
}

/// Complexity scores for every function in `source` without formatting it.
pub fn score(rel: &str, lang: Lang, source: &str) -> Result<Vec<ComplexityScore>, TsError> {
    let _ = (rel, lang, source);

    Ok(Vec::new())
}
