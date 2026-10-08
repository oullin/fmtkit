//! `anti-slop/no-chained-type-assertions`: no nested `as` or angle-bracket
//! assertions, parenthesized or not, unless every link is `as const`.

use oxc_ast::AstKind;
use oxc_ast::ast::{Expression, TSType, TSTypeName};
use oxc_semantic::AstNode;
use oxc_span::{GetSpan, Span};
use serde_json::Value;

use super::super::{Context, Rule};
use super::shared::{parent, strip_parens};

pub const NAME: &str = "anti-slop/no-chained-type-assertions";

pub fn build(_options: &[Value]) -> Result<Box<dyn Rule>, String> {
    Ok(Box::new(NoChainedTypeAssertions))
}

struct NoChainedTypeAssertions;

impl Rule for NoChainedTypeAssertions {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let Some((span, _, _)) = assertion(node.kind()) else {
            return;
        };

        let outermost = assertion(parent(ctx.semantic.nodes(), node.id()).kind()).is_none_or(|(_, inner, _)| strip_parens(inner).span() != span);

        if outermost && is_forbidden_chain(node.kind()) {
            ctx.report(span, "This assertion chain discards type evidence. Keep the original precise type, or parse untrusted input at its boundary before narrowing it.");
        }
    }
}

/// The span, operand and target type of an `as` or angle-bracket assertion.
fn assertion<'a>(kind: AstKind<'a>) -> Option<(Span, &'a Expression<'a>, &'a TSType<'a>)> {
    match kind {
        AstKind::TSAsExpression(it) => Some((it.span, &it.expression, &it.type_annotation)),
        AstKind::TSTypeAssertion(it) => Some((it.span, &it.expression, &it.type_annotation)),
        _ => None,
    }
}

fn assertion_expression<'a>(expression: &'a Expression<'a>) -> Option<(&'a Expression<'a>, &'a TSType<'a>)> {
    match expression {
        Expression::TSAsExpression(it) => Some((&it.expression, &it.type_annotation)),
        Expression::TSTypeAssertion(it) => Some((&it.expression, &it.type_annotation)),
        _ => None,
    }
}

fn is_const(ty: &TSType<'_>) -> bool {
    matches!(ty, TSType::TSTypeReference(reference) if matches!(&reference.type_name, TSTypeName::IdentifierReference(name) if name.name == "const"))
}

/// Whether the chain starting at an outermost assertion has more than one link
/// and at least one that is not `as const`.
fn is_forbidden_chain(kind: AstKind<'_>) -> bool {
    let Some((_, mut expression, ty)) = assertion(kind) else {
        return false;
    };

    let mut count = 1;
    let mut non_const = !is_const(ty);

    while let Some((inner, ty)) = assertion_expression(strip_parens(expression)) {
        count += 1;
        non_const |= !is_const(ty);
        expression = inner;
    }

    count > 1 && non_const
}
