//! `anti-slop/no-unknown-parameters`: no explicitly `unknown` parameters
//! except `cause`; unknown input is decoded at its I/O boundary.

use oxc_ast::ast::TSType;
use oxc_semantic::AstNode;
use oxc_span::GetSpan;

use super::super::{Context, Rule};
use super::shared::params::Signature;
use super::shared::strip_type_parens;

pub const NAME: &str = "anti-slop/no-unknown-parameters";

pub struct NoUnknownParameters;

impl Rule for NoUnknownParameters {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let Some(signature) = Signature::of(node.kind()) else {
            return;
        };

        let source = ctx.source();

        for param in signature.params() {
            let Some(ty) = param.annotation().map(|annotation| strip_type_parens(&annotation.type_annotation)) else {
                continue;
            };

            if !matches!(ty, TSType::TSUnknownKeyword(_)) {
                continue;
            }

            let parameter = param.inner_display_name(source, "unknown");

            if parameter != "cause" {
                ctx.report(
                    ty.span(),
                    format!(
                        "Parameter `{parameter}` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function."
                    ),
                );
            }
        }
    }
}
