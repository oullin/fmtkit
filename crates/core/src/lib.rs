//! Shared vocabulary for every fmtkit crate: languages, run modes, text edits,
//! diagnostics, complexity scores, and the per-file and per-run outcomes the
//! engine produces and the reporters render.
//!
//! This crate carries no behaviour beyond small value methods, so every lane can
//! depend on it without pulling in another lane.

mod edit;
mod lang;
mod lines;
mod outcome;

pub use edit::{Edit, EditConflict, EditSet};
pub use lang::{Lane, Lang, is_declaration, is_test_file};
pub use lines::LineIndex;
pub use outcome::{AllowEntryStatus, ComplexityFinding, ComplexityScore, Diagnostic, FileOutcome, Mode, Report, RunResult, Severity, VetOutcome};

/// The schema version stamped on every machine-readable report.
pub const REPORT_SCHEMA: u32 = 2;

/// The fmtkit version, shared by the CLI, the cache key, and the Go helper handshake.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
