//! Timers as the test-timing rules see them: the callback timers, the promise
//! timers of `node:timers/promises`, and the promises that only wait.

use oxc_ast::AstKind;
use oxc_ast::ast::{Argument, ArrowFunctionExpression, CallExpression, Expression, Function, ImportDeclarationSpecifier};
use oxc_ast_visit::{Visit, walk};
use oxc_semantic::{AstNodes, NodeId, Semantic};
use oxc_span::GetSpan;
use oxc_syntax::scope::ScopeFlags;
use rustc_hash::FxHashSet;

use super::{ancestors, member, parent, strip_parens};

/// The timers that schedule a callback, or resolve a promise from `timers/promises`.
pub const TIMER_FUNCTIONS: [&str; 3] = ["setImmediate", "setInterval", "setTimeout"];

/// What the file imports from `timers/promises`.
#[derive(Default)]
pub struct PromiseTimers {
    /// Local names of `setTimeout` and the others: `import { setTimeout as sleep }`.
    locals: FxHashSet<String>,
    /// Local names of the module itself: `import * as timers`.
    namespaces: FxHashSet<String>,
}

impl PromiseTimers {
    pub fn collect(semantic: &Semantic<'_>) -> Self {
        let mut timers = Self::default();

        for node in semantic.nodes().iter() {
            let AstKind::ImportDeclaration(import) = node.kind() else {
                continue;
            };

            if !matches!(import.source.value.as_str(), "node:timers/promises" | "timers/promises") {
                continue;
            }

            for specifier in import.specifiers.iter().flatten() {
                match specifier {
                    ImportDeclarationSpecifier::ImportSpecifier(specifier) if TIMER_FUNCTIONS.contains(&specifier.imported.name().as_str()) => {
                        timers.locals.insert(specifier.local.name.to_string());
                    }
                    ImportDeclarationSpecifier::ImportNamespaceSpecifier(specifier) => {
                        timers.namespaces.insert(specifier.local.name.to_string());
                    }
                    ImportDeclarationSpecifier::ImportDefaultSpecifier(specifier) => {
                        timers.namespaces.insert(specifier.local.name.to_string());
                    }
                    ImportDeclarationSpecifier::ImportSpecifier(_) => {}
                }
            }
        }

        timers
    }

    /// Whether `callee` is a promise timer: `sleep(…)` or `timers.setTimeout(…)`.
    pub fn is_call(&self, callee: &Expression<'_>) -> bool {
        match strip_parens(callee) {
            Expression::Identifier(identifier) => self.locals.contains(identifier.name.as_str()),
            callee => member(callee).is_some_and(|(object, property)| {
                TIMER_FUNCTIONS.contains(&property)
                    && matches!(strip_parens(object), Expression::Identifier(object) if self.namespaces.contains(object.name.as_str()))
            }),
        }
    }

    /// Whether `callee` is a callback timer: `setTimeout`, not a promise timer of the same name.
    pub fn is_callback_timer(&self, callee: &Expression<'_>, names: &[&str]) -> bool {
        matches!(strip_parens(callee), Expression::Identifier(identifier) if names.contains(&identifier.name.as_str()) && !self.locals.contains(identifier.name.as_str()))
    }

    /// Whether `expression` is a promise that only waits: `Promise.resolve()`, a
    /// promise timer, or `new Promise` with an executor that starts a timer.
    pub fn is_tick_promise(&self, expression: &Expression<'_>) -> bool {
        const TICK_TIMERS: [&str; 2] = ["setImmediate", "setTimeout"];

        match strip_parens(expression) {
            Expression::CallExpression(call) => {
                self.is_call(&call.callee) || member(&call.callee).is_some_and(|(object, property)| property == "resolve" && is_identifier(object, "Promise"))
            }
            Expression::NewExpression(new) if is_identifier(&new.callee, "Promise") => {
                match new.arguments.first().and_then(Argument::as_expression).map(strip_parens) {
                    Some(Expression::ArrowFunctionExpression(executor)) => {
                        self.starts_timer(&TICK_TIMERS, |finder| finder.visit_arrow_function_body(&executor.body))
                    }
                    Some(Expression::FunctionExpression(executor)) => {
                        executor.body.as_deref().is_some_and(|body| self.starts_timer(&TICK_TIMERS, |finder| finder.visit_function_body(body)))
                    }
                    Some(executor) => self.is_callback_timer(executor, &TICK_TIMERS),
                    None => false,
                }
            }
            _ => false,
        }
    }

    /// Whether the body `visit` walks calls one of the callback timers `names`
    /// outside its nested functions.
    fn starts_timer(&self, names: &[&str], visit: impl FnOnce(&mut TimerCall<'_>)) -> bool {
        let mut finder = TimerCall { timers: self, names, found: false };

        visit(&mut finder);
        finder.found
    }
}

struct TimerCall<'t> {
    timers: &'t PromiseTimers,
    names: &'t [&'t str],
    found: bool,
}

impl<'a> Visit<'a> for TimerCall<'_> {
    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        self.found |= self.timers.is_callback_timer(&call.callee, self.names);
        walk::walk_call_expression(self, call);
    }

    fn visit_function(&mut self, _: &Function<'a>, _: ScopeFlags) {}

    fn visit_arrow_function_expression(&mut self, _: &ArrowFunctionExpression<'a>) {}
}

fn is_identifier(expression: &Expression<'_>, name: &str) -> bool {
    matches!(strip_parens(expression), Expression::Identifier(identifier) if identifier.name == name)
}

/// Whether the nearest function around `id` is the executor of a `new Promise(…)`.
pub fn is_inside_promise_executor(nodes: &AstNodes<'_>, id: NodeId) -> bool {
    let Some(function) = ancestors(nodes, id).find(|node| matches!(node.kind(), AstKind::Function(_) | AstKind::ArrowFunctionExpression(_))) else {
        return false;
    };

    let AstKind::NewExpression(new) = parent(nodes, function.id()).kind() else {
        return false;
    };

    is_identifier(&new.callee, "Promise") && new.arguments.first().is_some_and(|first| first.span() == function.kind().span())
}
