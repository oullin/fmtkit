//! `anti-slop-martinfowler/long-parameter-list`: functions that take more
//! positional parameters than `max` (default 4).
//!
//! Every parameter but a rest parameter and TypeScript's `this` counts:
//! defaults, destructured patterns and constructor parameter properties too.
//! Bodyless signatures (overloads, `declare function`, interface methods) are
//! skipped: the implementation carries the list. The report underlines the
//! function's name, or the binding or key holding it, else the function.
//!
//! Options: `{ ignoreTests?: boolean = true, max?: integer >= 0 = 4 }`.
//! Test files (upstream's `isTestFile`) are skipped unless `ignoreTests` is
//! `false`.

use oxc_ast::AstKind;
use oxc_semantic::AstNode;
use serde_json::Value;

use super::super::{Context, Rule};
use super::shared::{Options, describe_function, function_name, function_report_span, is_function_node, is_test_file};

pub const NAME: &str = "anti-slop-martinfowler/long-parameter-list";

pub fn build(options: &[Value]) -> Result<Box<dyn Rule>, String> {
    let options = Options::parse(options, &["ignoreTests", "max"])?;

    Ok(Box::new(LongParameterList { ignore_tests: options.boolean("ignoreTests", true)?, max: options.integer("max", 0, 4)? }))
}

struct LongParameterList {
    ignore_tests: bool,
    max: i64,
}

impl Rule for LongParameterList {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let params = match node.kind() {
            AstKind::Function(function) if is_function_node(node.kind()) => &function.params,
            AstKind::ArrowFunctionExpression(arrow) => &arrow.params,
            _ => return,
        };

        let count = params.items.len();

        if i64::try_from(count).is_ok_and(|count| count <= self.max) || (self.ignore_tests && is_test_file(ctx.rel)) {
            return;
        }

        let semantic = ctx.semantic;
        let subject = describe_function(function_name(semantic, node).as_deref());
        let max = self.max;

        ctx.report(
            function_report_span(semantic.nodes(), node),
            format!(
                "Possibly Long Parameter List: {subject} takes {count} positional parameters (limit {max}), so every call site depends on argument order and adding one more breaks them all. Consider Introduce Parameter Object: group related parameters into one named input (in Effect, a `Schema.Struct` input type), or Preserve Whole Object when several come from the same value."
            ),
        );
    }
}
