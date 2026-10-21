//! What the test rules share: which files are tests, which names register a
//! test, and which objects are the Vitest or Jest test utilities.

use std::ffi::OsStr;
use std::path::Path;

use oxc_ast::AstKind;
use oxc_ast::ast::Expression;

use super::super::super::Context;
use super::strip_parens;

/// The functions that register a test or a suite.
pub const TEST_FUNCTIONS: [&str; 4] = ["describe", "it", "suite", "test"];

/// The script extensions a test file can have, as `[cm]?[jt]sx?`.
const SCRIPT_EXTENSIONS: [&str; 12] = ["js", "jsx", "ts", "tsx", "cjs", "cjsx", "cts", "ctsx", "mjs", "mjsx", "mts", "mtsx"];

/// Whether `rel` names a test file: `*.test.<ext>` or `*.spec.<ext>`.
pub fn is_test_file(rel: &str) -> bool {
    let path = Path::new(rel);
    let extension = path.extension().and_then(OsStr::to_str);
    let kind = path.file_stem().map(Path::new).and_then(Path::extension).and_then(OsStr::to_str);

    extension.is_some_and(|extension| SCRIPT_EXTENSIONS.contains(&extension)) && matches!(kind, Some("test" | "spec"))
}

/// The identifier a callee chain starts from: `it` in `it.skip.each(cases)`.
pub fn chain_root<'a>(mut expression: &Expression<'a>) -> Option<&'a str> {
    loop {
        match strip_parens(expression) {
            Expression::Identifier(identifier) => return Some(identifier.name.as_str()),
            Expression::StaticMemberExpression(member) => expression = &member.object,
            Expression::ComputedMemberExpression(member) => expression = &member.object,
            Expression::CallExpression(call) => expression = &call.callee,
            _ => return None,
        }
    }
}

/// Whether `expression` is a chain that registers a test or a suite.
pub fn is_test_chain(expression: &Expression<'_>) -> bool {
    chain_root(expression).is_some_and(|root| TEST_FUNCTIONS.contains(&root))
}

/// Whether `expression` names the global `vi` or `jest`, or `vi` imported from
/// `vitest` or `jest` from `@jest/globals` under any local name.
pub fn is_test_framework_object(ctx: &Context<'_, '_>, expression: &Expression<'_>) -> bool {
    let Expression::Identifier(identifier) = strip_parens(expression) else {
        return false;
    };

    let nodes = ctx.semantic.nodes();
    let scoping = ctx.semantic.scoping();
    let Some(symbol) = scoping.find_binding(nodes.get_node(identifier.node_id.get()).scope_id(), identifier.name) else {
        return identifier.name == "vi" || identifier.name == "jest";
    };

    let declaration = scoping.symbol_declaration(symbol);
    let AstKind::ImportSpecifier(specifier) = nodes.kind(declaration) else {
        return false;
    };

    let imported = specifier.imported.name();

    nodes
        .ancestor_kinds(declaration)
        .find_map(|kind| if let AstKind::ImportDeclaration(import) = kind { Some(import.source.value.as_str()) } else { None })
        .is_some_and(|source| (source == "vitest" && imported == "vi") || (source == "@jest/globals" && imported == "jest"))
}
