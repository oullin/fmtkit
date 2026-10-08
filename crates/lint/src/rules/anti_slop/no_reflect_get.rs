//! `anti-slop/no-reflect-get`: no `Reflect.get`; use typed property access or parse dynamic input into a domain type.

use oxc_ast::AstKind;
use oxc_semantic::AstNode;
use serde_json::Value;

use super::super::{Context, Rule};
use super::shared::is_global_member;

pub const NAME: &str = "anti-slop/no-reflect-get";

pub fn build(_options: &[Value]) -> Result<Box<dyn Rule>, String> {
    Ok(Box::new(NoReflectGet))
}

struct NoReflectGet;

impl Rule for NoReflectGet {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        if let AstKind::CallExpression(call) = node.kind()
            && is_global_member(ctx, &call.callee, "Reflect", "get")
        {
            ctx.report(call.span, "Replace `Reflect.get` with typed property access. Parse dynamic input into a named domain type before reading it.");
        }
    }
}
