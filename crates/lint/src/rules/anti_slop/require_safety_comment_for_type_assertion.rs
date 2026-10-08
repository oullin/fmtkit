//! `anti-slop/require-safety-comment-for-type-assertion`: every type assertion
//! but `as const` states its invariant in a nearby `SAFETY:` comment.

use oxc_ast::AstKind;
use oxc_ast::ast::{TSType, TSTypeName};
use oxc_semantic::AstNode;
use oxc_span::{GetSpan, Span};

use super::super::{Context, Rule};
use super::shared::{comment_value, comments_before, is_js_space, parent};

pub const NAME: &str = "anti-slop/require-safety-comment-for-type-assertion";

pub struct RequireSafetyCommentForTypeAssertion;

impl Rule for RequireSafetyCommentForTypeAssertion {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let (span, ty) = match node.kind() {
            AstKind::TSAsExpression(it) => (it.span, &it.type_annotation),
            AstKind::TSTypeAssertion(it) => (it.span, &it.type_annotation),
            _ => return,
        };

        if is_const(ty) || has_safety_comment(ctx, node, span) {
            return;
        }

        ctx.report(
            span,
            "This type assertion has no `SAFETY:` justification. State the checked invariant immediately before the assertion or its containing statement.",
        );
    }
}

fn is_const(ty: &TSType<'_>) -> bool {
    matches!(ty, TSType::TSTypeReference(reference) if matches!(&reference.type_name, TSTypeName::IdentifierReference(name) if name.name == "const"))
}

/// Whether a `SAFETY:` comment sits directly before the assertion or before one
/// of its ancestors, up to the statement or class field that holds it.
fn has_safety_comment(ctx: &Context<'_, '_>, node: &AstNode<'_>, assertion: Span) -> bool {
    let semantic = ctx.semantic;
    let nodes = semantic.nodes();
    let source = semantic.source_text();
    let mut current = node;

    loop {
        let start = current.kind().span().start;

        if comments_before(semantic, start).iter().any(|comment| comment.span.end <= assertion.start && mentions_safety(comment_value(source, comment))) {
            return true;
        }

        let next = parent(nodes, current.id());

        if is_comment_owner(current.kind()) || matches!(next.kind(), AstKind::Program(_)) || next.id() == current.id() {
            return false;
        }

        current = next;
    }
}

fn is_comment_owner(kind: AstKind<'_>) -> bool {
    matches!(
        kind,
        AstKind::ExpressionStatement(_)
            | AstKind::PropertyDefinition(_)
            | AstKind::ReturnStatement(_)
            | AstKind::ThrowStatement(_)
            | AstKind::VariableDeclaration(_)
    )
}

/// `/\bSAFETY\s*:/u`.
fn mentions_safety(text: &str) -> bool {
    text.match_indices("SAFETY").any(|(index, word)| {
        let bounded = !text[..index].chars().next_back().is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');

        bounded && text[index + word.len()..].trim_start_matches(is_js_space).starts_with(':')
    })
}
