//! `quality/no-unknown-type-aliases`: no alias that resolves to `unknown`,
//! which must stay visible where it is allowed.
//!
//! Unlike the k-mistele fork's `no-unknown-type-aliases`, every alias counts, nested
//! ones included; a union holding `unknown` resolves to it; and aliases
//! resolve lexically and through their type arguments (`G<unknown>`).

use oxc_ast::AstKind;
use oxc_semantic::AstNode;

use super::super::{Context, Rule};
use super::bindings::{Top, TypeBindings};

pub const NAME: &str = "quality/no-unknown-type-aliases";

pub struct NoUnknownTypeAliases;

impl Rule for NoUnknownTypeAliases {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let AstKind::TSTypeAliasDeclaration(alias) = node.kind() else {
            return;
        };

        let semantic = ctx.semantic;
        let bindings = TypeBindings::of(ctx);

        if bindings.with(semantic.nodes()).resolves_to(&alias.type_annotation, Top::Unknown) {
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
