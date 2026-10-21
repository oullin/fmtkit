//! `anti-slop/no-hookless-use-function`: a `use*` function calls a hook. The
//! React Compiler only compiles functions that do, so a hookless `useX` is
//! skipped and rebuilds what it returns on every render. Adapted from
//! t3code's `t3code/no-hookless-use-function`, after eslint-react's
//! `no-unnecessary-use-prefix`.
//!
//! Vue and other frameworks name plain composables `useX` too, so the rule
//! only reads React files: `.jsx` and `.tsx` files, and files importing
//! `react`. A `.vue` file never is one.

use std::ffi::OsStr;
use std::path::Path;

use oxc_ast::AstKind;
use oxc_ast::ast::{ArrowFunctionExpression, BindingPattern, CallExpression, Expression, Function};
use oxc_ast_visit::{Visit, walk};
use oxc_semantic::{AstNode, Semantic};
use oxc_syntax::scope::ScopeFlags;

use super::super::{Context, Rule};
use super::shared::strip_parens;

pub const NAME: &str = "anti-slop/no-hookless-use-function";

pub struct NoHooklessUseFunction;

impl Rule for NoHooklessUseFunction {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let (name, span, calls_hook) = match node.kind() {
            AstKind::Function(function) if function.is_declaration() => {
                let Some(id) = &function.id else {
                    return;
                };

                (id.name.as_str(), id.span, function.body.as_deref().is_some_and(|body| HookCall::find(|finder| finder.visit_function_body(body))))
            }
            AstKind::VariableDeclarator(declarator) => {
                let (BindingPattern::BindingIdentifier(id), Some(init)) = (&declarator.id, &declarator.init) else {
                    return;
                };

                let calls_hook = match init {
                    Expression::ArrowFunctionExpression(arrow) => HookCall::find(|finder| finder.visit_arrow_function_body(&arrow.body)),
                    Expression::FunctionExpression(function) => {
                        function.body.as_deref().is_some_and(|body| HookCall::find(|finder| finder.visit_function_body(body)))
                    }
                    _ => return,
                };

                (id.name.as_str(), id.span, calls_hook)
            }
            _ => return,
        };

        if is_hook_name(name) && !calls_hook && is_react_file(ctx) {
            ctx.report(
                span,
                format!(
                    "`{name}` calls no hooks, so the React Compiler skips it and whatever it returns is rebuilt on every render. Rename it to a plain function, or call the hook it depends on inside it."
                ),
            );
        }
    }
}

/// Whether the file is React code; see the module docs.
fn is_react_file(ctx: &mut Context<'_, '_>) -> bool {
    match Path::new(ctx.rel).extension().and_then(OsStr::to_str) {
        Some("vue") => false,
        Some("jsx" | "tsx") => true,
        _ => ctx.shared(ImportsReact::detect).0,
    }
}

/// Whether the file imports `react` or one of its subpaths.
struct ImportsReact(bool);

impl ImportsReact {
    fn detect(semantic: &Semantic<'_>) -> Self {
        Self(semantic.nodes().iter().any(
            |node| matches!(node.kind(), AstKind::ImportDeclaration(import) if import.source.value == "react" || import.source.value.starts_with("react/")),
        ))
    }
}

/// `useX` as a hook name: `use` then an uppercase letter.
fn is_hook_name(name: &str) -> bool {
    name.strip_prefix("use").is_some_and(|rest| rest.starts_with(|c: char| c.is_ascii_uppercase()))
}

/// Whether `callee` calls a hook: `useX(…)`, React 19's bare `use(…)`, or `x.useX(…)`.
fn is_hook_callee(callee: &Expression<'_>) -> bool {
    let name = match strip_parens(callee) {
        Expression::Identifier(identifier) => identifier.name.as_str(),
        Expression::StaticMemberExpression(member) => member.property.name.as_str(),
        _ => return false,
    };

    name == "use" || is_hook_name(name)
}

/// Finds a hook call outside nested functions: a hook called in a callback
/// runs when the callback does, not while rendering.
struct HookCall(bool);

impl HookCall {
    fn find(visit: impl FnOnce(&mut Self)) -> bool {
        let mut finder = Self(false);

        visit(&mut finder);
        finder.0
    }
}

impl<'a> Visit<'a> for HookCall {
    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        if is_hook_callee(&call.callee) {
            self.0 = true;
        } else {
            walk::walk_call_expression(self, call);
        }
    }

    fn visit_function(&mut self, _: &Function<'a>, _: ScopeFlags) {}

    fn visit_arrow_function_expression(&mut self, _: &ArrowFunctionExpression<'a>) {}
}
