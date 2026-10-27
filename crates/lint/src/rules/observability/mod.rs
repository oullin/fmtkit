//! Opt-in catch policies: require evidence of handling caught errors and reject
//! promise handlers that explicitly discard their result.

mod catches;
mod handlers;
mod options;
mod shared;

use oxc_ast::AstKind;
use oxc_semantic::AstNode;
use oxc_span::Span;

use super::{Context, Factory, Rule};
use options::Options;

pub const RULES: &[(&str, Factory)] = &[
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
