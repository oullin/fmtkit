//! Type-aware rules (oxc's tsgolint rules, such as
//! `typescript/no-floating-promises`) through the native `tsrs` executable.
//!
//! oxc_linter knows these rules and their options but does not run them in
//! process: they need a TypeScript program. fmtkit runs `tsrs --lint headless`
//! once per run over the files in scope, with the protocol oxlint uses for its
//! own type-aware backend, and judges what it reports against disable
//! directives, `[[exempt]]` entries and `[lint] ignore` as any other finding.
//! No Node.js is involved: the native binary is run directly.
//!
//! A type-aware result depends on other files, the tsconfig and
//! `node_modules`, so it never enters the per-file outcome cache.

mod locate;
mod protocol;

use std::path::PathBuf;
use std::process::ExitStatus;

use serde_json::Value;

pub use locate::{TSRS_ENV, locate};
pub use protocol::run;

/// One type-aware rule as the payload names it (`no-floating-promises`, not
/// `typescript/no-floating-promises`), with its options as oxc_linter
/// serializes them for its own type-aware backend.
#[derive(Debug, Clone, PartialEq)]
pub struct TsRule {
    pub name: String,
    pub options: Option<Value>,
}

/// Files that share one rule set.
#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    /// Repository-relative paths.
    pub files: Vec<String>,
    pub rules: Vec<TsRule>,
}

/// One `tsrs` run.
pub struct Request<'a> {
    /// The repository root: tsrs runs there, and paths are relative to it.
    pub root: &'a std::path::Path,
    pub groups: &'a [Group],
    /// In-memory text for files whose buffer differs from disk, keyed by
    /// repository-relative path.
    pub overrides: &'a [(String, String)],
}

/// One diagnostic as tsrs reported it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// The rule's payload name; `None` for a program diagnostic, such as a
    /// tsconfig error.
    pub rule: Option<String>,
    /// Repository-relative path.
    pub file: String,
    /// UTF-8 byte offsets into the file.
    pub start: u32,
    pub end: u32,
    pub message: String,
}

#[derive(Debug, thiserror::Error)]
pub enum TypeAwareError {
    #[error(
        "type-aware rules are on ({}), and they run through tsrs, which was not found. Install it in the repository with `npm i -D tsrs`, point {TSRS_ENV} or `[lint] tsrs` at its native binary (node_modules/@tsrs/<platform>/tsrs), or put `tsrs` on PATH; or turn the rules off under [lint.rules]",
        listed(rules)
    )]
    NotFound { rules: Vec<String> },
    #[error("{named_by} names {}, which is not an executable file; it must name tsrs's native binary (node_modules/@tsrs/<platform>/tsrs)", path.display())]
    NotExecutable { path: PathBuf, named_by: &'static str },
    #[error("could not run {}: {source}", path.display())]
    Spawn { path: PathBuf, source: std::io::Error },
    #[error("{}: {message}", path.display())]
    Backend { path: PathBuf, message: String },
    #[error("{} exited with {status}{}", path.display(), if stderr.is_empty() { String::new() } else { format!(": {stderr}") })]
    Exit { path: PathBuf, status: ExitStatus, stderr: String },
    #[error("{}: unexpected output: {message}", path.display())]
    Protocol { path: PathBuf, message: String },
}

impl TypeAwareError {
    /// Whether the user's setup is at fault (no usable tsrs), rather than tsrs failing.
    pub fn is_setup(&self) -> bool {
        matches!(self, Self::NotFound { .. } | Self::NotExecutable { .. })
    }
}

/// The first few rule names, for a message.
fn listed(rules: &[String]) -> String {
    const SHOWN: usize = 3;

    match rules.len() {
        0 => "under [lint.rules]".to_owned(),
        n if n <= SHOWN => rules.join(", "),
        n => format!("{}, and {} more", rules[..SHOWN].join(", "), n - SHOWN),
    }
}

#[cfg(test)]
mod tests {
    use super::{TypeAwareError, listed};

    #[test]
    fn the_not_found_error_says_how_to_install_tsrs() {
        let rules = ["typescript/a", "typescript/b", "typescript/c", "typescript/d"].map(String::from).to_vec();
        let message = TypeAwareError::NotFound { rules }.to_string();

        assert!(message.contains("npm i -D tsrs"), "{message}");
        assert!(message.contains("FMTKIT_TSRS") && message.contains("[lint] tsrs") && message.contains("PATH"), "{message}");
        assert!(message.contains("typescript/a, typescript/b, typescript/c, and 1 more"), "{message}");
        assert_eq!(listed(&[]), "under [lint.rules]");
        assert!(TypeAwareError::NotFound { rules: Vec::new() }.is_setup());
    }
}
