//! `anti-slop/no-unknown-type-aliases`: no module-level alias that resolves
//! to `unknown`, which must stay visible where it is allowed.

use oxc_ast::ast::{Declaration, Statement, TSType};
use serde_json::Value;

use super::super::{Context, Rule};
use super::shared::strip_type_parens;
use super::shared::types::{TypeEnvironment, Types, reference_name, type_arguments};

pub const NAME: &str = "anti-slop/no-unknown-type-aliases";

pub fn build(_options: &[Value]) -> Result<Box<dyn Rule>, String> {
    Ok(Box::new(NoUnknownTypeAliases))
}

struct NoUnknownTypeAliases;

impl Rule for NoUnknownTypeAliases {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run_once(&self, ctx: &mut Context<'_, '_>) {
        let semantic = ctx.semantic;
        let env = TypeEnvironment::of(ctx);
        let types = env.with(semantic.nodes());

        for statement in &semantic.nodes().program().body {
            let declaration = match statement {
                Statement::ExportNamedDeclaration(export) => export.declaration.as_ref(),
                _ => statement.as_declaration(),
            };

            let Some(Declaration::TSTypeAliasDeclaration(alias)) = declaration else {
                continue;
            };

            // A redeclared alias counts once, by its last declaration.
            if env.last_alias_id(&alias.id.name) != Some(alias.node_id.get()) {
                continue;
            }

            if resolves_to_unknown(types, &alias.type_annotation, &mut vec![alias.id.name.as_str()]) {
                let name = alias.id.name;

                ctx.report(
                    alias.id.span,
                    format!(
                        "Type alias `{name}` hides `unknown`. Keep `unknown` explicit at the parsing boundary or on an allowed `cause` field; otherwise use the parsed owner type."
                    ),
                );
            }
        }
    }
}

fn resolves_to_unknown<'a>(types: Types<'_, 'a>, ty: &'a TSType<'a>, visited: &mut Vec<&'a str>) -> bool {
    let reference = match strip_type_parens(ty) {
        TSType::TSUnknownKeyword(_) => return true,
        TSType::TSTypeReference(reference) => reference,
        _ => return false,
    };

    let Some(name) = reference_name(reference).filter(|name| type_arguments(reference).is_empty() && !visited.contains(name)) else {
        return false;
    };

    let Some(alias) = types.last_alias(name).filter(|alias| alias.type_parameters.is_none()) else {
        return false;
    };

    visited.push(name);

    let resolved = resolves_to_unknown(types, &alias.type_annotation, visited);

    visited.pop();
    resolved
}
