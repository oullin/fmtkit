//! `quality/no-reflect-apply`: no `Reflect.apply`; call typed functions directly or model dynamic dispatch behind an interface.

use oxc_ast::AstKind;
use oxc_semantic::AstNode;

use super::super::anti_slop::shared::is_global_member;
use super::super::{Context, Rule};

pub const NAME: &str = "quality/no-reflect-apply";

pub struct NoReflectApply;

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
