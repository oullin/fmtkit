//! `anti-slop/no-ambient-nondeterminism`: no reading the clock or the entropy
//! source; deterministic code takes time and randomness from its caller.

use oxc_ast::AstKind;
use oxc_ast::ast::Expression;
use oxc_semantic::AstNode;
use serde_json::Value;

use super::super::{Context, Rule};
use super::shared::{is_global, is_global_member};

pub const NAME: &str = "anti-slop/no-ambient-nondeterminism";

/// Global readings of the clock and the entropy source: owner, property and display name.
const AMBIENT_MEMBERS: [(&str, &str, &str); 6] = [
    ("Date", "now", "Date.now()"),
    ("Math", "random", "Math.random()"),
    ("performance", "now", "performance.now()"),
    ("crypto", "randomUUID", "crypto.randomUUID()"),
    ("crypto", "getRandomValues", "crypto.getRandomValues()"),
    ("process", "hrtime", "process.hrtime()"),
];

pub fn build(_options: &[Value]) -> Result<Box<dyn Rule>, String> {
    Ok(Box::new(NoAmbientNondeterminism))
}

struct NoAmbientNondeterminism;

impl Rule for NoAmbientNondeterminism {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        match node.kind() {
            AstKind::CallExpression(call) => {
                if matches!(call.callee, Expression::Super(_)) {
                    return;
                }

                if let Some((_, _, source)) = AMBIENT_MEMBERS.iter().find(|(owner, property, _)| is_global_member(ctx, &call.callee, owner, property)) {
                    ctx.report(call.span, format!("`{source}` makes this output depend on when and where it ran. Take the value as a parameter so the caller owns it."));
                }
            }
            AstKind::NewExpression(new) if new.arguments.is_empty() && is_global(ctx, &new.callee, "Date") => {
                ctx.report(new.span, "`new Date()` without an argument reads the ambient clock. Take the instant as a parameter so the caller owns it.");
            }
            _ => {}
        }
    }
}
