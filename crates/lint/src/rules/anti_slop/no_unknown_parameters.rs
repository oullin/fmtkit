//! `anti-slop/no-unknown-parameters`: no explicitly `unknown` parameters;
//! unknown input is decoded at its I/O boundary.
//!
//! Reports a parameter annotated `unknown` or a union containing it (through
//! parentheses), at the annotation. Skips `cause` and the subject of the
//! signature's type predicate or assertion (`value is T`, `asserts value`).
//! No options.

use oxc_ast::ast::{TSType, TSTypePredicateName};
use oxc_semantic::AstNode;
use oxc_span::GetSpan;

use super::super::{Context, Rule};
use super::shared::params::Signature;
use super::shared::strip_type_parens;

pub const NAME: &str = "anti-slop/no-unknown-parameters";

pub struct NoUnknownParameters;

/// Whether `ty` is `unknown`, or a union holding it, as upstream's
/// `containsUnknownType`.
fn contains_unknown(ty: &TSType<'_>) -> bool {
    match strip_type_parens(ty) {
        TSType::TSUnknownKeyword(_) => true,
        TSType::TSUnionType(union) => union.types.iter().any(contains_unknown),
        _ => false,
    }
}

impl Rule for NoUnknownParameters {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let Some(signature) = Signature::of(node.kind()) else {
            return;
        };

        let source = ctx.source();
        let predicate_subject = signature.return_type.and_then(|annotation| match &annotation.type_annotation {
            TSType::TSTypePredicate(predicate) => match &predicate.parameter_name {
                TSTypePredicateName::Identifier(identifier) => Some(identifier.name.as_str()),
                TSTypePredicateName::This(_) => None,
            },
            _ => None,
        });

        for param in signature.params() {
            let Some(ty) = param.annotation().map(|annotation| strip_type_parens(&annotation.type_annotation)) else {
                continue;
            };

            if !contains_unknown(ty) {
                continue;
            }

            let parameter = param.binding_display_name(source);

            if parameter == "cause" || predicate_subject == Some(parameter) {
                continue;
            }

            ctx.report(
                ty.span(),
                format!(
                    "Parameter `{parameter}` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function."
                ),
            );
        }
    }
}
