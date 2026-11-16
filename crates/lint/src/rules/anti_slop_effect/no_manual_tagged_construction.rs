//! `anti-slop-effect/no-manual-tagged-construction`: no object literal with a
//! `_tag` property and no class property named `_tag`; construct tagged
//! values with their Schema or Data constructor.
//!
//! An object literal is reported once, at its first `_tag` property: an
//! identifier key, or a string key, computed or not. A class property counts
//! with an identifier key, computed or not, or a string key; accessors,
//! abstract properties and `#_tag` do not. Takes no options.

use oxc_ast::AstKind;
use oxc_ast::ast::{ObjectPropertyKind, PropertyDefinitionType};
use oxc_semantic::AstNode;

use super::super::{Context, Rule};
use super::shared::tagged_values::{is_tag_key, property_name};

pub const NAME: &str = "anti-slop-effect/no-manual-tagged-construction";

const CONSTRUCTION: &str = "Use the existing Schema tagged `.make` or tagged class/error constructor instead of writing a literal `_tag` object.";
const DECLARATION: &str =
    "Declare tagged classes with Schema.TaggedClass, Schema.TaggedError, Data.TaggedClass, or Data.TaggedError instead of writing a literal `_tag` property.";

pub struct NoManualTaggedConstruction;

impl Rule for NoManualTaggedConstruction {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        match node.kind() {
            AstKind::ObjectExpression(object) => {
                let tag = object.properties.iter().find_map(|property| match property {
                    ObjectPropertyKind::ObjectProperty(property) if property_name(property) == Some("_tag") => Some(property.span),
                    _ => None,
                });

                if let Some(span) = tag {
                    ctx.report(span, CONSTRUCTION);
                }
            }
            AstKind::PropertyDefinition(property) if property.r#type == PropertyDefinitionType::PropertyDefinition && is_tag_key(&property.key) => {
                ctx.report(property.span, DECLARATION);
            }
            _ => {}
        }
    }
}
