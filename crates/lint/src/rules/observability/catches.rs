//! Evidence that a catch reports or propagates its own caught value.

use oxc_ast::AstKind;
use oxc_ast::ast::{BindingPattern, CatchClause, Expression, PropertyKey};
use oxc_ast_visit::Visit;
use oxc_semantic::Semantic;
use oxc_syntax::operator::AssignmentOperator;
use oxc_syntax::symbol::SymbolId;

use super::options::Observer;
use super::shared::Contains;

pub fn handled<'a>(clause: &CatchClause<'a>, observers: &[Observer], semantic: &Semantic<'a>) -> bool {
    let caught = clause.param.as_ref().and_then(|param| match &param.pattern {
        BindingPattern::BindingIdentifier(id) => id.symbol_id.get(),
        _ => None,
    });
    let mut evidence = Contains::new(|node| match node {
        AstKind::ThrowStatement(_) => true,
        AstKind::CallExpression(call) => observers.iter().any(|observer| observer.matches(&call.callee)),
        _ => caught.is_some_and(|caught| propagates(node, caught, semantic)),
    });

    evidence.visit_block_statement(&clause.body);

    evidence.found
}

/// The caught value is the catch binding itself, not a same-named shadow.
fn propagates<'a>(node: AstKind<'a>, caught: SymbolId, semantic: &Semantic<'a>) -> bool {
    let value = match node {
        AstKind::ReturnStatement(statement) => statement.argument.as_ref(),
        AstKind::VariableDeclarator(declaration) => declaration.init.as_ref(),
        AstKind::AssignmentExpression(assignment) if assignment.operator == AssignmentOperator::Assign => Some(&assignment.right),
        _ => None,
    };
    let Some(value) = value else { return false };
    let mut reference = Contains::new(|node| matches!(node, AstKind::IdentifierReference(id) if resolves(id, semantic) == Some(caught)));
    let mut error_class = Contains::new(|node| match node {
        AstKind::ObjectProperty(property) if !property.computed => match &property.key {
            PropertyKey::StaticIdentifier(key) => key.name == "errorClass",
            PropertyKey::StringLiteral(key) => key.value == "errorClass",
            _ => false,
        },
        _ => false,
    });

    reference.visit_expression(value);
    error_class.visit_expression(value);

    reference.found && error_class.found
}

fn resolves<'a>(id: &oxc_ast::ast::IdentifierReference<'a>, semantic: &Semantic<'a>) -> Option<SymbolId> {
    semantic.scoping().get_reference(id.reference_id.get()?).symbol_id()
}

/// An explicit undefined result, including either branch of a conditional.
/// A local binding named `undefined` is a value, not the global.
pub fn returns_undefined<'a>(expression: Option<&Expression<'a>>, semantic: &Semantic<'a>) -> bool {
    let Some(expression) = expression else { return true };
    let mut pending = vec![expression];

    while let Some(expression) = pending.pop() {
        match super::shared::unwrap(expression) {
            Expression::Identifier(id) if id.name == "undefined" && resolves(id, semantic).is_none() => return true,
            Expression::UnaryExpression(unary) if unary.operator == oxc_syntax::operator::UnaryOperator::Void => return true,
            Expression::ConditionalExpression(conditional) => {
                pending.push(&conditional.consequent);
                pending.push(&conditional.alternate);
            }
            _ => {}
        }
    }

    false
}
