//! `perf/no-regex-in-function`: a regular expression written inside a
//! function is built again on every call; a module-level constant is built
//! once.
//!
//! A pattern held in a variable with the `g` or `y` flag is left alone: it
//! keeps `lastIndex`, so a fresh one per call is what makes an `exec` loop
//! start from the beginning. Passed straight to `replace`, `split` or
//! `matchAll`, the flag is safe to share, so those are still reported.
//! Test files are skipped.

use oxc_ast::AstKind;
use oxc_ast::ast::RegExpFlags;
use oxc_semantic::AstNode;
use oxc_span::{GetSpan, Span};

use super::super::anti_slop::shared::testing::is_test_file;
use super::super::anti_slop::shared::{enclosing_function, is_global, parent};
use super::super::{Context, Rule};
use super::shared::static_arguments;

pub const NAME: &str = "perf/no-regex-in-function";

const MESSAGE: &str = "This regular expression is built on every call. Hoist it to a module-level constant.";

pub struct NoRegexInFunction;

impl Rule for NoRegexInFunction {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let Some(span) = built_pattern(ctx, node) else {
            return;
        };

        if is_test_file(ctx.rel) || enclosing_function(ctx.semantic.nodes(), node.id(), false).is_none() {
            return;
        }

        ctx.report(span, MESSAGE);
    }
}

/// The span of a pattern the function could share: a literal, or `RegExp`
/// called with static arguments.
fn built_pattern(ctx: &Context<'_, '_>, node: &AstNode<'_>) -> Option<Span> {
    match node.kind() {
        AstKind::RegExpLiteral(literal) => {
            let stateful = literal.regex.flags.intersects(RegExpFlags::G | RegExpFlags::Y);

            (!stateful || is_call_argument(ctx, node)).then_some(literal.span)
        }
        AstKind::NewExpression(new) if is_global(ctx, &new.callee, "RegExp") && static_arguments(&new.arguments) => Some(new.span),
        AstKind::CallExpression(call) if is_global(ctx, &call.callee, "RegExp") && static_arguments(&call.arguments) => Some(call.span),
        _ => None,
    }
}

/// Whether `node` is passed straight to a call, as in `text.replace(/a/g, "b")`.
fn is_call_argument(ctx: &Context<'_, '_>, node: &AstNode<'_>) -> bool {
    matches!(parent(ctx.semantic.nodes(), node.id()).kind(), AstKind::CallExpression(call) if call.arguments.iter().any(|argument| argument.span() == node.kind().span()))
}
