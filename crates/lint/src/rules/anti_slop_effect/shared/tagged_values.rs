//! Upstream's `effect/shared/tagged-values.ts`: `_tag` reads, comparisons and
//! keys, and whether code runs inside a broad `Effect.catch*` handler.

use oxc_ast::AstKind;
use oxc_ast::ast::{BinaryExpression, CallExpression, Expression, FunctionType, ObjectProperty, PropertyKey};
use oxc_semantic::{AstNodes, NodeId};
use oxc_span::GetSpan;

use super::super::super::anti_slop::shared::strip_parens;
use super::effect_modules::{estree_parent, has_argument};

/// The `Effect` methods that catch every error.
const BROAD_CATCH_METHODS: [&str; 3] = ["catch", "catchAll", "catchIf"];

/// The object of `<object>.<name>` or `<object>["<name>"]`.
fn named_member_object<'b, 'a>(expression: &'b Expression<'a>, name: &str) -> Option<&'b Expression<'a>> {
    match strip_parens(expression) {
        Expression::StaticMemberExpression(member) if member.property.name == name => Some(&member.object),
        Expression::ComputedMemberExpression(member) if matches!(strip_parens(&member.expression), Expression::StringLiteral(key) if key.value == name) => {
            Some(&member.object)
        }
        _ => None,
    }
}

/// The object of a `_tag` read: `value._tag` or `value["_tag"]`.
pub fn tag_member_object<'b, 'a>(expression: &'b Expression<'a>) -> Option<&'b Expression<'a>> {
    named_member_object(expression, "_tag")
}

/// The object of the `_tag` read an equality comparison with a string literal makes.
pub fn tag_member_from_comparison<'b, 'a>(binary: &'b BinaryExpression<'a>) -> Option<&'b Expression<'a>> {
    if !binary.operator.is_equality() {
        return None;
    }

    let is_string = |expression: &Expression<'_>| matches!(strip_parens(expression), Expression::StringLiteral(_));

    if let Some(object) = tag_member_object(&binary.left)
        && is_string(&binary.right)
    {
        return Some(object);
    }

    tag_member_object(&binary.right).filter(|_| is_string(&binary.left))
}

/// Whether the `_tag` read on `object` reads a `reason`'s tag: `error.reason._tag`.
pub fn is_reason_tag(object: &Expression<'_>) -> bool {
    named_member_object(object, "reason").is_some()
}

/// `Effect.catch(...)`, `Effect.catchAll(...)` or `Effect.catchIf(...)`, by the
/// literal name `Effect`.
fn is_broad_catch_call(call: &CallExpression<'_>) -> bool {
    match strip_parens(&call.callee) {
        Expression::StaticMemberExpression(member) => {
            matches!(strip_parens(&member.object), Expression::Identifier(object) if object.name == "Effect")
                && BROAD_CATCH_METHODS.contains(&member.property.name.as_str())
        }
        _ => false,
    }
}

/// Whether the nearest enclosing arrow or function expression is an argument
/// of a broad `Effect` catch call.
pub fn is_inside_broad_effect_handler(nodes: &AstNodes<'_>, id: NodeId) -> bool {
    let mut current = id;

    while let Some(node) = estree_parent(nodes, current) {
        let is_handler = match node.kind() {
            AstKind::ArrowFunctionExpression(_) => true,
            AstKind::Function(function) => function.r#type == FunctionType::FunctionExpression,
            _ => false,
        };

        if is_handler {
            let span = node.kind().span();

            return match estree_parent(nodes, node.id()).map(oxc_semantic::AstNode::kind) {
                Some(AstKind::CallExpression(call)) => is_broad_catch_call(call) && has_argument(call, span),
                _ => false,
            };
        }

        current = node.id();
    }

    false
}

/// An object property's name, as upstream's `propertyName`: a non-computed
/// identifier key, or a string literal key, computed or not.
pub fn property_name<'a>(property: &ObjectProperty<'a>) -> Option<&'a str> {
    match &property.key {
        PropertyKey::StaticIdentifier(identifier) if !property.computed => Some(identifier.name.as_str()),
        key => match key.as_expression().map(strip_parens) {
            Some(Expression::StringLiteral(literal)) => literal.value.as_str(),
            _ => None,
        },
    }
}

/// Whether a class property or type member key is `_tag`: an identifier,
/// computed or not, or a string literal.
pub fn is_tag_key(key: &PropertyKey<'_>) -> bool {
    match key {
        PropertyKey::StaticIdentifier(identifier) => identifier.name == "_tag",
        PropertyKey::PrivateIdentifier(_) => false,
        key => match key.as_expression().map(strip_parens) {
            Some(Expression::Identifier(identifier)) => identifier.name == "_tag",
            Some(Expression::StringLiteral(literal)) => literal.value == "_tag",
            _ => false,
        },
    }
}
