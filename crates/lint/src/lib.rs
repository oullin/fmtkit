//! Linting: oxc_linter with the bundled policy, plus fmtkit's native rules
//! (`@nkzw/*`, `perfectionist/*`, `no-only-tests/*`, `anti-slop/*`,
//! `observability/*`) run as one visitor over the same semantic model.
//! Type-aware rules, when one is on, run per project through `tsrs` (see
//! [`type_aware`]).

mod exempt;
mod oxc_bridge;
pub mod rules;
pub mod type_aware;

use std::collections::HashMap;
use std::path::Path;

use ignore::gitignore::Gitignore;
use oxc_allocator::Allocator;
use oxc_linter::ConfigStore;
use oxc_span::SourceType;

use fmtkit_core::{Diagnostic, EditSet, Lang, LineIndex};

pub use oxc_bridge::{NKZW_TYPE_AWARE, POLICY, PROGRAM_RULE, SYNTAX_RULE, UNUSED_DIRECTIVE_RULE};
pub use type_aware::TypeAwareError;

/// How many fix-and-relint rounds `lint` runs before giving up on convergence.
const MAX_FIX_ROUNDS: usize = 10;

#[derive(Debug, thiserror::Error)]
pub enum LintError {
    #[error("lint.rules.{rule}: {message}")]
    Rule { rule: String, message: String },
    #[error("lint.ignore: {pattern}: {message}")]
    Ignore { pattern: String, message: String },
    #[error("lint.exempt: {rule}: {message}")]
    Exempt { rule: String, message: String },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Linted {
    /// The source with every safe fix applied, when `fix` was asked for and
    /// something changed.
    pub fixed: Option<String>,
    /// What remains after fixes, positioned against `fixed` (or the input).
    pub diagnostics: Vec<Diagnostic>,
}

/// What [`Linter::lint_types`] reports.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TypeAwareLint {
    /// The findings, sorted by file, then position.
    pub diagnostics: Vec<Diagnostic>,
    /// The file, line and column of each disable directive that names no rule
    /// and suppressed a tsrs finding. [`Linter::lint`] reports such a
    /// directive as unused when it suppressed nothing there, so that report is
    /// withdrawn.
    pub used_directives: Vec<(String, u32, u32)>,
}

/// A configured linter. Build once, share across threads.
pub struct Linter {
    check: oxc_linter::Linter,
    fix: oxc_linter::Linter,
    natives: oxc_bridge::Natives,
    ignore: Option<Gitignore>,
    /// Resolves the type-aware rules of each file, overrides applied; `None`
    /// when none is on.
    type_store: Option<ConfigStore>,
    /// The type-aware rules on for some file.
    type_aware: Vec<String>,
    /// `[lint] tsrs`.
    tsrs: Option<String>,
}

const _: () = {
    const fn assert_send_sync<T: Send + Sync>() {}

    assert_send_sync::<Linter>();
};

impl Linter {
    pub fn new(config: &fmtkit_config::Lint) -> Result<Self, LintError> {
        let built = oxc_bridge::build(config)?;
        let (check, fix) = oxc_bridge::linters(built.store, built.unused_directives);
        let ignore = ignore_matcher(&config.ignore)?;

        Ok(Self { check, fix, natives: built.natives, ignore, type_store: built.type_store, type_aware: built.type_aware, tsrs: config.tsrs.clone() })
    }

    /// The type-aware rules that are on for some file, sorted. When there are
    /// any, [`Linter::lint_types`] needs `tsrs`.
    pub fn type_aware_rules(&self) -> &[String] {
        &self.type_aware
    }

    /// The `tsrs` that runs the type-aware rules for `root` (see
    /// [`type_aware::locate`]). Not finding one is an error that names the
    /// rules that need it and says how to install it.
    pub fn locate_tsrs(&self, root: &Path) -> Result<std::path::PathBuf, TypeAwareError> {
        type_aware::locate(root, self.tsrs.as_deref()).map_err(|error| match error {
            TypeAwareError::NotFound { .. } => TypeAwareError::NotFound { rules: self.type_aware.clone() },
            other => other,
        })
    }

    /// Run the type-aware rules over `files` (repository-relative) with one
    /// call of the `tsrs` at `tsrs`, from `root`, and judge what it reports as
    /// [`Linter::lint`] judges its findings. Files that are not scripts, that
    /// `[lint] ignore` skips, or that no type-aware rule applies to are left
    /// out, and with none left tsrs does not run. `overrides` give the text of
    /// files whose buffer differs from disk.
    ///
    /// The findings come sorted by file, then position. A program diagnostic
    /// (a tsconfig error) is reported as [`PROGRAM_RULE`] against the file it
    /// names, which may be outside `files`.
    pub fn lint_types(&self, root: &Path, tsrs: &Path, files: &[String], overrides: &[(String, String)]) -> Result<TypeAwareLint, TypeAwareError> {
        let Some(store) = &self.type_store else {
            return Ok(TypeAwareLint::default());
        };

        let mut groups: Vec<type_aware::Group> = Vec::new();
        let mut rule_sets: Vec<Vec<(type_aware::TsRule, fmtkit_core::Severity)>> = Vec::new();
        let mut linted: Vec<(&str, usize)> = Vec::new();

        for rel in files {
            let script = SourceType::from_path(rel).is_ok() && Lang::from_path(Path::new(rel)).is_some_and(Lang::is_script);

            if !script || self.ignores(rel) {
                continue;
            }

            let rules = oxc_bridge::type_aware_rules_for(store, rel);

            if rules.is_empty() {
                continue;
            }

            let at = rule_sets.iter().position(|set| *set == rules).unwrap_or_else(|| {
                groups.push(type_aware::Group { files: Vec::new(), rules: rules.iter().map(|(rule, _)| rule.clone()).collect() });
                rule_sets.push(rules);
                rule_sets.len() - 1
            });

            groups[at].files.push(rel.clone());
            linted.push((rel, at));
        }

        if groups.is_empty() {
            return Ok(TypeAwareLint::default());
        }

        let findings = type_aware::run(tsrs, &type_aware::Request { root, groups: &groups, overrides })?;
        let text =
            |rel: &str| overrides.iter().find(|(file, _)| file == rel).map(|(_, text)| text.clone()).or_else(|| std::fs::read_to_string(root.join(rel)).ok());
        let mut by_file: HashMap<String, Vec<type_aware::Finding>> = HashMap::new();

        for finding in findings {
            by_file.entry(finding.file.clone()).or_default().push(finding);
        }

        let mut diagnostics = Vec::new();
        let mut used_directives = Vec::new();

        for (rel, at) in &linted {
            let own = by_file.remove(*rel).unwrap_or_default();

            if own.is_empty() && self.natives.unused_directives.is_none() {
                continue;
            }

            let text = text(rel).unwrap_or_default();

            let (judged, used) = oxc_bridge::judge_type_aware(&self.natives, rel, &text, &own, &rule_sets[*at]);

            diagnostics.extend(judged);
            used_directives.extend(used.into_iter().map(|(line, column)| ((*rel).to_owned(), line, column)));
        }

        // Program diagnostics, and anything reported for a file that was not sent.
        for finding in by_file.into_values().flatten() {
            let (line, column) = text(&finding.file).map_or((0, 0), |text| LineIndex::new(&text).line_col(finding.start));
            let rule = finding.rule.as_ref().map_or_else(|| PROGRAM_RULE.to_owned(), |name| format!("typescript/{name}"));

            diagnostics.push(Diagnostic {
                rule,
                file: finding.file.clone(),
                line,
                column,
                message: finding.message.clone(),
                severity: fmtkit_core::Severity::Error,
            });
        }

        diagnostics.sort_by(|a, b| (&a.file, a.line, a.column, &a.rule, &a.message).cmp(&(&b.file, b.line, b.column, &b.rule, &b.message)));
        diagnostics.dedup();

        Ok(TypeAwareLint { diagnostics, used_directives })
    }

    /// Whether [`Linter::lint_types`] sends this file to tsrs: a script that
    /// `[lint] ignore` does not skip and that a type-aware rule applies to.
    fn type_checks(&self, rel: &str, lang: Lang) -> bool {
        self.type_store.as_ref().is_some_and(|store| {
            lang.is_script() && SourceType::from_path(rel).is_ok() && !self.ignores(rel) && !oxc_bridge::type_aware_rules_for(store, rel).is_empty()
        })
    }

    /// Whether `[lint] ignore` excludes this repository-relative path.
    pub fn ignores(&self, rel: &str) -> bool {
        self.ignore.as_ref().is_some_and(|matcher| exempt::matched(matcher, rel))
    }

    /// Lint one file. Vue files lint their `<script>` blocks.
    pub fn lint(&self, rel: &str, lang: Lang, source: &str, fix: bool) -> Linted {
        let mut allocator = Allocator::default();
        let type_checked = self.type_checks(rel, lang);

        if !fix {
            let pass = oxc_bridge::run(&self.check, &self.natives, &allocator, rel, lang, source, false, type_checked);

            return Linted { fixed: None, diagnostics: sorted(pass.diagnostics) };
        }

        let mut text = source.to_owned();
        let mut pass = oxc_bridge::run(&self.fix, &self.natives, &allocator, rel, lang, &text, true, type_checked);

        for _ in 0..MAX_FIX_ROUNDS {
            if pass.syntax_error || pass.fixes.is_empty() {
                break;
            }

            let Some(next) = apply_fixes(&text, pass.fixes) else { break };

            allocator.reset();

            let relinted = oxc_bridge::run(&self.fix, &self.natives, &allocator, rel, lang, &next, true, type_checked);

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

    let matcher = exempt::matcher(patterns).map_err(|(pattern, message)| LintError::Ignore { pattern, message })?;

    Ok(Some(matcher))
}
