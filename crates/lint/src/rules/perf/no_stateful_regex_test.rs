//! `perf/no-stateful-regex-test`: no `test` or `exec` on a regular expression
//! with the `g` or `y` flag. Such a pattern keeps `lastIndex` between calls,
//! so a shared one answers the same input differently on each call. This is
//! the guard that makes hoisting patterns to module level safe.
//!
//! A scanner that sets the pattern's `lastIndex` earlier in the same function
//! than the call positions the pattern on purpose and is not reported. An
//! assignment after the call does not cover it.

use oxc_ast::AstKind;
use oxc_ast::ast::{AssignmentTarget, Expression};
use oxc_semantic::AstNode;
use oxc_span::{GetSpan, Span};

use super::super::anti_slop::shared::{enclosing_function, member, strip_parens};
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

        if is_stateful_regex(ctx, object) && !positions_pattern(ctx, node, object, call.span) {
            ctx.report(
                call.span,
                format!("`{method}` on a pattern with the `g` or `y` flag starts from the previous match's `lastIndex`. Use a pattern without those flags."),
            );
        }
    }
}

/// Whether the function around `node` (or the module, outside any function)
/// assigns the pattern's `lastIndex` before `call`, as a scanner does before
/// each `exec`.
fn positions_pattern(ctx: &Context<'_, '_>, node: &AstNode<'_>, object: &Expression<'_>, call: Span) -> bool {
    let Expression::Identifier(pattern) = strip_parens(object) else {
        return false;
    };

    let nodes = ctx.semantic.nodes();
    let scope = enclosing_function(nodes, node.id(), false)
        .map_or_else(|| Span::new(0, ctx.source().len().try_into().unwrap_or(u32::MAX)), |function| function.kind().span());

    nodes.iter().any(|candidate| match candidate.kind() {
        AstKind::AssignmentExpression(assignment) if scope.contains_inclusive(assignment.span) && assignment.span.end <= call.start => matches!(
            &assignment.left,
            AssignmentTarget::StaticMemberExpression(target)
                if target.property.name == "lastIndex" && matches!(strip_parens(&target.object), Expression::Identifier(name) if name.name == pattern.name)
        ),
        _ => false,
    })
}
