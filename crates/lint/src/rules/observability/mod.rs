//! Opt-in catch policies: require handling evidence, reject discarded promise
//! errors, and restrict catches to failure boundaries.

mod catches;
mod handlers;
mod options;
mod shared;

use oxc_ast::AstKind;
use oxc_semantic::AstNode;
use oxc_span::Span;

use super::{Context, Factory, Rule};
use options::{BoundaryOptions, Options};

pub const RULES: &[(&str, Factory)] = &[
    (NoCatchOutsideBoundary::NAME, |options| Ok(Box::new(NoCatchOutsideBoundary(BoundaryOptions::parse(options)?)))),
    (NoUnobservedCatch::NAME, |options| Ok(Box::new(NoUnobservedCatch(Options::parse(options, true)?)))),
    (NoSwallowingPromiseCatch::NAME, |options| Ok(Box::new(NoSwallowingPromiseCatch(Options::parse(options, false)?)))),
];

struct NoUnobservedCatch(Options);

impl NoUnobservedCatch {
    const NAME: &str = "observability/no-unobserved-catch";
}

impl Rule for NoUnobservedCatch {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let AstKind::CatchClause(clause) = node.kind() else { return };

        if self.0.ignore.is_match(ctx.rel) || shared::escaped(ctx, clause.span.start) || catches::handled(clause, &self.0.observers, ctx.semantic) {
            return;
        }

        ctx.report(Span::new(clause.span.start, clause.span.start + 5), "Catch must report, throw, or carry the caught error into an outcome with errorClass");
    }
}

struct NoSwallowingPromiseCatch(Options);

impl NoSwallowingPromiseCatch {
    const NAME: &str = "observability/no-swallowing-promise-catch";
}

impl Rule for NoSwallowingPromiseCatch {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let AstKind::CallExpression(call) = node.kind() else { return };
        let Some((_, "catch", span)) = shared::member(&call.callee) else { return };

        if self.0.ignore.is_match(ctx.rel) || shared::escaped(ctx, span.start) {
            return;
        }

        let Some(handler) = call.arguments.first().and_then(oxc_ast::ast::Argument::as_expression) else { return };

        if handlers::swallows(handler, ctx.semantic) {
            ctx.report(span, "Promise catch handler must not be empty or return undefined");
        }
    }
}

struct NoCatchOutsideBoundary(BoundaryOptions);

impl NoCatchOutsideBoundary {
    const NAME: &str = "observability/no-catch-outside-boundary";
}

impl Rule for NoCatchOutsideBoundary {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let span = match node.kind() {
            AstKind::CatchClause(clause) => {
                let caught = clause.param.as_ref().map(|param| handlers::bound_symbols(&param.pattern)).unwrap_or_default();

                if self.0.translate && handlers::translates_statements(&clause.body.body, &caught, ctx.semantic) {
                    return;
                }

                Span::new(clause.span.start, clause.span.start + 5)
            }
            AstKind::CallExpression(call) => {
                let Some((receiver, "catch", span)) = shared::member(&call.callee) else { return };
                let Some(handler) = call.arguments.first() else { return };

                if handlers::zod_schema(receiver, ctx.semantic) || handler.as_expression().is_some_and(|handler| handlers::non_callable(handler, ctx.semantic))
                {
                    return;
                }

                if self.0.translate && handler.as_expression().is_some_and(|handler| handlers::translates(handler, ctx.semantic)) {
                    return;
                }

                span
            }
            _ => return,
        };

        if self.0.allow.matches(ctx.rel) || self.0.boundaries.as_ref().is_some_and(|boundaries| shared::imports_boundary(ctx, boundaries)) {
            return;
        }

        if !shared::escaped(ctx, span.start) {
            ctx.report(span, "A catch outside a failure boundary: raise an OperationalFailure and let the edge observe it.");
        }
    }
}
