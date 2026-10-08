//! `anti-slop/no-module-mocking`: no Vitest or Jest module mocking; tests
//! replace dependencies through real interfaces.

use oxc_ast::AstKind;
use oxc_ast::ast::Expression;
use oxc_semantic::AstNode;

use super::super::{Context, Rule};
use super::shared::{member, strip_parens};

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

/// Whether `expression` names the global `vi` or `jest`, or `vi` imported from
/// `vitest` or `jest` from `@jest/globals` under any local name.
fn is_test_framework_object(ctx: &Context<'_, '_>, expression: &Expression<'_>) -> bool {
    let Expression::Identifier(identifier) = strip_parens(expression) else {
        return false;
    };

    let nodes = ctx.semantic.nodes();
    let scoping = ctx.semantic.scoping();
    let Some(symbol) = scoping.find_binding(nodes.get_node(identifier.node_id.get()).scope_id(), identifier.name) else {
        return identifier.name == "vi" || identifier.name == "jest";
    };

    let declaration = scoping.symbol_declaration(symbol);
    let AstKind::ImportSpecifier(specifier) = nodes.kind(declaration) else {
        return false;
    };

    let imported = specifier.imported.name();

    nodes
        .ancestor_kinds(declaration)
        .find_map(|kind| if let AstKind::ImportDeclaration(import) = kind { Some(import.source.value.as_str()) } else { None })
        .is_some_and(|source| (source == "vitest" && imported == "vi") || (source == "@jest/globals" && imported == "jest"))
}
