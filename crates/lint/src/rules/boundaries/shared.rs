//! Facts shared by enabled boundary rules, without resolving types or imports.

use oxc_ast::AstKind;
use oxc_ast::ast::{
    Declaration, ExportDefaultDeclarationKind, Expression, ImportDeclarationSpecifier, ImportOrExportKind, MethodDefinitionKind, Program, Statement,
    StringLiteral, TSModuleReference, TSType, TSTypeName,
};
use oxc_semantic::Semantic;
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::UnaryOperator;

#[derive(Default)]
pub struct Facts {
    pub specifiers: Vec<(Span, String)>,
    pub identifiers: Vec<(Span, String)>,
    pub exported_classes: Vec<Span>,
    pub non_barrel: Option<Span>,
    pub reflect_calls: Vec<Span>,
    pub anys: Vec<Span>,
    pub typeofs: Vec<Span>,
    pub zod_helpers: Vec<Span>,
    pub infer_imports: Vec<Span>,
    pub nullish_unions: Vec<Span>,
}

impl Facts {
    pub fn new(semantic: &Semantic<'_>) -> Self {
        let mut facts = Self::default();

        for node in semantic.nodes().iter() {
            match node.kind() {
                AstKind::Program(program) => facts.layout(program),
                AstKind::ImportDeclaration(import) => {
                    facts.literal_specifier(&import.source);

                    if import.import_kind == ImportOrExportKind::Type
                        && import.specifiers.as_ref().is_some_and(|specifiers| {
                            specifiers.iter().any(|specifier| {
                                matches!(specifier, ImportDeclarationSpecifier::ImportSpecifier(specifier)
                                    if specifier.imported.name() == "infer" || specifier.local.name == "infer")
                            })
                        })
                    {
                        facts.infer_imports.push(import.span);
                    }
                }
                AstKind::ExportFromDeclaration(export) => facts.literal_specifier(&export.source),
                AstKind::ExportAllDeclaration(export) => facts.literal_specifier(&export.source),
                AstKind::TSImportEqualsDeclaration(import) => {
                    if let TSModuleReference::ExternalModuleReference(reference) = &import.module_reference {
                        facts.literal_specifier(&reference.expression);
                    }
                }
                AstKind::ImportExpression(import) => facts.specifier(&import.source),
                AstKind::CallExpression(call) => {
                    if matches!(&call.callee, Expression::Identifier(id) if id.name == "require")
                        && let Some(argument) = call.arguments.first().and_then(oxc_ast::ast::Argument::as_expression)
                    {
                        facts.specifier(argument);
                    }
                    if let Expression::StaticMemberExpression(member) = &call.callee
                        && matches!(&member.object, Expression::Identifier(id) if id.name == "Reflect")
                        && matches!(member.property.name.as_str(), "get" | "set")
                    {
                        facts.reflect_calls.push(call.span);
                    }
                }
                AstKind::TSAnyKeyword(keyword) => facts.anys.push(keyword.span),
                AstKind::UnaryExpression(unary) if unary.operator == UnaryOperator::Typeof => facts.typeofs.push(unary.span),
                AstKind::TSTypeReference(reference) => {
                    if let TSTypeName::QualifiedName(name) = &reference.type_name
                        && matches!(&name.left, TSTypeName::IdentifierReference(id) if id.name == "z")
                        && matches!(name.right.name.as_str(), "infer" | "input" | "output")
                    {
                        facts.zod_helpers.push(reference.span);
                    }
                }
                AstKind::TSUnionType(union) if union.types.iter().any(|member| matches!(member, TSType::TSNullKeyword(_) | TSType::TSUndefinedKeyword(_))) => {
                    facts.nullish_unions.push(union.span);
                }
                AstKind::IdentifierName(id) => {
                    // TypeScript represents a constructor declaration without
                    // an identifier; ordinary properties named constructor count.
                    if !matches!(semantic.nodes().parent_kind(node.id()), AstKind::MethodDefinition(method) if method.kind == MethodDefinitionKind::Constructor)
                    {
                        facts.identifier(id.span, &id.name);
                    }
                }
                AstKind::IdentifierReference(id) => facts.identifier(id.span, &id.name),
                AstKind::BindingIdentifier(id) => facts.identifier(id.span, &id.name),
                AstKind::LabelIdentifier(id) => facts.identifier(id.span, &id.name),
                AstKind::JSXIdentifier(id) => facts.identifier(id.span, &id.name),
                _ => {}
            }
        }

        // ESTree represents shorthand properties and imports with two identifier
        // nodes at the same source site; the TypeScript extractor visits one.
        facts.identifiers.sort_by_key(|(span, _)| (span.start, span.end));
        facts.identifiers.dedup();

        facts
    }

    fn identifier(&mut self, span: Span, name: &str) {
        self.identifiers.push((span, name.to_owned()));
    }

    fn specifier(&mut self, expression: &Expression<'_>) {
        match expression {
            Expression::StringLiteral(literal) => self.literal_specifier(literal),
            Expression::TemplateLiteral(template) if template.expressions.is_empty() => {
                if let Some(quasi) = template.quasis.first()
                    && let Some(value) = &quasi.value.cooked
                {
                    self.specifiers.push((template.span, module_text(value, quasi.lone_surrogates)));
                }
            }
            _ => {}
        }
    }

    fn literal_specifier(&mut self, literal: &StringLiteral<'_>) {
        self.specifiers.push((literal.span, module_text(&literal.value, literal.lone_surrogates)));
    }

    fn layout(&mut self, program: &Program<'_>) {
        self.non_barrel = program.directives.first().map(GetSpan::span);

        for statement in &program.body {
            let barrel = match statement {
                Statement::ImportDeclaration(_)
                | Statement::TSImportEqualsDeclaration(_)
                | Statement::ExportAllDeclaration(_)
                | Statement::TSExportAssignment(_)
                | Statement::ExportNamedDeclaration(_)
                | Statement::ExportFromDeclaration(_) => true,
                Statement::ExportDeclaration(export) => {
                    if matches!(&export.declaration, Declaration::ClassDeclaration(_)) {
                        self.exported_classes.push(export.span);
                    }
                    matches!(&export.declaration, Declaration::TSImportEqualsDeclaration(_))
                }
                Statement::ExportDefaultDeclaration(export) => {
                    if matches!(&export.declaration, ExportDefaultDeclarationKind::ClassDeclaration(_)) {
                        self.exported_classes.push(export.span);
                    }
                    export.declaration.as_expression().is_some()
                }
                _ => false,
            };

            if !barrel && self.non_barrel.is_none() {
                self.non_barrel = Some(statement.span());
            }
        }
    }
}

/// Go uses Unicode simple lowercase, which keeps one scalar per character.
/// Rust's full mapping expands U+0130 to an i followed by a combining dot.
pub fn simple_lowercase(value: &str) -> String {
    value.chars().filter_map(|character| character.to_lowercase().next()).collect()
}

/// The extractor's JSON reaches Go as lossy Unicode. Decode oxc's surrogate
/// escape representation to that same text before comparing module names.
fn module_text(value: &str, lone_surrogates: bool) -> String {
    if !lone_surrogates {
        return value.to_owned();
    }

    let mut text = String::new();
    let mut characters = value.chars();

    while let Some(character) = characters.next() {
        text.push(character);
        if character == '\u{fffd}' {
            for _ in 0..4 {
                characters.next();
            }
        }
    }

    text
}
