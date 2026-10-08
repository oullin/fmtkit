//! `anti-slop/no-known-value-widening`: no syntactically known value flowing
//! into an explicitly broad or anonymous target type.

use oxc_ast::AstKind;
use oxc_ast::ast::{
    AccessorPropertyType, AssignmentOperator, AssignmentTarget, BindingPattern, Expression, MethodDefinitionType, PropertyDefinitionType, PropertyKey, TSType,
    TSTypeAnnotation, VariableDeclarationKind, VariableDeclarator,
};
use oxc_semantic::{AstNode, Semantic, SymbolId};
use oxc_span::GetSpan;
use oxc_syntax::number::ToJsString;

use super::super::{Context, Rule};
use super::shared::types::{TypeEnvironment, WideningTarget};
use super::shared::{enclosing_function, parent, strip_parens};

pub const NAME: &str = "anti-slop/no-known-value-widening";

pub struct NoKnownValueWidening;

impl Rule for NoKnownValueWidening {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let semantic = ctx.semantic;
        let source = semantic.source_text();
        let nodes = semantic.nodes();

        let (expression, target, subject): (&Expression<'a>, Option<&'a TSType<'a>>, String) = match node.kind() {
            AstKind::VariableDeclarator(declarator) => {
                let (Some(init), BindingPattern::BindingIdentifier(id)) = (&declarator.init, &declarator.id) else {
                    return;
                };

                (init, annotated(declarator.type_annotation.as_deref()), format!("binding `{}`", id.name))
            }
            AstKind::PropertyDefinition(property) if property.r#type == PropertyDefinitionType::PropertyDefinition => {
                let Some(value) = &property.value else {
                    return;
                };

                (value, annotated(property.type_annotation.as_deref()), format!("property `{}`", key_name(source, &property.key)))
            }
            AstKind::AccessorProperty(property) if property.r#type == AccessorPropertyType::AccessorProperty => {
                let Some(value) = &property.value else {
                    return;
                };

                (value, annotated(property.type_annotation.as_deref()), format!("property `{}`", key_name(source, &property.key)))
            }
            AstKind::AssignmentExpression(assignment) if assignment.operator == AssignmentOperator::Assign => {
                let AssignmentTarget::AssignmentTargetIdentifier(left) = &assignment.left else {
                    return;
                };

                let Some(symbol) = semantic.scoping().find_binding(nodes.get_node(left.node_id.get()).scope_id(), left.name) else {
                    return;
                };

                let Some(declarator) = only_declarator(semantic, symbol) else {
                    return;
                };

                let BindingPattern::BindingIdentifier(id) = &declarator.id else {
                    return;
                };

                (&assignment.right, annotated(declarator.type_annotation.as_deref()), format!("binding `{}`", id.name))
            }
            AstKind::ReturnStatement(statement) => {
                let Some(argument) = &statement.argument else {
                    return;
                };

                let owner = enclosing_function(nodes, node.id(), false);
                let return_type = owner.and_then(|owner| match owner.kind() {
                    AstKind::Function(function) => function.return_type.as_deref(),
                    AstKind::ArrowFunctionExpression(arrow) => arrow.return_type.as_deref(),
                    _ => None,
                });

                (argument, annotated(return_type), format!("return value of `{}`", function_name(semantic, owner)))
            }
            AstKind::ArrowFunctionExpression(arrow) => {
                let Some(body) = arrow.body.as_expression() else {
                    return;
                };

                (body, annotated(arrow.return_type.as_deref()), format!("return value of `{}`", function_name(semantic, Some(node))))
            }
            AstKind::TSAsExpression(it) if !has_parent_assertion(semantic, node) => (&it.expression, Some(&it.type_annotation), "assertion".to_owned()),
            AstKind::TSTypeAssertion(it) if !has_parent_assertion(semantic, node) => (&it.expression, Some(&it.type_annotation), "assertion".to_owned()),
            _ => return,
        };

        let Some(target) = target else {
            return;
        };

        let env = TypeEnvironment::of(ctx);
        let Some(destination) = env.with(nodes).classify_widening_target(target) else {
            return;
        };

        if matches!(destination, WideningTarget::OpenDictionary | WideningTarget::GenericContainer) && is_empty_object(expression) {
            return;
        }

        if has_known_evidence(semantic, expression, &mut Vec::new()) {
            let target = destination.as_str();

            ctx.report(
                strip_parens(expression).span(),
                format!("The explicit {target} type on {subject} discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract."),
            );
        }
    }
}

fn annotated<'a>(annotation: Option<&'a TSTypeAnnotation<'a>>) -> Option<&'a TSType<'a>> {
    annotation.map(|annotation| &annotation.type_annotation)
}

fn has_parent_assertion(semantic: &Semantic<'_>, node: &AstNode<'_>) -> bool {
    matches!(parent(semantic.nodes(), node.id()).kind(), AstKind::TSAsExpression(_) | AstKind::TSTypeAssertion(_))
}

/// `expression` without parentheses, assertions, `satisfies` and `!`.
fn unwrap<'b, 'a>(mut expression: &'b Expression<'a>) -> &'b Expression<'a> {
    loop {
        expression = match expression {
            Expression::ParenthesizedExpression(it) => &it.expression,
            Expression::TSAsExpression(it) => &it.expression,
            Expression::TSSatisfiesExpression(it) => &it.expression,
            Expression::TSTypeAssertion(it) => &it.expression,
            Expression::TSNonNullExpression(it) => &it.expression,
            _ => return expression,
        };
    }
}

fn is_empty_object(expression: &Expression<'_>) -> bool {
    matches!(unwrap(expression), Expression::ObjectExpression(object) if object.properties.is_empty())
}

/// Whether the value of `expression` is syntactically established: a literal
/// of any kind, a function, a class or a construction.
fn is_known_evidence(expression: &Expression<'_>) -> bool {
    matches!(
        unwrap(expression),
        Expression::ObjectExpression(_)
            | Expression::ArrayExpression(_)
            | Expression::ArrowFunctionExpression(_)
            | Expression::ClassExpression(_)
            | Expression::FunctionExpression(_)
            | Expression::NewExpression(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BigIntLiteral(_)
            | Expression::RegExpLiteral(_)
            | Expression::StringLiteral(_)
            | Expression::TemplateLiteral(_)
            | Expression::UnaryExpression(_)
    )
}

/// [`is_known_evidence`], or an identifier bound once by a `const` that is
/// never written and whose initializer has known evidence.
fn has_known_evidence(semantic: &Semantic<'_>, expression: &Expression<'_>, visited: &mut Vec<SymbolId>) -> bool {
    if is_known_evidence(expression) {
        return true;
    }

    let Expression::Identifier(identifier) = unwrap(expression) else {
        return false;
    };

    let scoping = semantic.scoping();
    let Some(symbol) = scoping.find_binding(semantic.nodes().get_node(identifier.node_id.get()).scope_id(), identifier.name) else {
        return false;
    };

    if visited.contains(&symbol) {
        return false;
    }

    let Some(declarator) = only_declarator(semantic, symbol) else {
        return false;
    };

    let Some(init) = &declarator.init else {
        return false;
    };

    let is_const = matches!(semantic.nodes().parent_kind(scoping.symbol_declaration(symbol)), AstKind::VariableDeclaration(declaration) if declaration.kind == VariableDeclarationKind::Const);

    if !is_const || scoping.get_resolved_references(symbol).any(oxc_semantic::Reference::is_write) {
        return false;
    }

    visited.push(symbol);

    has_known_evidence(semantic, init, visited)
}

/// The declarator of a symbol declared exactly once, by a variable declaration.
fn only_declarator<'a>(semantic: &Semantic<'a>, symbol: SymbolId) -> Option<&'a VariableDeclarator<'a>> {
    let scoping = semantic.scoping();

    if !scoping.symbol_redeclarations(symbol).is_empty() {
        return None;
    }

    match semantic.nodes().kind(scoping.symbol_declaration(symbol)) {
        AstKind::VariableDeclarator(declarator) => Some(declarator),
        _ => None,
    }
}

/// The name v1 shows for the function that owns a return.
fn function_name<'a>(semantic: &Semantic<'a>, owner: Option<&AstNode<'a>>) -> String {
    const ANONYMOUS: &str = "anonymous function";

    let Some(owner) = owner else {
        return ANONYMOUS.to_owned();
    };

    if let AstKind::Function(function) = owner.kind()
        && let Some(id) = &function.id
    {
        return id.name.to_string();
    }

    match parent(semantic.nodes(), owner.id()).kind() {
        AstKind::VariableDeclarator(declarator) => match &declarator.id {
            BindingPattern::BindingIdentifier(id) => id.name.to_string(),
            _ => ANONYMOUS.to_owned(),
        },
        AstKind::MethodDefinition(method) if method.r#type == MethodDefinitionType::TSAbstractMethodDefinition => ANONYMOUS.to_owned(),
        AstKind::MethodDefinition(method) => key_name(semantic.source_text(), &method.key),
        _ => ANONYMOUS.to_owned(),
    }
}

/// v1's `sourceKeyName`: the name, the literal value as a string, or the source text.
fn key_name(source: &str, key: &PropertyKey<'_>) -> String {
    let expression = match key {
        PropertyKey::StaticIdentifier(it) => return it.name.to_string(),
        PropertyKey::PrivateIdentifier(it) => return it.name.to_string(),
        _ => strip_parens(key.to_expression()),
    };

    match expression {
        Expression::Identifier(it) => it.name.to_string(),
        Expression::StringLiteral(it) => it.value.to_string(),
        Expression::NumericLiteral(it) => it.value.to_js_string(),
        Expression::BigIntLiteral(it) => it.value.to_string(),
        Expression::BooleanLiteral(it) => it.value.to_string(),
        Expression::NullLiteral(_) => "null".to_owned(),
        _ => expression.span().source_text(source).to_owned(),
    }
}
