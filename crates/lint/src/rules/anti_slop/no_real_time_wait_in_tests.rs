//! `anti-slop/no-real-time-wait-in-tests`: no test waits on the real clock.
//! A sleep long enough on a fast machine is too short on a loaded runner, and
//! every sleep is time the suite spends doing nothing. Adapted from the
//! plain-JavaScript half of t3code's `t3code/no-real-time-wait-in-tests`.
//!
//! A file that turns on fake timers is left alone: its timers fire when the
//! test advances them, not when the clock does.

use oxc_ast::AstKind;
use oxc_ast::ast::Expression;
use oxc_semantic::{AstNode, Semantic};
use oxc_span::GetSpan;

use super::super::{Context, Rule};
use super::shared::member;
use super::shared::testing::is_test_file;
use super::shared::timers::{PromiseTimers, TIMER_FUNCTIONS, is_inside_promise_executor};

pub const NAME: &str = "anti-slop/no-real-time-wait-in-tests";

const MESSAGE: &str = "Real-clock wait in a test: it passes or fails with runner load. Await the promise, event, or callback that marks the milestone, or turn on fake timers and advance them.";

pub struct NoRealTimeWaitInTests;

impl Rule for NoRealTimeWaitInTests {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let AstKind::CallExpression(call) = node.kind() else {
            return;
        };

        if !is_test_file(ctx.rel) {
            return;
        }

        let timers = ctx.shared(PromiseTimers::collect);
        let waits = timers.is_call(&call.callee)
            || (timers.is_callback_timer(&call.callee, &TIMER_FUNCTIONS) && is_inside_promise_executor(ctx.semantic.nodes(), node.id()));

        if waits && !ctx.shared(FakeTimers::detect).0 {
            ctx.report(call.callee.span(), MESSAGE);
        }
    }
}

/// Whether the file calls `vi.useFakeTimers()` or `jest.useFakeTimers()`.
struct FakeTimers(bool);

impl FakeTimers {
    fn detect(semantic: &Semantic<'_>) -> Self {
        Self(semantic.nodes().iter().any(|node| {
            let AstKind::CallExpression(call) = node.kind() else {
                return false;
            };

            member(&call.callee).is_some_and(|(object, property)| {
                property == "useFakeTimers" && matches!(object, Expression::Identifier(object) if object.name == "vi" || object.name == "jest")
            })
        }))
    }
}
