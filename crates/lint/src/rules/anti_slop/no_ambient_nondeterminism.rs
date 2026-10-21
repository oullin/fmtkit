//! `anti-slop/no-ambient-nondeterminism`: no reading the clock or the entropy
//! source; deterministic code takes time and randomness from its caller.
//!
//! A reading passed to a mock (`vi.setSystemTime(Date.now())`, or inside a
//! `mockReturnValue` or `vi.mock` factory) seeds fake time rather than measures
//! the run, so it is allowed, as in t3code's `t3code/no-wall-clock-in-tests`.

use oxc_ast::AstKind;
use oxc_ast::ast::Expression;
use oxc_semantic::AstNode;
use oxc_span::GetSpan;

use super::super::{Context, Rule};
use super::shared::testing::is_test_framework_object;
use super::shared::{ancestors, is_global, is_global_member, member};

pub const NAME: &str = "anti-slop/no-ambient-nondeterminism";

/// Global readings of the clock and the entropy source: owner, property and display name.
const AMBIENT_MEMBERS: [(&str, &str, &str); 6] = [
    ("Date", "now", "Date.now()"),
    ("Math", "random", "Math.random()"),
    ("performance", "now", "performance.now()"),
    ("crypto", "randomUUID", "crypto.randomUUID()"),
    ("crypto", "getRandomValues", "crypto.getRandomValues()"),
    ("process", "hrtime", "process.hrtime()"),
];

/// Methods of a mock function that set what it returns.
const MOCK_METHODS: [&str; 6] =
    ["mockImplementation", "mockImplementationOnce", "mockResolvedValue", "mockResolvedValueOnce", "mockReturnValue", "mockReturnValueOnce"];

/// Members of `vi` and `jest` that build a mock or set fake time.
const FRAMEWORK_MOCKS: [&str; 5] = ["doMock", "fn", "mock", "setSystemTime", "spyOn"];

pub struct NoAmbientNondeterminism;

impl Rule for NoAmbientNondeterminism {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        match node.kind() {
            AstKind::CallExpression(call) => {
                if matches!(call.callee, Expression::Super(_)) {
                    return;
                }

                let source = AMBIENT_MEMBERS
                    .iter()
                    .find(|(owner, property, _)| is_global_member(ctx, &call.callee, owner, property))
                    .map(|(_, _, source)| *source)
                    .or_else(|| is_hrtime_bigint(ctx, &call.callee).then_some("process.hrtime.bigint()"));

                if let Some(source) = source
                    && !feeds_mock(ctx, node)
                {
                    ctx.report(
                        call.span,
                        format!("`{source}` makes this output depend on when and where it ran. Take the value as a parameter so the caller owns it."),
                    );
                }
            }
            AstKind::NewExpression(new) if new.arguments.is_empty() && is_global(ctx, &new.callee, "Date") && !feeds_mock(ctx, node) => {
                ctx.report(new.span, "`new Date()` without an argument reads the ambient clock. Take the instant as a parameter so the caller owns it.");
            }
            _ => {}
        }
    }
}

/// `process.hrtime.bigint`, off the unshadowed global `process`.
fn is_hrtime_bigint(ctx: &Context<'_, '_>, callee: &Expression<'_>) -> bool {
    member(callee).is_some_and(|(object, property)| property == "bigint" && is_global_member(ctx, object, "process", "hrtime"))
}

/// Whether `node` is inside the arguments of a call that builds a mock or sets fake time.
fn feeds_mock(ctx: &Context<'_, '_>, node: &AstNode<'_>) -> bool {
    let span = node.kind().span();

    ancestors(ctx.semantic.nodes(), node.id()).any(|ancestor| {
        let AstKind::CallExpression(call) = ancestor.kind() else {
            return false;
        };

        !call.callee.span().contains_inclusive(span)
            && member(&call.callee).is_some_and(|(object, property)| {
                MOCK_METHODS.contains(&property) || (FRAMEWORK_MOCKS.contains(&property) && is_test_framework_object(ctx, object))
            })
    })
}
