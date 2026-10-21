//! `anti-slop/no-unreasoned-test-skip`: a skipped or to-do test links the
//! issue that tracks it in a comment above, so it does not stay skipped
//! forever. Adapted from the skip half of t3code's
//! `t3code/no-test-timeout-or-unreasoned-skip`.

use oxc_ast::AstKind;
use oxc_ast::ast::{Argument, Expression, ObjectPropertyKind};
use oxc_semantic::AstNode;
use oxc_span::{GetSpan, Span};

use super::super::{Context, Rule};
use super::shared::testing::{is_test_chain, is_test_file};
use super::shared::{ancestors, comment_value, comments_before, links_issue, strip_parens};

pub const NAME: &str = "anti-slop/no-unreasoned-test-skip";

const MESSAGE: &str = "Say why this test is skipped: link the issue that tracks it (`#123` or a URL) in a comment above it.";

/// The modifiers that keep a test from running: `it.skip`, `it.todo`.
const SKIP_MODIFIERS: [&str; 2] = ["skip", "todo"];

/// The Jasmine-style names for a skipped test or suite.
const SKIP_FUNCTIONS: [&str; 3] = ["xdescribe", "xit", "xtest"];

pub struct NoUnreasonedTestSkip;

impl Rule for NoUnreasonedTestSkip {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        if !is_test_file(ctx.rel) {
            return;
        }

        let span = match node.kind() {
            AstKind::CallExpression(call) => match strip_parens(&call.callee) {
                Expression::Identifier(identifier) if SKIP_FUNCTIONS.contains(&identifier.name.as_str()) => identifier.span,
                // `test(name, { skip: true }, fn)` is the options spelling of `test.skip`. A computed
                // value such as `{ skip: isWindows }` is a platform gate, like `skipIf`, and stays allowed.
                callee if is_test_chain(callee) && call.arguments.iter().skip(1).take(2).any(skips_by_option) => call.span,
                _ => return,
            },
            AstKind::StaticMemberExpression(member) if SKIP_MODIFIERS.contains(&member.property.name.as_str()) && is_test_chain(&member.object) => member.span,
            // `it["skip"](…)` is the same registration; only a dynamic key is unknowable here.
            AstKind::ComputedMemberExpression(member)
                if matches!(strip_parens(&member.expression), Expression::StringLiteral(key) if SKIP_MODIFIERS.contains(&key.value.as_str()))
                    && is_test_chain(&member.object) =>
            {
                member.span
            }
            _ => return,
        };

        if !is_tracked(ctx, node) {
            ctx.report(span, MESSAGE);
        }
    }
}

/// Whether `argument` is an options object with `skip: true` or `todo: true`.
fn skips_by_option(argument: &Argument<'_>) -> bool {
    let Some(Expression::ObjectExpression(object)) = argument.as_expression().map(strip_parens) else {
        return false;
    };

    object.properties.iter().any(|property| {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            return false;
        };

        property.key.static_name().is_some_and(|key| SKIP_MODIFIERS.contains(&key.as_ref()))
            && matches!(strip_parens(&property.value), Expression::BooleanLiteral(value) if value.value)
    })
}

/// Whether a comment directly above the statement holding `node` links an issue.
fn is_tracked(ctx: &Context<'_, '_>, node: &AstNode<'_>) -> bool {
    let statement = std::iter::once(node).chain(ancestors(ctx.semantic.nodes(), node.id())).find_map(|node| statement_span(node.kind()));
    let Some(statement) = statement else {
        return false;
    };

    let source = ctx.source();

    comments_before(ctx.semantic, statement.start).iter().any(|comment| links_issue(comment_value(source, comment)))
}

/// The span of `kind` when it is a statement or a declaration.
fn statement_span(kind: AstKind<'_>) -> Option<Span> {
    (kind.is_statement() || kind.is_declaration()).then(|| kind.span())
}
