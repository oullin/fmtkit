//! `anti-slop-effect/no-manual-tagged-type`: no `_tag` property in a type
//! literal or an interface; derive tagged types from a tagged Schema or Data
//! constructor.
//!
//! A property signature counts with an identifier key, computed or not, or a
//! string key. Indexed access such as `Value["_tag"]` and method signatures
//! are not declarations of `_tag`. Takes no options.

use oxc_ast::AstKind;
use oxc_semantic::AstNode;

use super::super::{Context, Rule};
use super::shared::tagged_values::is_tag_key;

pub const NAME: &str = "anti-slop-effect/no-manual-tagged-type";

const MESSAGE: &str = "Do not declare `_tag` by hand. Define the value with Schema.TaggedStruct, Schema.TaggedClass, Schema.TaggedError, Schema.TaggedUnion, Data.TaggedClass, Data.TaggedError, or Data.taggedEnum, then infer the type from it, e.g. `export type X = typeof X.Type`.";

pub struct NoManualTaggedType;

impl Rule for NoManualTaggedType {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        if let AstKind::TSPropertySignature(signature) = node.kind()
            && is_tag_key(&signature.key)
        {
            ctx.report(signature.span, MESSAGE);
        }
    }
}
