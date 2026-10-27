//! Facts shared by enabled boundary rules, without resolving types or imports.

use oxc_ast::AstKind;
use oxc_ast::ast::{
    Declaration, ExportDefaultDeclarationKind, Expression, ImportDeclarationSpecifier, ImportOrExportKind, MethodDefinitionKind, Program, Statement,
    TSModuleReference, TSType, TSTypeName,
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
                    facts.specifiers.push((import.source.span, import.source.value.to_string()));

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
                AstKind::ExportFromDeclaration(export) => facts.specifiers.push((export.source.span, export.source.value.to_string())),
                AstKind::ExportAllDeclaration(export) => facts.specifiers.push((export.source.span, export.source.value.to_string())),
                AstKind::TSImportEqualsDeclaration(import) => {
                    if let TSModuleReference::ExternalModuleReference(reference) = &import.module_reference {
                        facts.specifiers.push((reference.expression.span, reference.expression.value.to_string()));
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
        self.identifiers.push((span, name.to_lowercase()));
    }

    fn specifier(&mut self, expression: &Expression<'_>) {
        match expression {
            Expression::StringLiteral(literal) => self.specifiers.push((literal.span, literal.value.to_string())),
            Expression::TemplateLiteral(template) if template.expressions.is_empty() => {
                if let Some(value) = template.quasis.first().and_then(|quasi| quasi.value.cooked.as_ref()) {
                    self.specifiers.push((template.span, value.to_string()));
                }
            }
            _ => {}
        }
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
