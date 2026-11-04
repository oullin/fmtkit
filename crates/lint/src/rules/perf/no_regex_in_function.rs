//! `perf/no-regex-in-function`: a regular expression written inside a
//! function is built again on every call; a module-level constant is built
//! once.
//!
//! A pattern with the `g` or `y` flag keeps `lastIndex`, so a fresh one per
//! call is what lets an `exec` loop start from the beginning. Such a pattern
//! is reported only where sharing it is safe: as the pattern argument of
//! `split`, which copies it, or, with the `g` flag, of `replace`,
//! `replaceAll`, `match` or `matchAll`, which reset `lastIndex` first. Held in
//! a variable or passed to any other call, it is left alone. Test files are
//! skipped.

use oxc_ast::AstKind;
use oxc_semantic::AstNode;
use oxc_span::Span;

use super::super::anti_slop::shared::testing::is_test_file;
use super::super::anti_slop::shared::{enclosing_function, is_global};
use super::super::{Context, Rule};
use super::shared::{StateFlags, is_shared_safely, static_arguments};

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
/// called with static arguments, when its flags make sharing it safe here.
fn built_pattern(ctx: &Context<'_, '_>, node: &AstNode<'_>) -> Option<Span> {
    let (span, flags) = match node.kind() {
        AstKind::RegExpLiteral(literal) => (literal.span, StateFlags::of_literal(literal.regex.flags)),
        AstKind::NewExpression(new) if is_global(ctx, &new.callee, "RegExp") && static_arguments(&new.arguments) => {
            (new.span, StateFlags::of_arguments(&new.arguments))
        }
        AstKind::CallExpression(call) if is_global(ctx, &call.callee, "RegExp") && static_arguments(&call.arguments) => {
            (call.span, StateFlags::of_arguments(&call.arguments))
        }
        _ => return None,
    };

    (!flags.stateful() || is_shared_safely(ctx, node, flags)).then_some(span)
}
