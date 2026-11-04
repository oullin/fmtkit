//! `perf/no-repeated-url-parse`: no second `new URL(…)` of the same input in
//! one function, or at module level. Parse once and read every part (`origin`,
//! `hostname`, `pathname`) from that `URL`.
//!
//! Only inputs that read the same value each time count: names, property
//! reads, strings and templates without substitutions. `new URL(next())`
//! never matches another parse, and neither does an input whose name is
//! assigned again anywhere. Two inputs match only when their text is the same
//! and each name in them refers to the same binding, so a loop variable reused
//! by a second loop is a different input.
//!
//! Parses in opposite arms of one `if` or `?:` never both run, so they do not
//! match. A class field initialiser is grouped with its class, not the module.

use oxc_ast::AstKind;
use oxc_ast::ast::{Argument, Expression, FunctionType};
use oxc_semantic::{AstNode, AstNodes, NodeId, Scoping};
use oxc_span::{GetSpan, Span};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;

use super::super::anti_slop::shared::{ancestors, is_global, strip_parens};
use super::super::{Context, Rule};

pub const NAME: &str = "perf/no-repeated-url-parse";

const MESSAGE: &str = "This input is already parsed as a URL here. Parse it once and read every part from that `URL`.";

/// One parse: its function or class (none at module level), its input text,
/// and the binding each argument's root name refers to.
type Parse<'s> = (Option<NodeId>, &'s str, Vec<Option<SymbolId>>);

pub struct NoRepeatedUrlParse;

impl Rule for NoRepeatedUrlParse {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run_once(&self, ctx: &mut Context<'_, '_>) {
        let semantic = ctx.semantic;
        let (source, scoping, nodes) = (semantic.source_text(), semantic.scoping(), semantic.nodes());
        let mut parsed: FxHashMap<Parse<'_>, Vec<NodeId>> = FxHashMap::default();
        let mut repeats: Vec<Span> = Vec::new();

        for node in nodes.iter() {
            let AstKind::NewExpression(new) = node.kind() else {
                continue;
            };

            let Some(input) = stable_input(&new.arguments) else {
                continue;
            };

            if !is_global(ctx, &new.callee, "URL") {
                continue;
            }

            let bindings: Vec<_> =
                new.arguments.iter().map(|argument| argument.as_expression().and_then(|expression| root_binding(scoping, expression))).collect();

            if bindings.iter().flatten().any(|&symbol| scoping.symbol_is_mutated(symbol)) {
                continue;
            }

            let earlier = parsed.entry((parse_scope(nodes, node.id()), &source[input.start as usize..input.end as usize], bindings)).or_default();

            if earlier.iter().any(|&other| !exclusive(nodes, other, node.id())) {
                repeats.push(new.span);
            }

            earlier.push(node.id());
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

/// The binding the name at the root of `expression` refers to: `base` in
/// `base` or `runtime` in `runtime.baseUrl`; none for literals, `this` and
/// globals.
fn root_binding(scoping: &Scoping, expression: &Expression<'_>) -> Option<SymbolId> {
    match strip_parens(expression) {
        Expression::Identifier(identifier) => scoping.get_reference(identifier.reference_id.get()?).symbol_id(),
        Expression::StaticMemberExpression(member) => root_binding(scoping, &member.object),
        _ => None,
    }
}

/// The function a parse runs in, or the class whose field initialiser holds
/// it; none at module level.
fn parse_scope(nodes: &AstNodes<'_>, id: NodeId) -> Option<NodeId> {
    ancestors(nodes, id)
        .find(|node| match node.kind() {
            AstKind::ArrowFunctionExpression(_) | AstKind::Class(_) => true,
            AstKind::Function(function) => matches!(function.r#type, FunctionType::FunctionDeclaration | FunctionType::FunctionExpression),
            _ => false,
        })
        .map(AstNode::id)
}

/// Whether `a` and `b` sit in opposite arms of one `if` or `?:`, so at most
/// one of them runs.
fn exclusive(nodes: &AstNodes<'_>, a: NodeId, b: NodeId) -> bool {
    let (a_span, b_span) = (nodes.get_node(a).kind().span(), nodes.get_node(b).kind().span());
    let opposite = |left: Span, right: Option<Span>| {
        right.is_some_and(|right| {
            (left.contains_inclusive(a_span) && right.contains_inclusive(b_span)) || (left.contains_inclusive(b_span) && right.contains_inclusive(a_span))
        })
    };

    ancestors(nodes, a).any(|node| match node.kind() {
        AstKind::IfStatement(statement) => opposite(statement.consequent.span(), statement.alternate.as_ref().map(GetSpan::span)),
        AstKind::ConditionalExpression(conditional) => opposite(conditional.consequent.span(), Some(conditional.alternate.span())),
        _ => false,
    })
}
