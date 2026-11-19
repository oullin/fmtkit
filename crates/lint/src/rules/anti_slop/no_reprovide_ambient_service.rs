//! `anti-slop/no-reprovide-ambient-service`: no capturing an Effect service
//! with `const service = yield* Service` and handing it back with
//! `Effect.provideService(Service, service)`.
//!
//! Reports the `provideService` call (data-first or data-last) on an `Effect`
//! namespace imported earlier in the file, as `{ Effect }` from `effect` or a
//! namespace or default import of `effect/Effect`, when its implementation
//! argument is a single-declaration binding initialized directly by
//! `yield*` of the same service text. Skips derived implementations
//! (`configure(service)`), other services and other libraries. As upstream,
//! namespaces are matched by name, not by scope. No options.

use std::rc::Rc;

use oxc_ast::AstKind;
use oxc_ast::ast::{Argument, Expression, ImportDeclarationSpecifier};
use oxc_semantic::{AstNode, Semantic};
use oxc_span::GetSpan;

use super::super::{Context, Rule};

pub const NAME: &str = "anti-slop/no-reprovide-ambient-service";

pub struct NoReprovideAmbientService;

/// The local names bound to Effect's namespace, with where their import
/// declaration starts: upstream collects them as it walks, so a call only
/// sees the imports before it.
struct EffectNamespaces(Vec<(String, u32)>);

impl EffectNamespaces {
    fn collect(semantic: &Semantic<'_>) -> Self {
        let mut names = Vec::new();

        for node in semantic.nodes().iter() {
            let AstKind::ImportDeclaration(import) = node.kind() else {
                continue;
            };

            let Some(source) = import.source.value.as_str() else {
                continue;
            };

            for specifier in import.specifiers.iter().flatten() {
                let local = match (source, specifier) {
                    ("effect", ImportDeclarationSpecifier::ImportSpecifier(it)) if it.imported.name() == "Effect" => &it.local,
                    ("effect/Effect", ImportDeclarationSpecifier::ImportNamespaceSpecifier(it)) => &it.local,
                    ("effect/Effect", ImportDeclarationSpecifier::ImportDefaultSpecifier(it)) => &it.local,
                    _ => continue,
                };

                names.push((local.name.to_string(), import.span.start));
            }
        }

        Self(names)
    }
}

impl Rule for NoReprovideAmbientService {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let AstKind::CallExpression(call) = node.kind() else {
            return;
        };

        let Expression::StaticMemberExpression(callee) = unwrap(&call.callee) else {
            return;
        };

        let Expression::Identifier(namespace) = &callee.object else {
            return;
        };

        let [.., service, implementation] = call.arguments.as_slice() else {
            return;
        };

        if callee.property.name != "provideService" {
            return;
        }

        let (Some(service), Some(implementation)) = (Argument::as_expression(service), Argument::as_expression(implementation)) else {
            return;
        };

        let namespaces: Rc<EffectNamespaces> = ctx.shared(EffectNamespaces::collect);

        if !namespaces.0.iter().any(|(name, start)| name == namespace.name.as_str() && *start < call.span.start) {
            return;
        }

        let Some(yielded) = directly_yielded_service(ctx.semantic, implementation) else {
            return;
        };

        let source = ctx.source();
        let requested = unwrap(service).span().source_text(source);

        if yielded.span().source_text(source) == requested {
            ctx.report(
                call.span,
                format!(
                    "Do not capture \"{requested}\" from the Effect environment and re-provide it with Effect.provideService. Leave \"{requested}\" in the operation's R channel and provide its implementing Layer once at the application runtime composition root. If the runtime has an explicit service union, include \"{requested}\" there; use Layer.provideMerge when the service must remain available after layer construction. A local provide is only appropriate for a genuinely dynamic implementation, such as a client configured from per-tenant credentials."
                ),
            );
        }
    }
}

/// `expression` without parentheses, `as`, `<T>`, `!` and `satisfies`.
fn unwrap<'b, 'a>(mut expression: &'b Expression<'a>) -> &'b Expression<'a> {
    loop {
        expression = match expression {
            Expression::ParenthesizedExpression(it) => &it.expression,
            Expression::TSAsExpression(it) => &it.expression,
            Expression::TSSatisfiesExpression(it) => &it.expression,
            Expression::TSTypeAssertion(it) => &it.expression,
            Expression::TSNonNullExpression(it) => &it.expression,
            _ => return expression,
        };
    }
}

/// The service `expression` was yielded from: the binding it names has one
/// declaration, a variable declarator initialized by `yield* service`.
fn directly_yielded_service<'a>(semantic: &Semantic<'a>, expression: &Expression<'_>) -> Option<&'a Expression<'a>> {
    let Expression::Identifier(identifier) = unwrap(expression) else {
        return None;
    };

    let scoping = semantic.scoping();
    let scope = semantic.nodes().get_node(identifier.node_id.get()).scope_id();
    let symbol = scoping.find_binding(scope, identifier.name)?;

    if scoping.symbol_redeclarations(symbol).len() > 1 {
        return None;
    }

    let AstKind::VariableDeclarator(declarator) = semantic.nodes().kind(scoping.symbol_declaration(symbol)) else {
        return None;
    };

    match unwrap(declarator.init.as_ref()?) {
        Expression::YieldExpression(yielded) if yielded.delegate => yielded.argument.as_ref().map(unwrap),
        _ => None,
    }
}
