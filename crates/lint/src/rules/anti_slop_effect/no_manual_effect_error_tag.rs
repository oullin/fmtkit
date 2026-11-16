//! `anti-slop-effect/no-manual-effect-error-tag`: inside a broad
//! `Effect.catch`, `Effect.catchAll` or `Effect.catchIf` handler, no
//! comparison of `_tag` with a string literal and no `switch` on `_tag`; use
//! `Effect.catchTag(s)`, or `Effect.catchReason(s)` for `error.reason._tag`.
//!
//! The handler is the nearest arrow or function expression around the check,
//! passed as an argument to a call of the literal name `Effect`; checks in a
//! nested callback, or outside any handler, are left to
//! `no-manual-tag-comparison`. Takes no options.

use oxc_ast::AstKind;
use oxc_semantic::AstNode;

use super::super::{Context, Rule};
use super::shared::tagged_values::{is_inside_broad_effect_handler, is_reason_tag, tag_member_from_comparison, tag_member_object};

pub const NAME: &str = "anti-slop-effect/no-manual-effect-error-tag";

const TAG: &str = "Use Effect.catchTag or Effect.catchTags instead of manually discriminating a tagged error in a broad Effect catch handler.";
const REASON: &str = "Use Effect.catchReason or Effect.catchReasons instead of manually discriminating a tagged `reason` in a broad Effect catch handler.";

pub struct NoManualEffectErrorTag;

impl Rule for NoManualEffectErrorTag {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let (span, object) = match node.kind() {
            AstKind::BinaryExpression(binary) => match tag_member_from_comparison(binary) {
                Some(object) => (binary.span, object),
                None => return,
            },
            AstKind::SwitchStatement(switch) => match tag_member_object(&switch.discriminant) {
                Some(object) => (switch.span, object),
                None => return,
            },
            _ => return,
        };

        if is_inside_broad_effect_handler(ctx.semantic.nodes(), node.id()) {
            ctx.report(span, if is_reason_tag(object) { REASON } else { TAG });
        }
    }
}
