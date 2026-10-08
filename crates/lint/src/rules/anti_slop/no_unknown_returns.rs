//! `anti-slop/no-unknown-returns`: no explicit `unknown` or `Promise<unknown>`
//! return contracts, directly or through a module-level alias.

use oxc_ast::ast::TSType;
use oxc_semantic::AstNode;
use oxc_span::GetSpan;
use rustc_hash::FxHashSet;

use super::super::{Context, Rule};
use super::shared::lexical::type_parameter_names;
use super::shared::params::Signature;
use super::shared::strip_type_parens;
use super::shared::types::{TypeEnvironment, Types, reference_name, type_arguments};

pub const NAME: &str = "anti-slop/no-unknown-returns";

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
        let env = TypeEnvironment::of(ctx);
        let types = env.with(semantic.nodes());
        let shadowed = type_parameter_names(semantic.nodes(), node.id());
        let ty = strip_type_parens(&annotation.type_annotation);

        if resolves_to_unknown(types, ty, &shadowed, &mut Vec::new()) {
            ctx.report(ty.span(), "This function exposes `unknown` to its caller. Parse the value at its boundary and return a named domain type.");
        }
    }
}

fn resolves_to_unknown<'a>(types: Types<'_, 'a>, ty: &'a TSType<'a>, shadowed: &FxHashSet<&str>, visited: &mut Vec<&'a str>) -> bool {
    let reference = match strip_type_parens(ty) {
        TSType::TSUnknownKeyword(_) => return true,
        TSType::TSUnionType(union) => return union.types.iter().any(|member| resolves_to_unknown(types, member, shadowed, visited)),
        TSType::TSTypeReference(reference) => reference,
        _ => return false,
    };

    let Some(name) = reference_name(reference) else {
        return false;
    };

    let arguments = type_arguments(reference);

    if name == "Promise" || name == "PromiseLike" {
        return arguments.first().is_some_and(|value| resolves_to_unknown(types, value, shadowed, visited));
    }

    if !arguments.is_empty() || visited.contains(&name) || shadowed.contains(name) {
        return false;
    }

    let Some(alias) = types.last_alias(name).filter(|alias| alias.type_parameters.is_none()) else {
        return false;
    };

    visited.push(name);

    let resolved = resolves_to_unknown(types, &alias.type_annotation, shadowed, visited);

    visited.pop();

    resolved
}
