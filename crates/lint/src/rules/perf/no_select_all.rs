//! `perf/no-select-all`: a Drizzle `select()` or `returning()` with no column
//! map reads every column of the row, including large `jsonb` and `text`
//! columns the caller never reads. Pass the columns the caller uses.

use oxc_ast::AstKind;
use oxc_semantic::AstNode;

use super::super::anti_slop::shared::{member, parent};
use super::super::{Context, Rule};
use super::shared::chain_calls;

pub const NAME: &str = "perf/no-select-all";

/// The statements whose `returning()` reads the written rows back.
const WRITES: [&str; 3] = ["delete", "insert", "update"];

pub struct NoSelectAll;

impl Rule for NoSelectAll {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let AstKind::CallExpression(call) = node.kind() else {
            return;
        };

        if !call.arguments.is_empty() {
            return;
        }

        let Some((object, method)) = member(&call.callee) else {
            return;
        };

        let reads_all = match method {
            "select" | "selectDistinct" => is_followed_by_from(ctx, node),
            "returning" => chain_calls(object, &WRITES),
            _ => false,
        };

        if reads_all {
            ctx.report(call.span, format!("`{method}()` reads every column. Pass only the columns the caller reads, as `{method}({{ id: table.id }})`."));
        }
    }
}

/// Whether the call is the object of `.from(…)`, as in `db.select().from(t)`.
fn is_followed_by_from(ctx: &Context<'_, '_>, node: &AstNode<'_>) -> bool {
    matches!(parent(ctx.semantic.nodes(), node.id()).kind(), AstKind::StaticMemberExpression(next) if next.property.name == "from")
}
