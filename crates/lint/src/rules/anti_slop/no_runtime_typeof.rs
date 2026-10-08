//! `anti-slop/no-runtime-typeof`: no runtime `typeof` checks; external values
//! are decoded into meaningful types at their I/O boundary.

use oxc_ast::AstKind;
use oxc_ast::ast::TSType;
use oxc_semantic::AstNode;
use oxc_syntax::operator::UnaryOperator;
use serde_json::Value;

use super::super::{Context, Rule};
use super::shared::enclosing_function;

pub const NAME: &str = "anti-slop/no-runtime-typeof";

/// `[{ allowInTypeGuards?: boolean }]`, validated like v1's schema.
pub fn build(options: &[Value]) -> Result<Box<dyn Rule>, String> {
    let mut rule = NoRuntimeTypeof { allow_in_type_guards: false };

    match options {
        [] => {}
        [Value::Object(object)] => {
            for (key, value) in object {
                match key.as_str() {
                    "allowInTypeGuards" => rule.allow_in_type_guards = value.as_bool().ok_or("\"allowInTypeGuards\" must be a boolean")?,
                    other => return Err(format!("unknown option \"{other}\"")),
                }
            }
        }
        [_] => return Err("expected an options object".into()),
        _ => return Err("expected at most one options object".into()),
    }

    Ok(Box::new(rule))
}

struct NoRuntimeTypeof {
    allow_in_type_guards: bool,
}

impl Rule for NoRuntimeTypeof {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let AstKind::UnaryExpression(unary) = node.kind() else {
            return;
        };

        if unary.operator != UnaryOperator::Typeof || (self.allow_in_type_guards && is_inside_type_guard(ctx, node)) {
            return;
        }

        ctx.report(
            unary.span,
            "A `typeof` check narrows a representation without establishing its contract. Parse input at its I/O boundary, then branch on the domain value.",
        );
    }
}

/// Whether the nearest enclosing function returns a type predicate.
fn is_inside_type_guard(ctx: &Context<'_, '_>, node: &AstNode<'_>) -> bool {
    let return_type = enclosing_function(ctx.semantic.nodes(), node.id(), false).and_then(|function| match function.kind() {
        AstKind::Function(function) => function.return_type.as_deref(),
        AstKind::ArrowFunctionExpression(arrow) => arrow.return_type.as_deref(),
        _ => None,
    });

    return_type.is_some_and(|annotation| matches!(annotation.type_annotation, TSType::TSTypePredicate(_)))
}
