//! `anti-slop/no-shape-in-symbol-names`: no case-insensitive "shape" in any
//! JavaScript, TypeScript, private or JSX symbol name.

use oxc_ast::AstKind;
use oxc_ast::ast::{BindingIdentifier, BindingPattern, TSTypeAnnotation};
use oxc_semantic::AstNode;
use oxc_span::Span;
use serde_json::Value;

use super::super::{Context, Rule};

pub const NAME: &str = "anti-slop/no-shape-in-symbol-names";

pub fn build(_options: &[Value]) -> Result<Box<dyn Rule>, String> {
    Ok(Box::new(NoShapeInSymbolNames))
}

struct NoShapeInSymbolNames;

impl Rule for NoShapeInSymbolNames {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let (name, span) = match node.kind() {
            AstKind::IdentifierName(it) => (it.name.as_str(), it.span),
            AstKind::IdentifierReference(it) => (it.name.as_str(), it.span),
            AstKind::BindingIdentifier(it) => (it.name.as_str(), binding_span(ctx, node, it)),
            AstKind::LabelIdentifier(it) => (it.name.as_str(), it.span),
            AstKind::PrivateIdentifier(it) => (it.name.as_str(), it.span),
            AstKind::JSXIdentifier(it) => (it.name.as_str(), it.span),
            AstKind::TSIndexSignatureName(it) => (it.name.as_str(), it.span),
            // ESTree holds the shorthand target twice, as the key and as the value.
            AstKind::AssignmentTargetPropertyIdentifier(it) => (it.binding.name.as_str(), it.binding.span),
            _ => return,
        };

        if name.to_lowercase().contains("shape") {
            ctx.report(span, format!("Rename symbol \"{name}\" for its domain role; \"shape\" describes structure rather than ownership."));
        }
    }
}

/// ESTree hangs a declared name's type annotation on its identifier, so the
/// identifier's range runs to the end of the annotation.
fn binding_span(ctx: &Context<'_, '_>, node: &AstNode<'_>, identifier: &BindingIdentifier<'_>) -> Span {
    let is_this = |pattern: &BindingPattern<'_>| matches!(pattern, BindingPattern::BindingIdentifier(it) if it.span == identifier.span);
    let through = |annotation: Option<&TSTypeAnnotation<'_>>| annotation.map_or(identifier.span, |annotation| Span::new(identifier.span.start, annotation.span.end));

    match ctx.semantic.nodes().parent_kind(node.id()) {
        AstKind::VariableDeclarator(it) if is_this(&it.id) => through(it.type_annotation.as_deref()),
        AstKind::CatchParameter(it) if is_this(&it.pattern) => through(it.type_annotation.as_deref()),
        AstKind::FormalParameter(it) if is_this(&it.pattern) => match it.type_annotation.as_deref() {
            None if it.optional && it.initializer.is_none() => Span::new(identifier.span.start, it.span.end),
            annotation => through(annotation),
        },
        _ => identifier.span,
    }
}
