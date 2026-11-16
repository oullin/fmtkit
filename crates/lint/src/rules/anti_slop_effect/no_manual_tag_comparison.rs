//! `anti-slop-effect/no-manual-tag-comparison`: no comparison of `_tag` with
//! a string literal and no `switch` on `_tag`; branch on tagged values with
//! `Match.tag(s)`, `Predicate.isTagged` or a tagged enum's `$match`.
//!
//! Checks inside a broad `Effect.catch*` handler are left to
//! `no-manual-effect-error-tag`, which suggests the error-channel helpers
//! instead. Takes no options.

use oxc_ast::AstKind;
use oxc_semantic::AstNode;

use super::super::{Context, Rule};
use super::shared::tagged_values::{is_inside_broad_effect_handler, tag_member_from_comparison, tag_member_object};

pub const NAME: &str = "anti-slop-effect/no-manual-tag-comparison";

const COMPARISON: &str = "Use Match.tag/Match.tags for tagged-value branching, or Predicate.isTagged for a simple reusable predicate.";
const SWITCH: &str =
    "Use Match.value(value).pipe(Match.tag/Match.tags/Match.tagsExhaustive) or the tagged enum `$match` helper instead of switching on `_tag`.";

pub struct NoManualTagComparison;

impl Rule for NoManualTagComparison {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let (span, message) = match node.kind() {
            AstKind::BinaryExpression(binary) if tag_member_from_comparison(binary).is_some() => (binary.span, COMPARISON),
            AstKind::SwitchStatement(switch) if tag_member_object(&switch.discriminant).is_some() => (switch.span, SWITCH),
            _ => return,
        };

        if !is_inside_broad_effect_handler(ctx.semantic.nodes(), node.id()) {
            ctx.report(span, message);
        }
    }
}
