//! `perf/no-stateful-regex-test`: no `test` or `exec` on a shared regular
//! expression with the `g` or `y` flag. Such a pattern keeps `lastIndex`
//! between calls, so a shared one answers the same input differently on each
//! call. This is the guard that makes hoisting patterns to module level safe.
//!
//! Only a name bound to a `const` pattern in this file counts, and only when
//! the pattern outlives the function making the call. A literal or a
//! `RegExp(…)` written at the call, or a `const` declared in the same function
//! call, is built fresh, starting at `lastIndex` 0, and its `y` flag changes
//! what it matches.
//!
//! A scanner that sets the pattern's `lastIndex` before the call on every path
//! to it positions the pattern on purpose and is not reported: the assignment
//! is a statement in a block that holds the call, in the same function. An
//! assignment after the call, inside a branch, or outside the closure that
//! makes the call does not cover it.

use oxc_ast::AstKind;
use oxc_ast::ast::{AssignmentTarget, Expression, FunctionType, IdentifierReference};
use oxc_semantic::{AstNode, AstNodes, NodeId};
use oxc_span::GetSpan;
use oxc_syntax::symbol::SymbolId;

use super::super::anti_slop::shared::{ancestors, enclosing_function, member, parent, strip_parens};
use super::super::{Context, Rule};
use super::shared::is_stateful_regex;

pub const NAME: &str = "perf/no-stateful-regex-test";

pub struct NoStatefulRegexTest;

impl Rule for NoStatefulRegexTest {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let AstKind::CallExpression(call) = node.kind() else {
            return;
        };

        let Some((object, method @ ("test" | "exec"))) = member(&call.callee) else {
            return;
        };

        let Expression::Identifier(pattern) = strip_parens(object) else {
            return;
        };

        if is_stateful_regex(ctx, object) && !is_fresh_per_call(ctx, node, pattern) && !positions_pattern(ctx, node, pattern) {
            ctx.report(
                call.span,
                format!("`{method}` on a pattern with the `g` or `y` flag starts from the previous match's `lastIndex`. Use a pattern without those flags."),
            );
        }
    }
}

/// Whether the function making `call` declares `pattern` itself, so every
/// invocation tests a pattern of its own.
fn is_fresh_per_call(ctx: &Context<'_, '_>, call: &AstNode<'_>, pattern: &IdentifierReference<'_>) -> bool {
    let (Some(symbol), Some(function)) = (binding(ctx, pattern), enclosing_function(ctx.semantic.nodes(), call.id(), false)) else {
        return false;
    };

    function.kind().span().contains_inclusive(ctx.semantic.symbol_declaration(symbol).kind().span())
}

/// Whether `pattern.lastIndex = …` runs before `call` on every path to it: an
/// expression statement, earlier in the source, whose block holds the call
/// without a function boundary between them.
fn positions_pattern(ctx: &Context<'_, '_>, call: &AstNode<'_>, pattern: &IdentifierReference<'_>) -> bool {
    let Some(symbol) = binding(ctx, pattern) else {
        return false;
    };

    let nodes = ctx.semantic.nodes();
    let blocks = enclosing_blocks(nodes, call.id());
    let start = call.kind().span().start;

    nodes.iter().any(|candidate| {
        let AstKind::AssignmentExpression(assignment) = candidate.kind() else {
            return false;
        };

        let AssignmentTarget::StaticMemberExpression(target) = &assignment.left else {
            return false;
        };

        let resets =
            target.property.name == "lastIndex" && matches!(strip_parens(&target.object), Expression::Identifier(name) if binding(ctx, name) == Some(symbol));

        resets && assignment.span.end <= start && {
            let statement = parent(nodes, candidate.id());

            matches!(statement.kind(), AstKind::ExpressionStatement(_)) && blocks.contains(&parent(nodes, statement.id()).id())
        }
    })
}

/// The binding `identifier` refers to.
fn binding(ctx: &Context<'_, '_>, identifier: &IdentifierReference<'_>) -> Option<SymbolId> {
    ctx.semantic.scoping().get_reference(identifier.reference_id.get()?).symbol_id()
}

/// The ancestors of `id` up to and including its function, or up to the
/// module outside any function.
fn enclosing_blocks(nodes: &AstNodes<'_>, id: NodeId) -> Vec<NodeId> {
    let mut blocks = Vec::new();

    for node in ancestors(nodes, id) {
        blocks.push(node.id());

        if matches!(node.kind(), AstKind::ArrowFunctionExpression(_))
            || matches!(node.kind(), AstKind::Function(function) if matches!(function.r#type, FunctionType::FunctionDeclaration | FunctionType::FunctionExpression))
        {
            break;
        }
    }

    blocks
}
