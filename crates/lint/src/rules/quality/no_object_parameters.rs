//! `quality/no-object-parameters`: no broad `object` parameter types,
//! directly or through an alias visible where the parameter is.
//!
//! Unlike the k-mistele fork's `no-object-parameters`, aliases resolve lexically and
//! through their type arguments (`G<string>` where `type G<T> = object`),
//! and the message names only the parameter's binding (`d`, not
//! `d: object = {}`; `x`, not `private readonly x`).

use oxc_semantic::AstNode;
use oxc_span::GetSpan;

use super::super::anti_slop::shared::params::Signature;
use super::super::anti_slop::shared::strip_type_parens;
use super::super::{Context, Rule};
use super::bindings::{Top, TypeBindings};

pub const NAME: &str = "quality/no-object-parameters";

pub struct NoObjectParameters;

impl Rule for NoObjectParameters {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let Some(signature) = Signature::of(node.kind()) else {
            return;
        };

        let semantic = ctx.semantic;
        let source = semantic.source_text();
        let mut bindings = None;

        for param in signature.params() {
            let Some(annotation) = param.annotation() else {
                continue;
            };

            let scoped = bindings.get_or_insert_with(|| TypeBindings::of(ctx)).with(semantic.nodes());

            if scoped.resolves_to(&annotation.type_annotation, Top::Object) {
                let parameter = param.binding_display_name(source);

                ctx.report(
                    strip_type_parens(&annotation.type_annotation).span(),
                    format!("Parameter `{parameter}` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function."),
                );
            }
        }
    }
}
