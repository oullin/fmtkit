//! `anti-slop/no-unsafe-dictionary-type`: no dictionary whose value type is
//! `unknown`, `any`, `object`, `{}`, or a union or alias holding one of them.

use oxc_ast::AstKind;
use oxc_semantic::AstNode;
use oxc_span::Span;
use serde_json::Value;

use super::super::{Context, Rule};
use super::shared::ancestors;
use super::shared::types::{TypeEnvironment, Types, UnsafeValue, reference_name};

pub const NAME: &str = "anti-slop/no-unsafe-dictionary-type";

pub fn build(_options: &[Value]) -> Result<Box<dyn Rule>, String> {
    Ok(Box::new(NoUnsafeDictionaryType))
}

struct NoUnsafeDictionaryType;

impl Rule for NoUnsafeDictionaryType {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let kind = node.kind();
        let span = match kind {
            AstKind::TSTypeReference(it) => it.span,
            AstKind::TSTypeLiteral(it) => it.span,
            AstKind::TSMappedType(it) => it.span,
            AstKind::TSIndexSignature(it) => it.span,
            _ => return,
        };

        let semantic = ctx.semantic;
        let env = TypeEnvironment::of(ctx);
        let types = env.with(semantic.nodes());
        let unsafe_value = if let AstKind::TSIndexSignature(signature) = kind {
            // Index signatures of type literals are judged with their literal.
            if matches!(semantic.nodes().parent_kind(node.id()), AstKind::TSTypeLiteral(_)) {
                return;
            }

            types.classify_unsafe_value(&signature.type_annotation.type_annotation)
        } else {
            should_report(types, node, kind)
        };

        if let Some(value) = unsafe_value {
            report(ctx, span, value);
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

/// The unsafe value of a dictionary type, unless the type is a plain use of a
/// module alias (judged at the alias) or sits inside a larger unsafe dictionary.
fn should_report<'a>(types: Types<'_, 'a>, node: &AstNode<'a>, kind: AstKind<'a>) -> Option<UnsafeValue> {
    let nodes = types.nodes();

    if let AstKind::TSTypeReference(reference) = kind
        && reference.type_arguments.as_ref().is_none_or(|arguments| arguments.params.is_empty())
        && reference_name(reference).is_some_and(|name| types.has_alias(name))
        && !ancestors(nodes, node.id()).any(|ancestor| matches!(ancestor.kind(), AstKind::TSTypeAliasDeclaration(_)))
    {
        return None;
    }

    let value = types.classify_unsafe_dictionary_kind(kind)?;

    if ancestors(nodes, node.id()).any(|ancestor| types.classify_unsafe_dictionary_kind(ancestor.kind()).is_some()) {
        return None;
    }

    Some(value)
}
