//! The TypeScript and JavaScript lane: one in-memory pipeline per file.
//!
//! parse → segment passes (BodyWrap until stable, ClassReorder,
//! DeclarationReorder, BlankLine) → oxc_formatter → fluent passes (FluentChain,
//! DrizzleQuery, ExpandedCall) → segment passes → validate → complexity.
//! Lint fixes are applied by the caller before this pipeline runs.

pub mod complexity;
mod embed;
mod fingerprint;
mod format;
mod passes;
mod pipeline;
mod syntax;

use fmtkit_config::TsFormat;
use fmtkit_core::{ComplexityScore, Lang};
use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use oxc_span::SourceType;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TsError {
    /// The input does not parse. `line` and `column` are 1-based.
    #[error("{line}:{column}: {message}")]
    Syntax { line: u32, column: u32, message: String },
    /// A step produced text that does not parse or that parses to a different
    /// program, or (with `step` `idempotency`) the output never stopped
    /// changing; the file is left untouched.
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
///
/// `applied` is empty when the output equals the input. The output is a
/// fixed point: when it differs from the input, the pipeline runs again on it
/// until it stops changing. A declaration file (`.d.ts`) is validated and
/// scored but never rewritten.
pub fn format_source(rel: &str, lang: Lang, source: &str, options: &TsFormat, score: bool) -> Result<Formatted, TsError> {
    pipeline::run(rel, source_type(lang, rel)?, source, options, score, &pipeline::FULL)
}

/// Format a script embedded in a host document (a Vue `<script>`, an HTML
/// `<script>`, a Markdown fence). `lang` picks the dialect.
pub fn format_embedded(lang: Lang, source: &str, options: &TsFormat) -> Result<String, TsError> {
    format_source("", lang, source, options, false).map(|f| f.output)
}

/// Complexity scores for every function in `source` without formatting it.
pub fn score(rel: &str, lang: Lang, source: &str) -> Result<Vec<ComplexityScore>, TsError> {
    with_program(rel, lang, source, |program| complexity::score_program(rel, program, source))
}

/// Check that `source` parses, as v1's `validate-syntax` did.
pub fn validate(rel: &str, lang: Lang, source: &str) -> Result<(), TsError> {
    with_program(rel, lang, source, |_| ())
}

fn with_program<T>(rel: &str, lang: Lang, source: &str, f: impl FnOnce(&Program<'_>) -> T) -> Result<T, TsError> {
    let allocator = Allocator::default();
    let program = syntax::parse(&allocator, source, source_type(lang, rel)?).map_err(|failure| pipeline::syntax_error(source, failure))?;

    Ok(f(program))
}

fn source_type(lang: Lang, rel: &str) -> Result<SourceType, TsError> {
    syntax::source_type(lang, rel).ok_or_else(|| TsError::Invariant { step: "parse", message: format!("{lang:?} is not a script language") })
}

#[cfg(test)]
mod tests;
