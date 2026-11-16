//! `anti-slop-effect/prefer-tagged-error-handling`: inside a broad
//! `Effect.catch`, `Effect.catchAll` or `Effect.catchIf` handler, no
//! `Predicate.isTagged` check; use `Effect.catchTag(s)`, or
//! `Effect.catchReason(s)` when the subject is `error.reason`.
//!
//! Both forms count: `Predicate.isTagged(value, "Tag")` and
//! `Predicate.isTagged("Tag")(value)`, by the literal name `Predicate`. A check
//! in a callback nested in the handler is not the handler's own. Takes no
//! options.

use oxc_ast::AstKind;
use oxc_ast::ast::{Argument, CallExpression, Expression};
use oxc_semantic::AstNode;

use super::super::anti_slop::shared::strip_parens;
use super::super::{Context, Rule};
use super::shared::tagged_values::is_inside_broad_effect_handler;

pub const NAME: &str = "anti-slop-effect/prefer-tagged-error-handling";

const ERROR: &str = "Use Effect.catchTag or Effect.catchTags instead of checking the error with `Predicate.isTagged` in a broad Effect catch handler.";
const REASON: &str =
    "Use Effect.catchReason or Effect.catchReasons instead of checking the error `reason` with `Predicate.isTagged` in a broad Effect catch handler.";

pub struct PreferTaggedErrorHandling;

impl Rule for PreferTaggedErrorHandling {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let AstKind::CallExpression(call) = node.kind() else {
            return;
        };
        let Some(subject) = tagged_subject(call) else {
            return;
        };

        if !is_inside_broad_effect_handler(ctx.semantic.nodes(), node.id()) {
            return;
        }

        let is_reason = subject
            .as_expression()
            .is_some_and(|subject| matches!(strip_parens(subject), Expression::StaticMemberExpression(member) if member.property.name == "reason"));

        ctx.report(call.span, if is_reason { REASON } else { ERROR });
    }
}

/// `Predicate.isTagged`, by the literal name.
fn is_predicate_is_tagged(expression: &Expression<'_>) -> bool {
    match strip_parens(expression) {
        Expression::StaticMemberExpression(member) => {
            member.property.name == "isTagged" && matches!(strip_parens(&member.object), Expression::Identifier(object) if object.name == "Predicate")
        }
        _ => false,
    }
}

/// The value a `Predicate.isTagged` call checks.
fn tagged_subject<'b, 'a>(call: &'b CallExpression<'a>) -> Option<&'b Argument<'a>> {
    if is_predicate_is_tagged(&call.callee) && call.arguments.len() == 2 {
        return call.arguments.first();
    }

    match strip_parens(&call.callee) {
        Expression::CallExpression(inner) if is_predicate_is_tagged(&inner.callee) && inner.arguments.len() == 1 && call.arguments.len() == 1 => {
            call.arguments.first()
        }
        _ => None,
    }
}
