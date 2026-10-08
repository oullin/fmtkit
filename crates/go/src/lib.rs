//! The Go lane: a client for the `fmtkit-go-helper` process and `go vet`.
//!
//! One helper serves a whole run. Requests are written as frames tagged with an
//! id and may be answered out of order; [`Helper::submit`] never waits for a
//! reply, and [`Ticket::wait`] blocks only the caller that needs it. The wire
//! format lives in [`proto`] and is specified in `go/helper/proto/PROTOCOL.md`.

mod client;
mod locate;
pub mod proto;
mod vet;

use std::path::{Path, PathBuf};

use fmtkit_core::{ComplexityScore, Diagnostic, VetOutcome};

pub use client::{DEV_VERSION, Helper, Ticket, compatible};
pub use locate::{HELPER_NAME, install_candidates};
pub use vet::RULE as VET_RULE;

/// Names the helper executable, overriding the lookup beside `fmtkit`.
pub const HELPER_ENV: &str = "FMTKIT_GO_HELPER";

#[derive(Debug, thiserror::Error)]
pub enum GoError {
    #[error("go helper not found (looked for {0}); install fmtkit-go-helper beside fmtkit or set FMTKIT_GO_HELPER")]
    NotFound(PathBuf),
    #[error("go helper: {0}")]
    Spawn(String),
    #[error("go helper speaks protocol {found}, fmtkit {version} needs {expected}")]
    Version { found: String, expected: String, version: String },
    #[error("go helper died: {0}")]
    Crashed(String),
    #[error("go helper sent a malformed frame: {0}")]
    Protocol(String),
}

/// What the helper should do to one file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(clippy::struct_excessive_bools)]
pub struct Steps {
    pub spacing: bool,
    pub gofmt: bool,
    pub goimports: bool,
    pub resolve_imports: bool,
    /// Score the final text.
    pub complexity: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// Repository-relative path; keys complexity scores and diagnostics.
    pub rel: String,
    /// Absolute path; goimports resolves the package from it.
    pub abs: PathBuf,
    pub source: Vec<u8>,
    pub steps: Steps,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Reply {
    /// The formatted text; equal to the input when nothing changed.
    pub output: Vec<u8>,
    /// The steps that changed the text, in order: `spacing`, `gofmt`, `goimports`.
    pub applied: Vec<String>,
    /// Spacing rule findings against the input.
    pub violations: Vec<Diagnostic>,
    pub complexity: Vec<ComplexityScore>,
    /// The file could not be processed (a syntax error, usually).
    pub error: Option<String>,
}

/// Which packages `go vet` checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VetTargets {
    /// `./...` in every module under the root.
    All,
    /// The packages (directories) holding these repository-relative Go files.
    Files(Vec<String>),
}

/// Run `go vet` under `root`. Blocks; run it on its own thread.
///
/// Every module (a directory holding `go.mod`, outside `vendor`, `testdata`,
/// ignored, and `.`/`_` directories) is vetted from its own directory: with
/// `./...` for [`VetTargets::All`], or with just the packages holding the
/// listed files. Findings become [`VET_RULE`] diagnostics with
/// repository-relative paths. Vet is skipped, with the reason recorded, when
/// `go` is not on `PATH` or nothing in scope belongs to a module.
pub fn vet(root: &Path, targets: &VetTargets) -> VetOutcome {
    vet::run(root, targets)
}
