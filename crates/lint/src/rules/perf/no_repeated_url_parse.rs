//! `perf/no-repeated-url-parse`: no second `new URL(…)` of the same input in
//! one function, or at module level. Parse once and read every part (`origin`,
//! `hostname`, `pathname`) from that `URL`.
//!
//! Only inputs that read the same value each time count: names, property
//! reads, strings and templates without substitutions. `new URL(next())`
//! never matches another parse.

use oxc_ast::AstKind;
use oxc_ast::ast::{Argument, Expression};
use oxc_semantic::{AstNode, NodeId};
use oxc_span::{GetSpan, Span};
use rustc_hash::FxHashSet;

use super::super::anti_slop::shared::{enclosing_function, is_global, strip_parens};
use super::super::{Context, Rule};

pub const NAME: &str = "perf/no-repeated-url-parse";

const MESSAGE: &str = "This input is already parsed as a URL here. Parse it once and read every part from that `URL`.";

pub struct NoRepeatedUrlParse;

impl Rule for NoRepeatedUrlParse {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run_once(&self, ctx: &mut Context<'_, '_>) {
        let semantic = ctx.semantic;
        let source = semantic.source_text();
        let mut parsed: FxHashSet<(Option<NodeId>, &str)> = FxHashSet::default();
        let mut repeats: Vec<Span> = Vec::new();

        for node in semantic.nodes().iter() {
            let AstKind::NewExpression(new) = node.kind() else {
                continue;
            };

            let Some(input) = stable_input(&new.arguments) else {
                continue;
            };

            if !is_global(ctx, &new.callee, "URL") {
                continue;
            }

            let scope = enclosing_function(semantic.nodes(), node.id(), false).map(AstNode::id);

            if !parsed.insert((scope, &source[input.start as usize..input.end as usize])) {
                repeats.push(new.span);
            }
        }

        for span in repeats {
            ctx.report(span, MESSAGE);
        }
    }
}

/// The span of the arguments when each one reads the same value every time.
fn stable_input(arguments: &[Argument<'_>]) -> Option<Span> {
    let (first, last) = (arguments.first()?, arguments.last()?);

    arguments.iter().all(|argument| argument.as_expression().is_some_and(is_stable)).then(|| Span::new(first.span().start, last.span().end))
}

fn is_stable(expression: &Expression<'_>) -> bool {
    match strip_parens(expression) {
        Expression::Identifier(_) | Expression::StringLiteral(_) | Expression::ThisExpression(_) => true,
        Expression::TemplateLiteral(template) => template.expressions.is_empty(),
        Expression::StaticMemberExpression(member) => is_stable(&member.object),
        _ => false,
    }
}
