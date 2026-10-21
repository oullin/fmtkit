//! `anti-slop/no-polling-loop-in-tests`: no test spins on a tick or a sleep
//! until something becomes true. How many turns that takes depends on the
//! scheduler and the runner's load, so the test is slow at best and flaky at
//! worst. Adapted from the plain-JavaScript half of t3code's
//! `t3code/no-polling-loop-in-tests`.
//!
//! A loop polls when it waits on every turn and a predicate decides when it
//! stops: an `if` that leaves the loop, or a test that is neither `true` nor a
//! counter the loop advances. A fixed number of flushes is not polling.

use oxc_ast::AstKind;
use oxc_ast::ast::{
    ArrowFunctionExpression, AssignmentExpression, AssignmentOperator, AssignmentTarget, AwaitExpression, BinaryOperator, BreakStatement, CallExpression,
    Expression, Function, IfStatement, ReturnStatement, SimpleAssignmentTarget, Statement, ThrowStatement, UpdateExpression,
};
use oxc_ast_visit::{Visit, walk};
use oxc_semantic::AstNode;
use oxc_span::GetSpan;
use oxc_syntax::scope::ScopeFlags;

use super::super::{Context, Rule};
use super::shared::strip_parens;
use super::shared::testing::is_test_file;
use super::shared::timers::PromiseTimers;

pub const NAME: &str = "anti-slop/no-polling-loop-in-tests";

const MESSAGE: &str = "Polling loop in a test: spinning on a tick or a sleep depends on the scheduler and runner load. Await the promise, event, or callback that marks the milestone, or use `expect.poll`.";

pub struct NoPollingLoopInTests;

impl Rule for NoPollingLoopInTests {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let (test, body, update) = match node.kind() {
            AstKind::ForStatement(it) => (it.test.as_ref(), &it.body, it.update.as_ref()),
            AstKind::WhileStatement(it) => (Some(&it.test), &it.body, None),
            AstKind::DoWhileStatement(it) => (Some(&it.test), &it.body, None),
            _ => return,
        };

        if !is_test_file(ctx.rel) {
            return;
        }

        let timers = ctx.shared(PromiseTimers::collect);
        let mut scan = Scan::new(&timers, None);

        scan.visit_statement(body);

        if !scan.waits {
            return;
        }

        if scan.exits_on_condition || test.is_some_and(|test| !is_true(test) && !is_counter_test(&timers, test, body, update)) {
            ctx.report(node.kind().span(), MESSAGE);
        }
    }
}

fn is_true(test: &Expression<'_>) -> bool {
    matches!(strip_parens(test), Expression::BooleanLiteral(literal) if literal.value)
}

/// A bare counter (`i < 3`, `attempt < max`) the loop itself advances bounds
/// the loop without asking anything of the code under test. `events.length < 1`,
/// or a counter nothing advances, is a predicate.
fn is_counter_test(timers: &PromiseTimers, test: &Expression<'_>, body: &Statement<'_>, update: Option<&Expression<'_>>) -> bool {
    let Expression::BinaryExpression(binary) = strip_parens(test) else {
        return false;
    };

    if !matches!(binary.operator, BinaryOperator::LessThan | BinaryOperator::LessEqualThan | BinaryOperator::GreaterThan | BinaryOperator::GreaterEqualThan) {
        return false;
    }

    [(&binary.left, &binary.right), (&binary.right, &binary.left)].into_iter().any(|(counter, bound)| {
        let Expression::Identifier(counter) = strip_parens(counter) else {
            return false;
        };

        if !matches!(strip_parens(bound), Expression::Identifier(_) | Expression::NumericLiteral(_)) {
            return false;
        }

        let mut scan = Scan::new(timers, Some(counter.name.as_str()));

        scan.visit_statement(body);

        if let Some(update) = update {
            scan.visit_expression(update);
        }

        scan.advances_counter
    })
}

/// One pass over a loop body that skips nested functions: a wait or an exit in
/// a callback runs on the callback's schedule, not the loop's.
struct Scan<'t> {
    timers: &'t PromiseTimers,
    counter: Option<&'t str>,
    waits: bool,
    exits_on_condition: bool,
    advances_counter: bool,
    /// How many `if` branches enclose the current node.
    in_branch: usize,
}

impl<'t> Scan<'t> {
    fn new(timers: &'t PromiseTimers, counter: Option<&'t str>) -> Self {
        Self { timers, counter, waits: false, exits_on_condition: false, advances_counter: false, in_branch: 0 }
    }

    fn is_counter(&self, name: &str) -> bool {
        self.counter == Some(name)
    }
}

impl<'a> Visit<'a> for Scan<'_> {
    fn visit_await_expression(&mut self, it: &AwaitExpression<'a>) {
        self.waits |= self.timers.is_tick_promise(&it.argument);
        walk::walk_await_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        // `setImmediate(…)` on its own yields a turn, the callback-style tick.
        self.waits |= self.timers.is_callback_timer(&it.callee, &["setImmediate"]);
        walk::walk_call_expression(self, it);
    }

    fn visit_if_statement(&mut self, it: &IfStatement<'a>) {
        self.visit_expression(&it.test);
        self.in_branch += 1;
        self.visit_statement(&it.consequent);

        if let Some(alternate) = &it.alternate {
            self.visit_statement(alternate);
        }

        self.in_branch -= 1;
    }

    fn visit_return_statement(&mut self, it: &ReturnStatement<'a>) {
        self.exits_on_condition |= self.in_branch > 0;
        walk::walk_return_statement(self, it);
    }

    fn visit_break_statement(&mut self, it: &BreakStatement<'a>) {
        self.exits_on_condition |= self.in_branch > 0;
        walk::walk_break_statement(self, it);
    }

    fn visit_throw_statement(&mut self, it: &ThrowStatement<'a>) {
        self.exits_on_condition |= self.in_branch > 0;
        walk::walk_throw_statement(self, it);
    }

    fn visit_update_expression(&mut self, it: &UpdateExpression<'a>) {
        if let SimpleAssignmentTarget::AssignmentTargetIdentifier(identifier) = &it.argument {
            self.advances_counter |= self.is_counter(&identifier.name);
        }

        walk::walk_update_expression(self, it);
    }

    fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
        if let AssignmentTarget::AssignmentTargetIdentifier(identifier) = &it.left
            && matches!(it.operator, AssignmentOperator::Addition | AssignmentOperator::Subtraction)
        {
            self.advances_counter |= self.is_counter(&identifier.name);
        }

        walk::walk_assignment_expression(self, it);
    }

    fn visit_function(&mut self, _: &Function<'a>, _: ScopeFlags) {}

    fn visit_arrow_function_expression(&mut self, _: &ArrowFunctionExpression<'a>) {}
}
