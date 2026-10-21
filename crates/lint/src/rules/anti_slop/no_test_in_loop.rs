//! `anti-slop/no-test-in-loop`: no test registered inside a `for` loop; the
//! cases go through `.each`, which names each one and runs it on its own.
//! Adapted from t3code's `t3code/no-test-in-loop`.

use oxc_ast::AstKind;
use oxc_ast::ast::Expression;
use oxc_semantic::AstNode;
use oxc_span::GetSpan;

use super::super::{Context, Rule};
use super::shared::testing::{TEST_FUNCTIONS, is_test_file};
use super::shared::{ancestors, strip_parens};

pub const NAME: &str = "anti-slop/no-test-in-loop";

/// The modifiers between a test function and its call that also have `.each`.
const EACH_MODIFIERS: [&str; 6] = ["concurrent", "effect", "live", "only", "sequential", "skip"];

pub struct NoTestInLoop;

impl Rule for NoTestInLoop {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let AstKind::CallExpression(call) = node.kind() else {
            return;
        };

        if !is_test_file(ctx.rel) {
            return;
        }

        let Some(name) = test_call_name(&call.callee) else {
            return;
        };

        if is_inside_loop(ctx, node) && !uses_node_test(ctx.source()) {
            ctx.report(call.callee.span(), format!("Do not call `{name}(…)` inside a for loop. Use `{name}.each(cases)(name, …)` instead."));
        }
    }
}

/// `node:test` has no `.each`, so files written against it keep their loops.
fn uses_node_test(source: &str) -> bool {
    source.contains("\"node:test\"") || source.contains("'node:test'")
}

/// `it` or `it.skip` as written, when `callee` registers a test.
fn test_call_name(callee: &Expression<'_>) -> Option<String> {
    match strip_parens(callee) {
        Expression::Identifier(identifier) if TEST_FUNCTIONS.contains(&identifier.name.as_str()) => Some(identifier.name.to_string()),
        Expression::StaticMemberExpression(member) if EACH_MODIFIERS.contains(&member.property.name.as_str()) => match strip_parens(&member.object) {
            Expression::Identifier(object) if TEST_FUNCTIONS.contains(&object.name.as_str()) => Some(format!("{}.{}", object.name, member.property.name)),
            _ => None,
        },
        _ => None,
    }
}

/// Whether a `for` loop encloses `node` within its own function.
fn is_inside_loop(ctx: &Context<'_, '_>, node: &AstNode<'_>) -> bool {
    for ancestor in ancestors(ctx.semantic.nodes(), node.id()) {
        match ancestor.kind() {
            AstKind::ForStatement(_) | AstKind::ForInStatement(_) | AstKind::ForOfStatement(_) => return true,
            AstKind::Function(_) | AstKind::ArrowFunctionExpression(_) => return false,
            _ => {}
        }
    }

    false
}
