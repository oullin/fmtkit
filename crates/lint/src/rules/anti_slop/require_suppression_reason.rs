//! `anti-slop/require-suppression-reason`: every oxlint suppression names its
//! rules and says why after `--`; directives for other linters are rejected.

use oxc_ast::ast::Comment;
use serde_json::Value;

use super::super::{Context, Rule};
use super::shared::{comment_value, is_js_space};

pub const NAME: &str = "anti-slop/require-suppression-reason";

/// Splits a directive body into the rules it names and the reason after it.
const REASON_SEPARATOR: &str = " -- ";

/// A word or two is a placeholder, not a justification. In UTF-16 units.
const MINIMUM_REASON_LENGTH: usize = 16;

pub fn build(_options: &[Value]) -> Result<Box<dyn Rule>, String> {
    Ok(Box::new(RequireSuppressionReason))
}

struct RequireSuppressionReason;

impl Rule for RequireSuppressionReason {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run_once(&self, ctx: &mut Context<'_, '_>) {
        let semantic = ctx.semantic;
        let source = semantic.source_text();

        for comment in semantic.comments() {
            if let Some(fault) = fault(source, comment) {
                ctx.report(comment.span, fault);
            }
        }
    }
}

/// The message for a comment's suppression fault, if it has one.
fn fault(source: &str, comment: &Comment) -> Option<&'static str> {
    let value = comment_value(source, comment).trim_start_matches(is_js_space);

    if is_foreign(value) {
        return Some("This directive targets another linter. Write `oxlint-disable-next-line <rule> -- <reason>`, or delete it.");
    }

    // `oxlint-(disable|disable-next-line|disable-line)\b`: the first alternative
    // wins, so the body of every form starts right after `disable`.
    let body = value.strip_prefix("oxlint-disable").filter(|body| !starts_with_word(body))?;
    let no_rules = "This suppression names no rule, so it silences every future diagnostic on its target. Name the rules it is meant to silence.";

    let Some((rules, reason)) = body.split_once(REASON_SEPARATOR) else {
        return Some(if trim(body).is_empty() { no_rules } else { "This suppression has no justification. Append `-- <reason>` explaining the invariant that makes it safe." });
    };

    if trim(rules).is_empty() {
        return Some(no_rules);
    }

    (trim(reason).encode_utf16().count() < MINIMUM_REASON_LENGTH)
        .then_some("This suppression's justification is too short to be one. State the invariant that makes silencing the rule correct.")
}

/// `/^\s*(eslint|biome|tslint|prettier)-(disable|ignore)\b/u`, after the leading space.
fn is_foreign(value: &str) -> bool {
    ["eslint-", "biome-", "tslint-", "prettier-"].iter().any(|engine| {
        value.strip_prefix(engine).is_some_and(|rest| ["disable", "ignore"].iter().any(|verb| rest.strip_prefix(verb).is_some_and(|rest| !starts_with_word(rest))))
    })
}

/// Whether `text` starts with a word character, so no `\b` precedes it.
fn starts_with_word(text: &str) -> bool {
    text.chars().next().is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// JavaScript's `String.prototype.trim`.
fn trim(text: &str) -> &str {
    text.trim_matches(is_js_space)
}
