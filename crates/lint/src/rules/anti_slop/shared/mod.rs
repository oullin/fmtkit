//! Helpers shared by the anti-slop rules.
//!
//! The v1 rules read oxlint's ESTree, which has no parentheses (expression or
//! type) and no `FormalParameters` node. These helpers hide the same nodes so
//! the ported logic sees the tree v1 saw.

pub mod lexical;
pub mod params;
pub mod types;

use oxc_ast::AstKind;
use oxc_ast::ast::{Comment, Expression, FunctionType, TSType};
use oxc_semantic::{AstNode, AstNodes, NodeId, Semantic};

use super::super::Context;

/// `expression` without its parentheses.
pub fn strip_parens<'b, 'a>(mut expression: &'b Expression<'a>) -> &'b Expression<'a> {
    while let Expression::ParenthesizedExpression(inner) = expression {
        expression = &inner.expression;
    }

    expression
}

/// `ty` without its parentheses.
pub fn strip_type_parens<'b, 'a>(mut ty: &'b TSType<'a>) -> &'b TSType<'a> {
    while let TSType::TSParenthesizedType(inner) = ty {
        ty = &inner.type_annotation;
    }

    ty
}

/// Whether ESTree has no node for this kind.
fn is_hidden(kind: AstKind<'_>) -> bool {
    matches!(kind, AstKind::ParenthesizedExpression(_) | AstKind::TSParenthesizedType(_) | AstKind::FormalParameters(_))
}

/// The ancestors of `id` that ESTree has, nearest first, ending at `Program`.
pub fn ancestors<'s, 'a>(nodes: &'s AstNodes<'a>, id: NodeId) -> impl Iterator<Item = &'s AstNode<'a>> + 's {
    nodes.ancestors(id).filter(|node| !is_hidden(node.kind()))
}

/// The parent of `id` as ESTree has it.
pub fn parent<'s, 'a>(nodes: &'s AstNodes<'a>, id: NodeId) -> &'s AstNode<'a> {
    ancestors(nodes, id).next().unwrap_or_else(|| nodes.get_node(id))
}

/// The nearest function (declaration, expression or arrow) enclosing `id`.
/// With `bodyless`, declared and abstract functions count too.
pub fn enclosing_function<'s, 'a>(nodes: &'s AstNodes<'a>, id: NodeId, bodyless: bool) -> Option<&'s AstNode<'a>> {
    ancestors(nodes, id).find(|node| match node.kind() {
        AstKind::ArrowFunctionExpression(_) => true,
        AstKind::Function(function) => bodyless || matches!(function.r#type, FunctionType::FunctionDeclaration | FunctionType::FunctionExpression),
        _ => false,
    })
}

/// The globals of oxlint's default `builtin` environment that the rules name.
/// A value reference to one of them skips type-only declarations of the name.
const BUILTIN_GLOBALS: [&str; 3] = ["Date", "Math", "Reflect"];

/// Whether `expression` is the identifier `name` naming the global, as v1's
/// `isGlobalNamed`: a value reference to a builtin global, or a name with no
/// declaration in scope, type-only declarations included.
pub fn is_global(ctx: &Context<'_, '_>, expression: &Expression<'_>, name: &str) -> bool {
    let Expression::Identifier(identifier) = strip_parens(expression) else {
        return false;
    };

    if identifier.name != name {
        return false;
    }

    let scoping = ctx.semantic.scoping();

    if BUILTIN_GLOBALS.contains(&name) && identifier.reference_id.get().is_none_or(|reference| scoping.get_reference(reference).symbol_id().is_none()) {
        return true;
    }

    let scope = ctx.semantic.nodes().get_node(identifier.node_id.get()).scope_id();

    scoping.find_binding(scope, identifier.name).is_none()
}

/// The object and the statically known property name of a member access:
/// `a.b` or `a["b"]`, but not `a[b]` or `a.#b`.
pub fn member<'b, 'a>(expression: &'b Expression<'a>) -> Option<(&'b Expression<'a>, &'a str)> {
    match strip_parens(expression) {
        Expression::StaticMemberExpression(member) => Some((&member.object, member.property.name.as_str())),
        Expression::ComputedMemberExpression(member) => match strip_parens(&member.expression) {
            Expression::StringLiteral(key) => Some((&member.object, key.value.as_str())),
            _ => None,
        },
        _ => None,
    }
}

/// Whether `callee` reads `property` off the unshadowed global `owner`.
pub fn is_global_member(ctx: &Context<'_, '_>, callee: &Expression<'_>, owner: &str, property: &str) -> bool {
    member(callee).is_some_and(|(object, name)| name == property && is_global(ctx, object, owner))
}

/// JavaScript's `\s`: whitespace and line terminators.
pub fn is_js_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}'
    )
}

/// The comments directly before `position`: after the preceding token, with
/// nothing but whitespace around them, as oxlint's `getCommentsBefore`.
pub fn comments_before<'s>(semantic: &'s Semantic<'_>, position: u32) -> &'s [Comment] {
    let comments = semantic.comments();
    let source = semantic.source_text();
    let last = comments.partition_point(|comment| comment.span.end <= position);
    let mut first = last;
    let mut end = position;

    while first > 0 {
        let comment = &comments[first - 1];

        if !source[comment.span.end as usize..end as usize].chars().all(is_js_space) {
            break;
        }

        first -= 1;
        end = comment.span.start;
    }

    &comments[first..last]
}

/// The text of a comment without its delimiters, as ESTree's `Comment.value`.
pub fn comment_value<'a>(source: &'a str, comment: &Comment) -> &'a str {
    let span = comment.content_span();

    &source[span.start as usize..span.end as usize]
}
