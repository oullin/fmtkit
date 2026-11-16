//! `anti-slop/no-conditional-spread`: no spreading a ternary or a logical
//! expression (`&&`, `||`, `??`).
//!
//! Reports the spread in every position (object and array literals, call
//! arguments, JSX spread attributes), looking through parentheses only:
//! `...((a ? b : c) as T)` passes. Unlike
//! `anti-slop/no-conditional-empty-object-spread`, the branches need not be
//! empty. No options.

use oxc_ast::AstKind;
use oxc_ast::ast::Expression;
use oxc_semantic::AstNode;
use oxc_span::Span;

use super::super::{Context, Rule};
use super::shared::strip_parens;

pub const NAME: &str = "anti-slop/no-conditional-spread";

pub struct NoConditionalSpread;

impl Rule for NoConditionalSpread {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let (span, argument): (Span, &Expression<'_>) = match node.kind() {
            AstKind::SpreadElement(spread) => (spread.span, &spread.argument),
            AstKind::JSXSpreadAttribute(spread) => (spread.span, &spread.argument),
            _ => return,
        };

        if matches!(strip_parens(argument), Expression::ConditionalExpression(_) | Expression::LogicalExpression(_)) {
            ctx.report(span, "Conditional spreads obscure which values or properties are present. Build the value in separate statements instead.");
        }
    }
}
