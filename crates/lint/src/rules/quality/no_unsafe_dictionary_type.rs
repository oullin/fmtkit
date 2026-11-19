//! `quality/no-unsafe-dictionary-type`: no dictionary whose value type is
//! `unknown`, `any`, `object`, `{}`, or a union or alias holding one of them.
//!
//! Unlike the k-mistele fork's `no-unsafe-dictionary-type`, names resolve lexically; a
//! function named like a utility type (`Omit`) does not shadow it, while a
//! type parameter does; and a dictionary in a type parameter's constraint
//! is allowed.

use oxc_ast::AstKind;
use oxc_semantic::AstNode;
use oxc_span::{GetSpan, Span};

use super::super::anti_slop::shared::types::{UnsafeValue, reference_name, type_arguments};
use super::super::{Context, Rule};
use super::bindings::{Scoped, TypeBindings};
use super::dictionary::{classify_unsafe_dictionary, classify_unsafe_value};

pub const NAME: &str = "quality/no-unsafe-dictionary-type";

pub struct NoUnsafeDictionaryType;

impl Rule for NoUnsafeDictionaryType {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let kind = node.kind();

        if !matches!(kind, AstKind::TSTypeReference(_) | AstKind::TSTypeLiteral(_) | AstKind::TSMappedType(_) | AstKind::TSIndexSignature(_)) {
            return;
        }

        let semantic = ctx.semantic;
        let bindings = TypeBindings::of(ctx);
        let scoped = bindings.with(semantic.nodes());
        let unsafe_value = if let AstKind::TSIndexSignature(signature) = kind {
            // Index signatures of type literals are judged with their literal.
            if matches!(semantic.nodes().parent_kind(node.id()), AstKind::TSTypeLiteral(_)) {
                return;
            }

            classify_unsafe_value(scoped, &signature.type_annotation.type_annotation)
        } else {
            classify_unsafe_dictionary(scoped, kind).filter(|_| !is_suppressed(scoped, node))
        };

        if let Some(value) = unsafe_value {
            report(ctx, kind.span(), value);
        }
    }
}

fn report(ctx: &mut Context<'_, '_>, span: Span, value: UnsafeValue) {
    let value = value.as_str();

    ctx.report(
        span,
        format!("This dictionary's {value} value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion."),
    );
}

/// Whether an unsafe dictionary type is reported elsewhere or allowed: in a
/// type parameter's constraint, as a plain use of an alias (judged at the
/// alias), or inside a larger unsafe dictionary.
fn is_suppressed<'a>(scoped: Scoped<'_, 'a>, node: &AstNode<'a>) -> bool {
    let nodes = scoped.nodes();
    let span = node.kind().span();
    let mut ancestors = nodes.ancestors(node.id()).take_while(|ancestor| !matches!(ancestor.kind(), AstKind::Program(_)));

    if ancestors.clone().any(|ancestor| matches!(ancestor.kind(), AstKind::TSTypeParameter(parameter) if parameter.constraint.as_ref().is_some_and(|constraint| constraint.span().contains_inclusive(span))))
    {
        return true;
    }

    if let AstKind::TSTypeReference(reference) = node.kind()
        && type_arguments(reference).is_empty()
        && reference_name(reference).is_some_and(|name| scoped.alias(name, node.id()).is_some())
        && !ancestors.clone().any(|ancestor| matches!(ancestor.kind(), AstKind::TSTypeAliasDeclaration(_)))
    {
        return true;
    }

    ancestors.any(|ancestor| classify_unsafe_dictionary(scoped, ancestor.kind()).is_some())
}
