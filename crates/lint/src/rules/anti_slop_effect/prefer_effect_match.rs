//! `anti-slop-effect/prefer-effect-match`: no chain of two or more ternaries
//! that each compare the same value with a literal; use `Match` from Effect.
//!
//! The outermost ternary of a chain is reported. Every test in the chain must
//! be an equality comparison (`==`, `===`, `!=`, `!==`) of a literal with a
//! value of the same source text; one other test ends the check. Takes no
//! options.

use oxc_ast::AstKind;
use oxc_ast::ast::Expression;
use oxc_semantic::AstNode;
use oxc_span::GetSpan;

use super::super::anti_slop::shared::strip_parens;
use super::super::{Context, Rule};
use super::shared::effect_modules::estree_parent;

pub const NAME: &str = "anti-slop-effect/prefer-effect-match";

const MESSAGE: &str = "Use Match from Effect instead of a chained literal ternary.";

pub struct PreferEffectMatch;

impl Rule for PreferEffectMatch {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let AstKind::ConditionalExpression(conditional) = node.kind() else {
            return;
        };

        if matches!(estree_parent(ctx.semantic.nodes(), node.id()).map(AstNode::kind), Some(AstKind::ConditionalExpression(_))) {
            return;
        }

        let source = ctx.source();
        let Some(value) = compared_value(source, &conditional.test) else {
            return;
        };
        let mut alternate = strip_parens(&conditional.alternate);
        let mut literal_checks = 1;

        while let Expression::ConditionalExpression(inner) = alternate {
            if compared_value(source, &inner.test) != Some(value) {
                return;
            }

            literal_checks += 1;
            alternate = strip_parens(&inner.alternate);
        }

        if literal_checks > 1 {
            ctx.report(conditional.span, MESSAGE);
        }
    }
}

/// ESTree's `Literal`, or a template literal without expressions.
fn is_literal(expression: &Expression<'_>) -> bool {
    match strip_parens(expression) {
        Expression::StringLiteral(_)
        | Expression::NumericLiteral(_)
        | Expression::BigIntLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::RegExpLiteral(_) => true,
        Expression::TemplateLiteral(template) => template.expressions.is_empty(),
        _ => false,
    }
}

/// The source text of the value an equality test compares with a literal.
fn compared_value<'a>(source: &'a str, test: &Expression<'_>) -> Option<&'a str> {
    let Expression::BinaryExpression(binary) = strip_parens(test) else {
        return None;
    };

    if !binary.operator.is_equality() {
        return None;
    }

    let value = if is_literal(&binary.left) {
        &binary.right
    } else if is_literal(&binary.right) {
        &binary.left
    } else {
        return None;
    };

    source.get(strip_parens(value).span().start as usize..strip_parens(value).span().end as usize)
}
