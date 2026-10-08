//! `anti-slop/no-reflect-apply`: no `Reflect.apply`; call typed functions directly or model dynamic dispatch behind an interface.

use oxc_ast::AstKind;
use oxc_semantic::AstNode;
use serde_json::Value;

use super::super::{Context, Rule};
use super::shared::is_global_member;

pub const NAME: &str = "anti-slop/no-reflect-apply";

pub fn build(_options: &[Value]) -> Result<Box<dyn Rule>, String> {
    Ok(Box::new(NoReflectApply))
}

struct NoReflectApply;

impl Rule for NoReflectApply {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        if let AstKind::CallExpression(call) = node.kind()
            && is_global_member(ctx, &call.callee, "Reflect", "apply")
        {
            ctx.report(call.span, "Replace `Reflect.apply` with a typed function call. Model dynamic dispatch behind a named interface.");
        }
    }
}
