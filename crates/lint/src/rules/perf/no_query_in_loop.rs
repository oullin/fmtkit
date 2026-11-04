//! `perf/no-query-in-loop`: no awaited query inside a loop body, the N+1
//! shape. A query is an awaited call chain that starts from one of the
//! configured receivers, either directly (`tx.select()…`) or as a property of
//! `this` (`this.db.update(t)…`). A callback starts a new function, so a
//! query inside `items.map(async …)` is not reported here.

use oxc_ast::AstKind;
use oxc_ast::ast::Expression;
use oxc_semantic::AstNode;
use oxc_span::{GetSpan, Span};
use serde_json::Value;

use super::super::anti_slop::shared::{ancestors, strip_parens};
use super::super::{Context, Rule};
use super::shared::receiver;

pub const NAME: &str = "perf/no-query-in-loop";

/// The receivers checked when no `receivers` option is given.
const RECEIVERS: [&str; 4] = ["db", "transaction", "trx", "tx"];

/// `[{ receivers?: string[] }]`; `receivers` replaces the default list.
pub fn build(options: &[Value]) -> Result<Box<dyn Rule>, String> {
    let mut receivers: Vec<String> = RECEIVERS.iter().map(|name| (*name).to_owned()).collect();

    match options {
        [] => {}
        [Value::Object(object)] => {
            for (key, value) in object {
                match key.as_str() {
                    "receivers" => receivers = names(value).ok_or("\"receivers\" must be an array of non-empty strings")?,
                    other => return Err(format!("unknown option \"{other}\"")),
                }
            }
        }
        [_] => return Err("expected an options object".into()),
        _ => return Err("expected at most one options object".into()),
    }

    Ok(Box::new(NoQueryInLoop { receivers }))
}

fn names(value: &Value) -> Option<Vec<String>> {
    value.as_array()?.iter().map(|name| name.as_str().filter(|name| !name.is_empty()).map(str::to_owned)).collect()
}

struct NoQueryInLoop {
    receivers: Vec<String>,
}

impl Rule for NoQueryInLoop {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let AstKind::AwaitExpression(awaited) = node.kind() else {
            return;
        };

        let Expression::CallExpression(call) = strip_parens(&awaited.argument) else {
            return;
        };

        let Some(name) = receiver(&call.callee).filter(|name| self.receivers.iter().any(|receiver| receiver == name)) else {
            return;
        };

        if is_in_loop_body(ctx, node, awaited.span) {
            ctx.report(
                awaited.span,
                format!(
                    "`{name}` is queried once per loop iteration. Read every item in one query (`inArray`, a join, or one `sql` statement) before the loop."
                ),
            );
        }
    }
}

/// Whether a loop in the same function runs `span` once per iteration: it
/// sits in the loop's body, not in the iterable or the initialiser.
fn is_in_loop_body(ctx: &Context<'_, '_>, node: &AstNode<'_>, span: Span) -> bool {
    for ancestor in ancestors(ctx.semantic.nodes(), node.id()) {
        let body = match ancestor.kind() {
            AstKind::ForStatement(statement) => statement.body.span(),
            AstKind::ForInStatement(statement) => statement.body.span(),
            AstKind::ForOfStatement(statement) => statement.body.span(),
            AstKind::WhileStatement(statement) => statement.body.span(),
            AstKind::DoWhileStatement(statement) => statement.body.span(),
            AstKind::Function(_) | AstKind::ArrowFunctionExpression(_) => return false,
            _ => continue,
        };

        if body.contains_inclusive(span) {
            return true;
        }
    }

    false
}
