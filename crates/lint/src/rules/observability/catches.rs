//! Evidence that a catch reports or propagates its own caught value.

use oxc_ast::AstKind;
use oxc_ast::ast::{BindingPattern, CatchClause, Expression, PropertyKey};
use oxc_ast_visit::Visit;
use oxc_syntax::operator::AssignmentOperator;

use super::options::Observer;
use super::shared::Contains;

pub fn handled(clause: &CatchClause<'_>, observers: &[Observer]) -> bool {
    let caught = clause.param.as_ref().and_then(|param| match &param.pattern {
        BindingPattern::BindingIdentifier(id) => Some(id.name.as_str()),
        _ => None,
    });
    let mut evidence = Contains::new(|node| match node {
        AstKind::ThrowStatement(_) => true,
        AstKind::CallExpression(call) => observers.iter().any(|observer| observer.matches(&call.callee)),
        _ => caught.is_some_and(|caught| propagates(node, caught)),
    });

    evidence.visit_block_statement(&clause.body);

    evidence.found
}

fn propagates(node: AstKind<'_>, caught: &str) -> bool {
    let value = match node {
        AstKind::ReturnStatement(statement) => statement.argument.as_ref(),
        AstKind::VariableDeclarator(declaration) => declaration.init.as_ref(),
        AstKind::AssignmentExpression(assignment) if assignment.operator == AssignmentOperator::Assign => Some(&assignment.right),
        _ => None,
    };
    let Some(value) = value else { return false };
    let mut reference = Contains::new(|node| matches!(node, AstKind::IdentifierReference(id) if id.name == caught));
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

/// An explicit undefined result, including either branch of a conditional.
pub fn returns_undefined(expression: Option<&Expression<'_>>) -> bool {
    let Some(expression) = expression else { return true };
    let mut pending = vec![expression];

    while let Some(expression) = pending.pop() {
        match super::shared::unwrap(expression) {
            Expression::Identifier(id) if id.name == "undefined" => return true,
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
