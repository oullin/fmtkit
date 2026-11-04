//! `perf/no-stateful-regex-test`: no `test` or `exec` on a regular expression
//! with the `g` or `y` flag. Such a pattern keeps `lastIndex` between calls,
//! so a shared one answers the same input differently on each call. This is
//! the guard that makes hoisting patterns to module level safe.

use oxc_ast::AstKind;
use oxc_semantic::AstNode;

use super::super::anti_slop::shared::member;
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

        if is_stateful_regex(ctx, object) {
            ctx.report(
                call.span,
                format!("`{method}` on a pattern with the `g` or `y` flag starts from the previous match's `lastIndex`. Use a pattern without those flags."),
            );
        }
    }
}
