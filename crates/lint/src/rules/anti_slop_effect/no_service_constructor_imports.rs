//! `anti-slop-effect/no-service-constructor-imports`: outside test and spec
//! files, no named import of a `make<Capability>` constructor from a
//! project-local module (`./` or `../`); import the owning Layer and yield the
//! service instead.
//!
//! The imported name counts, not the local alias: `makeX as createX` is
//! reported. Package imports, default and namespace imports, and names such as
//! `makeissue` are left alone. `*.test.*` and `*.spec.*` files are skipped.
//! Takes no options.

use oxc_ast::AstKind;
use oxc_ast::ast::ImportDeclarationSpecifier;
use oxc_semantic::AstNode;

use super::super::anti_slop::shared::testing::is_test_file;
use super::super::{Context, Rule};

pub const NAME: &str = "anti-slop-effect/no-service-constructor-imports";

pub struct NoServiceConstructorImports;

impl Rule for NoServiceConstructorImports {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let AstKind::ImportDeclaration(import) = node.kind() else {
            return;
        };
        let Some(source) = import.source.value.as_str() else {
            return;
        };

        if !(source.starts_with("./") || source.starts_with("../")) || is_test_file(ctx.rel) {
            return;
        }

        for specifier in import.specifiers.iter().flatten() {
            let ImportDeclarationSpecifier::ImportSpecifier(specifier) = specifier else {
                continue;
            };
            let name = specifier.imported.name();

            if name.strip_prefix("make").is_some_and(|rest| rest.starts_with(|c: char| c.is_ascii_uppercase())) {
                ctx.report(
                    specifier.span,
                    format!(
                        "Do not import Effect service constructor \"{name}\" into runtime code. Import the owning Layer, yield the contextual service, and allow its requirements to propagate to the composition root."
                    ),
                );
            }
        }
    }
}
