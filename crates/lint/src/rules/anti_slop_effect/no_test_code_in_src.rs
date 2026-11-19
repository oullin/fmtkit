//! `anti-slop-effect/no-test-code-in-src`: inside a production source
//! directory, no test files, no relative imports or re-exports from a test
//! directory, and no exports or static class members with test names such as
//! `layerTest`, `FakeClock` or `makeMockStore`.
//!
//! A file is in production source when a segment of its repository-relative
//! path is a source directory; files elsewhere are not checked at all. Such a
//! file is itself reported (at the program) when it is a `*.test.*` or
//! `*.spec.*` file or sits in a test directory below the source directory.
//! Imports count only with a `./` or `../` specifier that has a test-directory
//! segment, through `import`, `import()`, `export ... from` and
//! `export * from`. Names count for local export declarations (not
//! `declare function` signatures), export specifiers, and non-computed static
//! properties and methods of any class.
//!
//! Options: `[{ sourceDirectories?, testDirectories?, testNamePatterns? }]`,
//! each a list of unique non-empty strings that replaces its default:
//! `["src"]`, `["test", "tests", "__tests__"]`, and five ECMAScript patterns
//! (Unicode mode) for test layers, fakes, mocks, stubs and their factories.

use oxc_ast::AstKind;
use oxc_ast::ast::{
    BindingPattern, Declaration, ExportDefaultDeclarationKind, Expression, FunctionType, MethodDefinitionType, Program, PropertyDefinitionType, PropertyKey,
    Statement,
};
use oxc_semantic::AstNode;
use oxc_span::{GetSpan, Span};
use regress::Regex;
use serde_json::Value;

use super::super::anti_slop::shared::strip_parens;
use super::super::anti_slop::shared::testing::is_test_file;
use super::super::{Context, Rule};

pub const NAME: &str = "anti-slop-effect/no-test-code-in-src";

const SOURCE_DIRECTORIES: [&str; 1] = ["src"];
const TEST_DIRECTORIES: [&str; 3] = ["test", "tests", "__tests__"];
const TEST_NAME_PATTERNS: [&str; 5] = [
    "^layer(?:Test|Fake|Mock|Stub)",
    "(?:Test|Fake|Mock|Stub)Layer$",
    "^(?:Fake|Mock|Stub)[A-Z]",
    "[a-z0-9](?:Fake|Mock|Stub)$",
    "^(?:make|create)(?:Test|Fake|Mock|Stub)[A-Z]",
];

/// `[{ sourceDirectories?, testDirectories?, testNamePatterns? }]`, validated
/// like upstream's schema: unique non-empty strings, no other keys.
pub fn build(options: &[Value]) -> Result<Box<dyn Rule>, String> {
    let mut source_directories = owned(&SOURCE_DIRECTORIES);
    let mut test_directories = owned(&TEST_DIRECTORIES);
    let mut test_name_patterns = owned(&TEST_NAME_PATTERNS);

    match options {
        [] => {}
        [Value::Object(object)] => {
            for (key, value) in object {
                match key.as_str() {
                    "sourceDirectories" => source_directories = strings(key, value)?,
                    "testDirectories" => test_directories = strings(key, value)?,
                    "testNamePatterns" => test_name_patterns = strings(key, value)?,
                    other => return Err(format!("unknown option \"{other}\"")),
                }
            }
        }
        [_] => return Err("expected an options object".into()),
        _ => return Err("expected at most one options object".into()),
    }

    let test_name_patterns = test_name_patterns
        .iter()
        .map(|pattern| Regex::with_flags(pattern, "u").map_err(|error| format!("invalid regular expression /{pattern}/u in \"testNamePatterns\": {error}")))
        .collect::<Result<_, _>>()?;

    Ok(Box::new(NoTestCodeInSrc { source_directories, test_directories, test_name_patterns }))
}

fn owned(items: &[&str]) -> Vec<String> {
    items.iter().map(|&item| item.to_owned()).collect()
}

/// A list of unique non-empty strings.
fn strings(key: &str, value: &Value) -> Result<Vec<String>, String> {
    let error = || format!("\"{key}\" must be a list of unique non-empty strings");
    let items = value.as_array().ok_or_else(error)?;
    let mut strings: Vec<String> = Vec::with_capacity(items.len());

    for item in items {
        let item = item.as_str().filter(|item| !item.is_empty()).ok_or_else(error)?;

        if strings.iter().any(|seen| seen == item) {
            return Err(error());
        }

        strings.push(item.to_owned());
    }

    Ok(strings)
}

struct NoTestCodeInSrc {
    source_directories: Vec<String>,
    test_directories: Vec<String>,
    test_name_patterns: Vec<Regex>,
}

const TEST_IMPORT: &str = "from a test directory. Production modules never depend on test code: move what it needs into source, or move this module to test/.";

impl Rule for NoTestCodeInSrc {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        match node.kind() {
            AstKind::Program(program) => {
                let segments: Vec<&str> = ctx.rel.split('/').collect();
                let Some(index) = self.source_index(&segments) else {
                    return;
                };
                let between = segments.get(index + 1..segments.len().saturating_sub(1)).unwrap_or_default();

                if is_test_file(ctx.rel) || between.iter().any(|segment| self.is_test_directory(segment)) {
                    let directory = segments.get(index).copied().unwrap_or("src");

                    ctx.report(
                        estree_program_span(program),
                        format!(
                            "This test file lives in production source (`{directory}/`). Move it to the package's test/ directory, with shared helpers in test/support/."
                        ),
                    );
                }
            }
            AstKind::ImportDeclaration(import) => self.check_source(ctx, &crate::rules::lossy(import.source.value), import.source.span),
            AstKind::ImportExpression(import) => {
                if let Expression::StringLiteral(source) = strip_parens(&import.source) {
                    self.check_source(ctx, &crate::rules::lossy(source.value), source.span);
                }
            }
            AstKind::ExportAllDeclaration(export) => self.check_source(ctx, &crate::rules::lossy(export.source.value), export.source.span),
            AstKind::ExportFromDeclaration(export) => {
                self.check_source(ctx, &crate::rules::lossy(export.source.value), export.source.span);

                for specifier in &export.specifiers {
                    self.report_name(ctx, specifier.span, &specifier.exported.name());
                }
            }
            AstKind::ExportNamedDeclaration(export) => {
                for specifier in &export.specifiers {
                    self.report_name(ctx, specifier.span, &specifier.exported.name());
                }
            }
            AstKind::ExportDeclaration(export) => {
                for (name, span) in declared_names(&export.declaration) {
                    self.report_name(ctx, span, name);
                }
            }
            AstKind::PropertyDefinition(property)
                if property.r#type == PropertyDefinitionType::PropertyDefinition && property.r#static && !property.computed =>
            {
                if let Some((name, span)) = static_key_name(&property.key) {
                    self.report_name(ctx, span, name);
                }
            }
            AstKind::MethodDefinition(method) if method.r#type == MethodDefinitionType::MethodDefinition && method.r#static && !method.computed => {
                if let Some((name, span)) = static_key_name(&method.key) {
                    self.report_name(ctx, span, name);
                }
            }
            _ => {}
        }
    }
}

impl NoTestCodeInSrc {
    /// The first path segment that is a source directory.
    fn source_index(&self, segments: &[&str]) -> Option<usize> {
        segments.iter().position(|segment| self.source_directories.iter().any(|directory| directory == segment))
    }

    fn is_in_source(&self, rel: &str) -> bool {
        rel.split('/').any(|segment| self.source_directories.iter().any(|directory| directory == segment))
    }

    fn is_test_directory(&self, segment: &str) -> bool {
        self.test_directories.iter().any(|directory| directory == segment)
    }

    fn check_source(&self, ctx: &mut Context<'_, '_>, path: &str, span: Span) {
        if !(path.starts_with("./") || path.starts_with("../"))
            || !path.split('/').any(|segment| self.is_test_directory(segment))
            || !self.is_in_source(ctx.rel)
        {
            return;
        }

        ctx.report(span, format!("Production source imports `{path}` {TEST_IMPORT}"));
    }

    fn report_name(&self, ctx: &mut Context<'_, '_>, span: Span, name: &str) {
        if self.test_name_patterns.iter().any(|pattern| pattern.find(name).is_some()) && self.is_in_source(ctx.rel) {
            ctx.report(
                span,
                format!(
                    "`{name}` is test code in production source. Move it to the package's test/ directory (shared helpers in test/support/) and build test layers there from the production class, e.g. `Service.layer.pipe(Layer.provide(Layer.succeed(Dep, Dep.of({{ ... }}))))`."
                ),
            );
        }
    }
}

/// The `Program` span oxlint gives JS plugins (TS-ESTree's, for JavaScript
/// files too): it starts at the first directive or statement (or a decorator
/// before an exported class), skipping leading whitespace and comments, and
/// is empty at the end when there is none.
fn estree_program_span(program: &Program<'_>) -> Span {
    let first = program.directives.first().map(GetSpan::span).or_else(|| program.body.first().map(statement_span));

    Span::new(first.map_or(program.span.end, |span| span.start), program.span.end)
}

/// A statement's span, starting at a decorator written before `export`.
fn statement_span(statement: &Statement<'_>) -> Span {
    let class = match statement {
        Statement::ExportDeclaration(export) => match &export.declaration {
            Declaration::ClassDeclaration(class) => Some(class.as_ref()),
            _ => None,
        },
        Statement::ExportDefaultDeclaration(export) => match &export.declaration {
            ExportDefaultDeclarationKind::ClassDeclaration(class) => Some(class.as_ref()),
            _ => None,
        },
        _ => None,
    };
    let span = statement.span();

    match class.and_then(|class| class.decorators.first()) {
        Some(decorator) if decorator.span.start < span.start => Span::new(decorator.span.start, span.end),
        _ => span,
    }
}

/// The names a local export declaration binds, at ESTree's identifier spans:
/// a declarator's identifier spans its type annotation too.
fn declared_names<'a>(declaration: &Declaration<'a>) -> Vec<(&'a str, Span)> {
    match declaration {
        Declaration::VariableDeclaration(variables) => variables
            .declarations
            .iter()
            .filter_map(|declarator| match &declarator.id {
                BindingPattern::BindingIdentifier(identifier) => {
                    let end = declarator.type_annotation.as_ref().map_or(identifier.span.end, |annotation| annotation.span.end);

                    Some((identifier.name.as_str(), Span::new(identifier.span.start, end)))
                }
                _ => None,
            })
            .collect(),
        Declaration::FunctionDeclaration(function) if function.r#type == FunctionType::FunctionDeclaration => {
            function.id.iter().map(|id| (id.name.as_str(), id.span)).collect()
        }
        Declaration::ClassDeclaration(class) => class.id.iter().map(|id| (id.name.as_str(), id.span)).collect(),
        Declaration::TSTypeAliasDeclaration(alias) => vec![(alias.id.name.as_str(), alias.id.span)],
        Declaration::TSInterfaceDeclaration(interface) => vec![(interface.id.name.as_str(), interface.id.span)],
        Declaration::TSEnumDeclaration(declaration) => vec![(declaration.id.name.as_str(), declaration.id.span)],
        _ => Vec::new(),
    }
}

/// The name of an identifier or private key.
fn static_key_name<'a>(key: &PropertyKey<'a>) -> Option<(&'a str, Span)> {
    match key {
        PropertyKey::StaticIdentifier(identifier) => Some((identifier.name.as_str(), identifier.span)),
        PropertyKey::PrivateIdentifier(identifier) => Some((identifier.name.as_str(), key.span())),
        _ => None,
    }
}
