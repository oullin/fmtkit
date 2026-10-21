//! `anti-slop/no-module-mocking`: no Vitest or Jest module mocking; tests
//! replace dependencies through real interfaces.

use oxc_ast::AstKind;
use oxc_semantic::AstNode;

use super::super::{Context, Rule};
use super::shared::member;
use super::shared::testing::is_test_framework_object;

pub const NAME: &str = "anti-slop/no-module-mocking";

const MOCK_METHODS: [&str; 3] = ["doMock", "mock", "unstable_mockModule"];

pub struct NoModuleMocking;

impl Rule for NoModuleMocking {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let AstKind::CallExpression(call) = node.kind() else {
            return;
        };

        if member(&call.callee).is_some_and(|(object, method)| MOCK_METHODS.contains(&method) && is_test_framework_object(ctx, object)) {
            ctx.report(call.span, "Replace module mocking with dependency injection through a real interface, service layer, or faithful test implementation.");
        }
    }
}
