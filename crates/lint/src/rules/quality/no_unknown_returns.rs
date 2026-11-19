//! `quality/no-unknown-returns`: no explicit `unknown` or `Promise<unknown>`
//! return contracts, directly or through an alias visible where the
//! signature is.
//!
//! Unlike the k-mistele fork's `no-unknown-returns`, aliases resolve lexically and
//! through their type arguments (`Gen<number>` where `type Gen<T> = unknown`).

use oxc_semantic::AstNode;
use oxc_span::GetSpan;

use super::super::anti_slop::shared::params::Signature;
use super::super::anti_slop::shared::strip_type_parens;
use super::super::{Context, Rule};
use super::bindings::{Top, TypeBindings};

pub const NAME: &str = "quality/no-unknown-returns";

pub struct NoUnknownReturns;

impl Rule for NoUnknownReturns {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let Some(annotation) = Signature::of(node.kind()).and_then(|signature| signature.return_type) else {
            return;
        };

        let semantic = ctx.semantic;
        let bindings = TypeBindings::of(ctx);

        if bindings.with(semantic.nodes()).resolves_to(&annotation.type_annotation, Top::UnknownOrPromise) {
            ctx.report(
                strip_type_parens(&annotation.type_annotation).span(),
                "This function exposes `unknown` to its caller. Parse the value at its boundary and return a named domain type.",
            );
        }
    }
}
