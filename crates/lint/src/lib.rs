//! Linting: oxc_linter with the bundled policy, plus fmtkit's native rules
//! (`@nkzw/*`, `perfectionist/*`, `no-only-tests/*`, `anti-slop/*`) run as one
//! visitor over the same semantic model.

mod oxc_bridge;
pub mod rules;

use ignore::gitignore::{Gitignore, GitignoreBuilder};
use oxc_allocator::Allocator;

use fmtkit_core::{Diagnostic, EditSet, Lang};

pub use oxc_bridge::{POLICY, SYNTAX_RULE};

/// How many fix-and-relint rounds `lint` runs before giving up on convergence.
const MAX_FIX_ROUNDS: usize = 10;

#[derive(Debug, thiserror::Error)]
pub enum LintError {
    #[error("lint.rules.{rule}: {message}")]
    Rule { rule: String, message: String },
    #[error("lint.ignore: {pattern}: {message}")]
    Ignore { pattern: String, message: String },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Linted {
    /// The source with every safe fix applied, when `fix` was asked for and
    /// something changed.
    pub fixed: Option<String>,
    /// What remains after fixes, positioned against `fixed` (or the input).
    pub diagnostics: Vec<Diagnostic>,
}

/// A configured linter. Build once, share across threads.
pub struct Linter {
    check: oxc_linter::Linter,
    fix: oxc_linter::Linter,
    natives: oxc_bridge::Natives,
    ignore: Option<Gitignore>,
}

const _: () = {
    const fn assert_send_sync<T: Send + Sync>() {}

    assert_send_sync::<Linter>();
};

impl Linter {
    pub fn new(config: &fmtkit_config::Lint) -> Result<Self, LintError> {
        let built = oxc_bridge::build(config)?;
        let (check, fix) = oxc_bridge::linters(built.store);
        let ignore = ignore_matcher(&config.ignore)?;

        Ok(Self { check, fix, natives: built.natives, ignore })
    }

    /// Whether `[lint] ignore` excludes this repository-relative path.
    pub fn ignores(&self, rel: &str) -> bool {
        self.ignore.as_ref().is_some_and(|matcher| matcher.matched_path_or_any_parents(rel, false).is_ignore())
    }

    /// Lint one file. Vue files lint their `<script>` blocks.
    pub fn lint(&self, rel: &str, lang: Lang, source: &str, fix: bool) -> Linted {
        let mut allocator = Allocator::default();

        if !fix {
            let pass = oxc_bridge::run(&self.check, &self.natives, &allocator, rel, lang, source, false);

            return Linted { fixed: None, diagnostics: sorted(pass.diagnostics) };
        }

        let mut text = source.to_owned();
        let mut pass = oxc_bridge::run(&self.fix, &self.natives, &allocator, rel, lang, &text, true);

        for _ in 0..MAX_FIX_ROUNDS {
            if pass.syntax_error || pass.fixes.is_empty() {
                break;
            }

            let Some(next) = apply_fixes(&text, pass.fixes) else { break };

            allocator.reset();

            let relinted = oxc_bridge::run(&self.fix, &self.natives, &allocator, rel, lang, &next, true);

            // A fix that breaks the parse is discarded along with the rest of its round.
            if relinted.syntax_error {
                pass.fixes = Vec::new();
                break;
            }

            text = next;
            pass = relinted;
        }

        // The last round's fixes are not applied, and a pass with fixes enabled
        // reports exactly what a check would, so its diagnostics stand.
        let fixed = (text != source).then_some(text);

        Linted { fixed, diagnostics: sorted(pass.diagnostics) }
    }
}

/// Apply the fixes that do not overlap an earlier one, as ESLint and oxlint do,
/// and return the new text when it changed.
fn apply_fixes(text: &str, mut fixes: Vec<oxc_bridge::FixUnit>) -> Option<String> {
    fixes.sort_by_key(|unit| (unit.start, unit.end));

    let mut set = EditSet::new();
    let mut last_end: Option<u32> = None;

    for unit in fixes {
        if last_end.is_some_and(|end| unit.start <= end) {
            continue;
        }

        last_end = Some(unit.end);
        set.extend(unit.edits);
    }

    set.normalize(text);

    let next = set.apply(text).ok()?;

    (next != text).then_some(next)
}

fn sorted(mut diagnostics: Vec<Diagnostic>) -> Vec<Diagnostic> {
    diagnostics.sort_by(|a, b| (a.line, a.column, &a.rule, &a.message).cmp(&(b.line, b.column, &b.rule, &b.message)));
    diagnostics.dedup();
    diagnostics
}

fn ignore_matcher(patterns: &[String]) -> Result<Option<Gitignore>, LintError> {
    if patterns.is_empty() {
        return Ok(None);
    }

    let mut builder = GitignoreBuilder::new("");

    for pattern in patterns {
        builder.add_line(None, pattern).map_err(|e| LintError::Ignore { pattern: pattern.clone(), message: e.to_string() })?;
    }

    let matcher = builder.build().map_err(|e| LintError::Ignore { pattern: patterns.join(", "), message: e.to_string() })?;

    Ok(Some(matcher))
}
