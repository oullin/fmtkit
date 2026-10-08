//! `@nkzw/eslint-plugin` 2.0.0: `ensure-relay-types`, `no-instanceof` and
//! `require-use-effect-arguments`.

use oxc_ast::AstKind;
use oxc_ast::ast::{BinaryOperator, CallExpression, Expression, ImportDeclarationSpecifier, MemberExpression, ModuleExportName, Statement};
use oxc_semantic::AstNode;

use super::{Context, Factory, Rule};

pub const RULES: &[(&str, Factory)] = &[
    (EnsureRelayTypes::NAME, |options| no_options(options, EnsureRelayTypes)),
    (NoInstanceof::NAME, |options| no_options(options, NoInstanceof)),
    (RequireUseEffectArguments::NAME, |options| no_options(options, RequireUseEffectArguments)),
];

/// The plugin's rules declare no schema, which ESLint 9 treats as accepting no options.
fn no_options(options: &[serde_json::Value], rule: impl Rule + 'static) -> Result<Box<dyn Rule>, String> {
    if options.is_empty() { Ok(Box::new(rule)) } else { Err("takes no options".into()) }
}

/// Local names bound by the `import` declarations that come before `before`,
/// as ESLint sees them while walking the file in order.
fn imported_names<'a>(
    ctx: &Context<'_, 'a>,
    before: u32,
    module: &str,
    mut name: impl FnMut(&ImportDeclarationSpecifier<'a>) -> Option<String>,
) -> Vec<String> {
    let program = ctx.semantic.nodes().program();
    let mut names = Vec::new();

    for statement in &program.body {
        if let Statement::ImportDeclaration(decl) = statement
            && decl.span.start < before
            && decl.source.value == module
        {
            names.extend(decl.specifiers.iter().flatten().filter_map(&mut name));
        }
    }

    names
}

/// The imported name of `{ imported as local }`, when it is an identifier.
fn imported_identifier<'s>(specifier: &'s ImportDeclarationSpecifier<'_>) -> Option<(&'s str, &'s str)> {
    let ImportDeclarationSpecifier::ImportSpecifier(specifier) = specifier else { return None };
    let imported = match &specifier.imported {
        ModuleExportName::IdentifierName(id) => id.name.as_str(),
        ModuleExportName::IdentifierReference(id) => id.name.as_str(),
        ModuleExportName::StringLiteral(_) => return None,
    };

    Some((imported, specifier.local.name.as_str()))
}

fn callee_identifier<'s>(call: &'s CallExpression<'_>) -> Option<&'s str> {
    match call.callee.without_parentheses() {
        Expression::Identifier(id) => Some(id.name.as_str()),
        _ => None,
    }
}

/// `@nkzw/ensure-relay-types`: Relay's `useMutation` and `usePaginationFragment`
/// need explicit type arguments.
pub struct EnsureRelayTypes;

impl EnsureRelayTypes {
    pub const NAME: &str = "@nkzw/ensure-relay-types";
}

impl Rule for EnsureRelayTypes {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let AstKind::CallExpression(call) = node.kind() else { return };
        let Some(name) = callee_identifier(call) else { return };

        if call.type_arguments.is_some() {
            return;
        }

        let tracked = imported_names(ctx, call.span.start, "react-relay/hooks.js", |specifier| {
            imported_identifier(specifier).filter(|(imported, _)| matches!(*imported, "useMutation" | "usePaginationFragment")).map(|(_, local)| local.to_owned())
        });

        if tracked.iter().any(|local| local == name) {
            ctx.report(call.span, format!("`{name}` calls must have type parameters."));
        }
    }
}

/// `@nkzw/no-instanceof`: `instanceof` only for errors and exceptions.
pub struct NoInstanceof;

impl NoInstanceof {
    pub const NAME: &str = "@nkzw/no-instanceof";
}

impl Rule for NoInstanceof {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let AstKind::BinaryExpression(binary) = node.kind() else { return };

        if binary.operator != BinaryOperator::Instanceof {
            return;
        }

        if let Expression::Identifier(right) = binary.right.without_parentheses()
            && (right.name.ends_with("Error") || right.name.ends_with("Exception"))
        {
            return;
        }

        ctx.report(binary.span, "The \"instanceof\" operator is not allowed.");
    }
}

/// `@nkzw/require-use-effect-arguments`: React's `useEffect` needs its dependency array.
pub struct RequireUseEffectArguments;

impl RequireUseEffectArguments {
    pub const NAME: &str = "@nkzw/require-use-effect-arguments";
}

impl Rule for RequireUseEffectArguments {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let AstKind::CallExpression(call) = node.kind() else { return };

        if call.arguments.len() >= 2 {
            return;
        }

        let name = match call.callee.without_parentheses() {
            Expression::Identifier(id) => id.name.to_string(),
            callee => match callee.as_member_expression() {
                Some(MemberExpression::StaticMemberExpression(member)) => match member.object.without_parentheses() {
                    Expression::Identifier(object) => format!("{}.{}", object.name, member.property.name),
                    _ => return,
                },
                Some(MemberExpression::ComputedMemberExpression(member)) => match (member.object.without_parentheses(), member.expression.without_parentheses()) {
                    (Expression::Identifier(object), Expression::Identifier(property)) => format!("{}.{}", object.name, property.name),
                    _ => return,
                },
                _ => return,
            },
        };

        let tracked = imported_names(ctx, call.span.start, "react", |specifier| match specifier {
            ImportDeclarationSpecifier::ImportDefaultSpecifier(default) => Some(format!("{}.useEffect", default.local.name)),
            specifier => imported_identifier(specifier).filter(|(imported, _)| *imported == "useEffect").map(|(_, local)| local.to_owned()),
        });

        if tracked.contains(&name) {
            ctx.report(call.span, format!("{name} must be called with a second argument (dependency array)."));
        }
    }
}
