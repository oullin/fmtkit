//! `anti-slop/no-conditional-empty-object-spread`: no `...(flag ? value : {})`
//! in object literals to omit a property.

use oxc_ast::AstKind;
use oxc_ast::ast::{Expression, ObjectPropertyKind};
use oxc_semantic::AstNode;
use serde_json::Value;

use super::super::{Context, Rule};
use super::shared::strip_parens;

pub const NAME: &str = "anti-slop/no-conditional-empty-object-spread";

pub fn build(_options: &[Value]) -> Result<Box<dyn Rule>, String> {
    Ok(Box::new(NoConditionalEmptyObjectSpread))
}

struct NoConditionalEmptyObjectSpread;

impl Rule for NoConditionalEmptyObjectSpread {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let AstKind::ObjectExpression(object) = node.kind() else {
            return;
        };

        for property in &object.properties {
            if let ObjectPropertyKind::SpreadProperty(spread) = property
                && let Expression::ConditionalExpression(conditional) = strip_parens(&spread.argument)
                && (is_empty_object(&conditional.consequent) || is_empty_object(&conditional.alternate))
            {
                ctx.report(
                    spread.span,
                    "This conditional spread hides property omission behind an empty object. Build the object in separate statements and add the property only when present.",
                );
            }
        }
    }
}

fn is_empty_object(expression: &Expression<'_>) -> bool {
    matches!(strip_parens(expression), Expression::ObjectExpression(object) if object.properties.is_empty())
}
