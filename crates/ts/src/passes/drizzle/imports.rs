//! The `drizzle-orm` bindings a module imports.

use oxc_ast::ast::{ImportDeclarationSpecifier, ModuleExportName, Program, Statement};
use rustc_hash::{FxHashMap, FxHashSet};

/// Local names bound to Drizzle exports, and namespace imports of Drizzle.
#[derive(Debug, Default)]
pub(crate) struct DrizzleImports<'a> {
    locals: FxHashMap<&'a str, &'a str>,
    namespaces: FxHashSet<&'a str>,
}

impl<'a> DrizzleImports<'a> {
    /// Read the top-level imports whose source starts with `drizzle-orm`.
    pub(crate) fn scan(program: &'a Program<'a>) -> Self {
        let mut imports = Self::default();

        for statement in &program.body {
            let Statement::ImportDeclaration(declaration) = statement else { continue };

            if !declaration.source.value.starts_with("drizzle-orm") {
                continue;
            }

            for specifier in declaration.specifiers.iter().flatten() {
                match specifier {
                    ImportDeclarationSpecifier::ImportSpecifier(specifier) => {
                        let imported = match &specifier.imported {
                            ModuleExportName::IdentifierName(name) => name.name.as_str(),
                            ModuleExportName::IdentifierReference(name) => name.name.as_str(),
                            ModuleExportName::StringLiteral(_) => continue,
                        };

                        imports.locals.insert(specifier.local.name.as_str(), imported);
                    }
                    ImportDeclarationSpecifier::ImportNamespaceSpecifier(specifier) => {
                        imports.namespaces.insert(specifier.local.name.as_str());
                    }
                    ImportDeclarationSpecifier::ImportDefaultSpecifier(_) => {}
                }
            }
        }

        imports
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.locals.is_empty() && self.namespaces.is_empty()
    }

    /// The Drizzle export a local name is bound to.
    pub(crate) fn local_import(&self, local: &str) -> Option<&'a str> {
        self.locals.get(local).copied()
    }

    pub(crate) fn has_namespace(&self, name: &str) -> bool {
        self.namespaces.contains(name)
    }
}

#[cfg(test)]
mod tests {
    use oxc_allocator::Allocator;
    use oxc_span::SourceType;

    use super::DrizzleImports;
    use crate::syntax::parse;

    fn with_imports(source: &str, check: impl FnOnce(&DrizzleImports<'_>)) {
        let allocator = Allocator::default();
        let program = parse(&allocator, source, SourceType::ts()).expect("fixture parses");

        check(&DrizzleImports::scan(program));
    }

    #[test]
    fn resolves_named_and_aliased_drizzle_imports() {
        with_imports("import { and as all, eq } from 'drizzle-orm';\n", |imports| {
            assert!(!imports.is_empty());
            assert_eq!(imports.local_import("all"), Some("and"));
            assert_eq!(imports.local_import("eq"), Some("eq"));
            assert_eq!(imports.local_import("missing"), None);
            assert!(!imports.has_namespace("all"));
        });
    }

    #[test]
    fn records_namespace_imports() {
        with_imports("import * as drizzle from 'drizzle-orm';\n", |imports| {
            assert!(!imports.is_empty());
            assert!(imports.has_namespace("drizzle"));
            assert!(!imports.has_namespace("other"));
            assert_eq!(imports.local_import("drizzle"), None);
        });
    }

    #[test]
    fn matches_submodule_sources() {
        with_imports("import { sql } from 'drizzle-orm/pg-core';\n", |imports| assert_eq!(imports.local_import("sql"), Some("sql")));
    }

    #[test]
    fn ignores_non_drizzle_imports() {
        with_imports("import { eq } from 'other-orm';\n", |imports| {
            assert!(imports.is_empty());
            assert_eq!(imports.local_import("eq"), None);
        });
    }

    #[test]
    fn empty_imports_carry_no_bindings() {
        let imports = DrizzleImports::default();

        assert!(imports.is_empty());
        assert_eq!(imports.local_import("eq"), None);
        assert!(!imports.has_namespace("drizzle"));
    }
}
