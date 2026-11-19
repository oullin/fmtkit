//! `quality/no-array-filter-map`: no eager `.filter(...).map(...)` or
//! `.map(...).filter(...)` pass pairs over a known array.
//!
//! Reports the outer call when its receiver, through parentheses, optional
//! chains and TS wrappers, is the adjacent opposite method called on local
//! array evidence (see [`array_method::is_known_array`]). Skips unknown
//! receivers, so iterator helpers (`.values().filter().map()`), custom
//! `filter`/`map` objects and unannotated parameters pass. No options.

use oxc_ast::AstKind;
use oxc_semantic::AstNode;

use super::super::anti_slop::shared::array_method::{self, Bare};
use super::super::{Context, Rule};

pub const NAME: &str = "quality/no-array-filter-map";

pub struct NoArrayFilterMap;

impl Rule for NoArrayFilterMap {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let AstKind::CallExpression(call) = node.kind() else {
            return;
        };

        let Some((second, object)) = array_method::method_target(&call.callee) else {
            return;
        };

        let first = match second {
            "map" => "filter",
            "filter" => "map",
            _ => return,
        };

        let Bare::Call(inner) = array_method::unwrap(object) else {
            return;
        };

        let Some((name, receiver)) = array_method::method_target(&inner.callee) else {
            return;
        };

        if name != first || !array_method::is_known_array(ctx.semantic, receiver, &mut Vec::new()) {
            return;
        }

        ctx.report(
            call.span,
            format!(
                "Avoid consecutive array `{first}` and `{second}` passes. Prefer `.values().{first}(...).{second}(...).toArray()` where iterator helpers are supported, or a single `flatMap`/mutating reducer. Preserve callback ordering, indexes, and filtering semantics."
            ),
        );
    }
}
