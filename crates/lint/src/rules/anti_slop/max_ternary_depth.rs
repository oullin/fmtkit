//! `anti-slop/max-ternary-depth`: no ternary chains deeper than `maxDepth`.
//!
//! Reports the outermost conditional of a chain whose depth, counted through
//! the test, consequent and alternate and through parentheses, `as`, `<T>`,
//! `!` and `satisfies`, exceeds the limit. A conditional nested in another
//! (through those wrappers) is judged with its outermost one only.
//!
//! Options: `[{ maxDepth?: integer >= 1 }]`, default `2`.

use oxc_ast::AstKind;
use oxc_ast::ast::Expression;
use oxc_semantic::AstNode;
use serde_json::Value;

use super::super::{Context, Rule};
use super::shared::ancestors;

pub const NAME: &str = "anti-slop/max-ternary-depth";

/// `[{ maxDepth?: integer >= 1 }]`, validated like upstream's schema.
pub fn build(options: &[Value]) -> Result<Box<dyn Rule>, String> {
    let mut rule = MaxTernaryDepth { max_depth: 2 };

    match options {
        [] => {}
        [Value::Object(object)] => {
            for (key, value) in object {
                match key.as_str() {
                    "maxDepth" => rule.max_depth = positive_integer(value).ok_or("\"maxDepth\" must be an integer of at least 1")?,
                    other => return Err(format!("unknown option \"{other}\"")),
                }
            }
        }
        [_] => return Err("expected an options object".into()),
        _ => return Err("expected at most one options object".into()),
    }

    Ok(Box::new(rule))
}

/// A JSON integer of at least 1; `3.0` counts, as JSON Schema's `integer`.
fn positive_integer(value: &Value) -> Option<u64> {
    let number = value.as_number()?;
    let integer = number.as_u64().or_else(|| {
        let float = number.as_f64()?;

        // Saturates above u64::MAX, where no ternary chain reaches anyway.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "the float is checked to be a whole number of at least 1")]
        (float.fract() == 0.0 && float >= 1.0).then_some(float as u64)
    })?;

    (integer >= 1).then_some(integer)
}

struct MaxTernaryDepth {
    max_depth: u64,
}

impl Rule for MaxTernaryDepth {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let AstKind::ConditionalExpression(conditional) = node.kind() else {
            return;
        };

        let parent = ancestors(ctx.semantic.nodes(), node.id()).find(|ancestor| {
            !matches!(
                ancestor.kind(),
                AstKind::TSAsExpression(_) | AstKind::TSSatisfiesExpression(_) | AstKind::TSTypeAssertion(_) | AstKind::TSNonNullExpression(_)
            )
        });

        if parent.is_some_and(|parent| matches!(parent.kind(), AstKind::ConditionalExpression(_))) {
            return;
        }

        let depth = 1 + depth(&conditional.test).max(depth(&conditional.consequent)).max(depth(&conditional.alternate));

        if depth > self.max_depth {
            ctx.report(
                conditional.span,
                format!(
                    "This ternary has a depth of {depth}; the configured maximum is {}. Use a lookup table, switch, or named decision function.",
                    self.max_depth
                ),
            );
        }
    }
}

/// The ternary depth of `expression` through its wrappers: 0 for anything
/// but a conditional.
fn depth(mut expression: &Expression<'_>) -> u64 {
    loop {
        expression = match expression {
            Expression::ParenthesizedExpression(it) => &it.expression,
            Expression::TSAsExpression(it) => &it.expression,
            Expression::TSSatisfiesExpression(it) => &it.expression,
            Expression::TSTypeAssertion(it) => &it.expression,
            Expression::TSNonNullExpression(it) => &it.expression,
            Expression::ConditionalExpression(it) => return 1 + depth(&it.test).max(depth(&it.consequent)).max(depth(&it.alternate)),
            _ => return 0,
        };
    }
}
