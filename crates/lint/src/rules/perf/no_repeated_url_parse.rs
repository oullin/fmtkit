//! `perf/no-repeated-url-parse`: no second `new URL(…)` of the same input in
//! one function, or at module level. Parse once and read every part (`origin`,
//! `hostname`, `pathname`) from that `URL`.
//!
//! Only inputs that read the same value each time count: names, property
//! reads, strings and templates without substitutions. `new URL(next())`
//! never matches another parse, and neither does an input whose name, or a
//! property path it reads, is assigned anywhere in the file. Two inputs match only when their text is the same
//! and each name in them refers to the same binding, so a loop variable reused
//! by a second loop is a different input.
//!
//! Parses in opposite arms of one `if` or `?:` never both run, so they do not
//! match, unless a loop in the same function repeats the branch. A class field
//! initialiser is grouped with its class, static and instance fields apart,
//! not with the module.

use oxc_ast::AstKind;
use oxc_ast::ast::{Argument, AssignmentTarget, Expression, FunctionType};
use oxc_semantic::{AstNodes, NodeId, Scoping};
use oxc_span::{GetSpan, Span};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

use super::super::anti_slop::shared::{ancestors, is_global, strip_parens};
use super::super::{Context, Rule};

pub const NAME: &str = "perf/no-repeated-url-parse";

const MESSAGE: &str = "This input is already parsed as a URL here. Parse it once and read every part from that `URL`.";

/// One parse: its function or class (none at module level) and whether it is a
/// static member's, its input text, and the binding each argument's root name
/// refers to.
type Parse<'s> = (Option<(NodeId, bool)>, &'s str, Vec<Option<SymbolId>>);

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
        let written = written_members(nodes, source);

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

            if bindings.iter().flatten().any(|&symbol| scoping.symbol_is_mutated(symbol)) || reads_written(&new.arguments, source, &written) {
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

/// The text of every property path the file assigns to: `cfg.url` in
/// `cfg.url = next`.
fn written_members<'s>(nodes: &AstNodes<'_>, source: &'s str) -> FxHashSet<&'s str> {
    nodes
        .iter()
        .filter_map(|node| match node.kind() {
            AstKind::AssignmentExpression(assignment) => match &assignment.left {
                AssignmentTarget::StaticMemberExpression(target) => Some(&source[target.span.start as usize..target.span.end as usize]),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

/// Whether an argument reads a property path the file assigns to, or a
/// property below one.
fn reads_written(arguments: &[Argument<'_>], source: &str, written: &FxHashSet<&str>) -> bool {
    arguments.iter().any(|argument| {
        let text = &source[argument.span().start as usize..argument.span().end as usize];

        written.iter().any(|path| text.strip_prefix(path).is_some_and(|rest| rest.is_empty() || rest.starts_with('.')))
    })
}

/// Whether `kind` starts a scope of its own: a function, or a class whose field
/// initialisers run apart from the module.
fn is_boundary(kind: AstKind<'_>) -> bool {
    match kind {
        AstKind::ArrowFunctionExpression(_) | AstKind::Class(_) => true,
        AstKind::Function(function) => matches!(function.r#type, FunctionType::FunctionDeclaration | FunctionType::FunctionExpression),
        _ => false,
    }
}

/// The function a parse runs in, or the class whose field initialiser or
/// static block holds it, with whether that is a static member; none at
/// module level.
fn parse_scope(nodes: &AstNodes<'_>, id: NodeId) -> Option<(NodeId, bool)> {
    let mut is_static = false;

    for node in ancestors(nodes, id) {
        match node.kind() {
            AstKind::PropertyDefinition(property) => is_static = property.r#static,
            AstKind::StaticBlock(_) => is_static = true,
            AstKind::Class(_) => return Some((node.id(), is_static)),
            kind if is_boundary(kind) => return Some((node.id(), false)),
            _ => {}
        }
    }

    None
}

/// Whether `a` and `b` sit in opposite arms of one `if` or `?:` that no loop
/// in the same function repeats, so at most one of them runs.
fn exclusive(nodes: &AstNodes<'_>, a: NodeId, b: NodeId) -> bool {
    let (a_span, b_span) = (nodes.get_node(a).kind().span(), nodes.get_node(b).kind().span());
    let opposite = |left: Span, right: Option<Span>| {
        right.is_some_and(|right| {
            (left.contains_inclusive(a_span) && right.contains_inclusive(b_span)) || (left.contains_inclusive(b_span) && right.contains_inclusive(a_span))
        })
    };

    ancestors(nodes, a).any(|node| {
        let branches = match node.kind() {
            AstKind::IfStatement(statement) => opposite(statement.consequent.span(), statement.alternate.as_ref().map(GetSpan::span)),
            AstKind::ConditionalExpression(conditional) => opposite(conditional.consequent.span(), Some(conditional.alternate.span())),
            _ => false,
        };

        branches && !in_loop(nodes, node.id())
    })
}

/// Whether a loop in the same function holds `id`.
fn in_loop(nodes: &AstNodes<'_>, id: NodeId) -> bool {
    ancestors(nodes, id).take_while(|node| !is_boundary(node.kind())).any(|node| {
        matches!(
            node.kind(),
            AstKind::ForStatement(_) | AstKind::ForInStatement(_) | AstKind::ForOfStatement(_) | AstKind::WhileStatement(_) | AstKind::DoWhileStatement(_)
        )
    })
}
