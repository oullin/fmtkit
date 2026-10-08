//! `anti-slop/no-object-parameters`: no broad `object` parameter types,
//! directly or through a module-level alias.

use oxc_ast::ast::TSType;
use oxc_semantic::AstNode;
use oxc_span::GetSpan;
use rustc_hash::FxHashSet;
use serde_json::Value;

use super::super::{Context, Rule};
use super::shared::lexical::type_parameter_names;
use super::shared::params::Signature;
use super::shared::strip_type_parens;
use super::shared::types::{TypeEnvironment, Types, reference_name};

pub const NAME: &str = "anti-slop/no-object-parameters";

pub fn build(_options: &[Value]) -> Result<Box<dyn Rule>, String> {
    Ok(Box::new(NoObjectParameters))
}

struct NoObjectParameters;

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
        let mut shadowed = None;
        let mut env = None;

        for param in signature.params() {
            let Some(annotation) = param.annotation() else {
                continue;
            };

            let shadowed = shadowed.get_or_insert_with(|| type_parameter_names(semantic.nodes(), node.id()));
            let types = env.get_or_insert_with(|| TypeEnvironment::of(ctx)).with(semantic.nodes());
            let ty = strip_type_parens(&annotation.type_annotation);

            if resolves_to_object(types, ty, shadowed, &mut Vec::new()) {
                let parameter = param.display_name(source, "object");

                ctx.report(
                    ty.span(),
                    format!("Parameter `{parameter}` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function."),
                );
            }
        }
    }
}

fn resolves_to_object<'a>(types: Types<'_, 'a>, ty: &'a TSType<'a>, shadowed: &FxHashSet<&str>, visited: &mut Vec<&'a str>) -> bool {
    match strip_type_parens(ty) {
        TSType::TSObjectKeyword(_) => true,
        TSType::TSUnionType(union) => union.types.iter().any(|member| resolves_to_object(types, member, shadowed, visited)),
        TSType::TSTypeReference(reference) => {
            let Some(name) = reference_name(reference) else {
                return false;
            };

            if reference.type_arguments.as_ref().is_some_and(|arguments| !arguments.params.is_empty()) || visited.contains(&name) || shadowed.contains(name) {
                return false;
            }

            let Some(alias) = types.last_plain_alias(name) else {
                return false;
            };

            visited.push(name);

            let resolved = resolves_to_object(types, &alias.type_annotation, shadowed, visited);

            visited.pop();
            resolved
        }
        _ => false,
    }
}
