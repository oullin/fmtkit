//! `anti-slop-martinfowler/loops`: loops that build a collection declared
//! outside the loop, by calling `.push`/`.unshift`/`.set`/`.add` on it,
//! assigning computed keys on it, or reassigning it to a spread or `.concat`
//! of itself.
//!
//! `for`, `for…in` and `for…of` loops are checked; `while` and `do…while`
//! only with `includeWhile`. A loop is not a transform, and is skipped, when:
//! - it exits early: a `return`, or a `break` that leaves it (outside nested
//!   functions);
//! - it writes any other outer binding (`total += x`, `state.done = true`):
//!   it is a state machine;
//! - the collection is the one it iterates (a worklist), or is declared
//!   inside it;
//! - the write is an index write inside a counted `for (;;)` loop (an
//!   in-place buffer or matrix update);
//! - an enclosing loop that qualifies, inside the collection's scope, is
//!   reported for it instead.
//!
//! One finding per loop, for the first collection found, scanning the body
//! the way upstream does (last statement first).
//!
//! Options: `{ ignoreTests?: boolean = true, includeWhile?: boolean = false }`.
//! Test files (upstream's `isTestFile`) are skipped unless `ignoreTests` is
//! `false`.

use oxc_ast::AstKind;
use oxc_ast::ast::{
    ArrayExpressionElement, ArrowFunctionExpression, AssignmentOperator, AssignmentTarget, Expression, Function, IdentifierReference, ObjectPropertyKind,
    Statement,
};
use oxc_ast_visit::Visit;
use oxc_semantic::{AstNode, AstNodes, NodeId, Semantic};
use oxc_span::{GetSpan, Span};
use oxc_syntax::scope::ScopeFlags;
use serde_json::Value;

use super::super::anti_slop::shared::{ancestors, parent, strip_parens};
use super::super::{Context, Rule};
use super::shared::{Member, Options, binding_span, expression, is_test_file, simple_target, unwrap};

pub const NAME: &str = "anti-slop-martinfowler/loops";

const COLLECTING_METHODS: [&str; 4] = ["push", "unshift", "set", "add"];

pub fn build(options: &[Value]) -> Result<Box<dyn Rule>, String> {
    let options = Options::parse(options, &["ignoreTests", "includeWhile"])?;

    Ok(Box::new(Loops { ignore_tests: options.boolean("ignoreTests", true)?, include_while: options.boolean("includeWhile", false)? }))
}

struct Loops {
    ignore_tests: bool,
    include_while: bool,
}

/// The parts of a loop the rule reads.
struct Loop<'a> {
    body: &'a Statement<'a>,
    /// What it iterates or tests: a `for…of`/`for…in` right side, else the test.
    iterated: Option<&'a Expression<'a>>,
    is_while: bool,
}

fn as_loop(kind: AstKind<'_>) -> Option<Loop<'_>> {
    match kind {
        AstKind::ForStatement(it) => Some(Loop { body: &it.body, iterated: it.test.as_ref(), is_while: false }),
        AstKind::ForInStatement(it) => Some(Loop { body: &it.body, iterated: Some(&it.right), is_while: false }),
        AstKind::ForOfStatement(it) => Some(Loop { body: &it.body, iterated: Some(&it.right), is_while: false }),
        AstKind::WhileStatement(it) => Some(Loop { body: &it.body, iterated: Some(&it.test), is_while: true }),
        AstKind::DoWhileStatement(it) => Some(Loop { body: &it.body, iterated: Some(&it.test), is_while: true }),
        _ => None,
    }
}

fn is_loop(kind: AstKind<'_>) -> bool {
    matches!(
        kind,
        AstKind::ForStatement(_) | AstKind::ForInStatement(_) | AstKind::ForOfStatement(_) | AstKind::WhileStatement(_) | AstKind::DoWhileStatement(_)
    )
}

fn contains(outer: Span, inner: Span) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

impl Loops {
    fn qualifies(&self, nodes: &AstNodes<'_>, node: &AstNode<'_>) -> bool {
        as_loop(node.kind()).is_some_and(|it| (self.include_while || !it.is_while) && !has_early_exit(nodes, node, it.body))
    }
}

impl Rule for Loops {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let Some(it) = as_loop(node.kind()) else {
            return;
        };

        let semantic = ctx.semantic;
        let nodes = semantic.nodes();

        if (self.ignore_tests && is_test_file(ctx.rel)) || !self.qualifies(nodes, node) {
            return;
        }

        let loop_span = node.kind().span();
        let is_outer =
            |identifier: &IdentifierReference<'_>| declaration_span(semantic, identifier).is_none_or(|declaration| !contains(loop_span, declaration));

        // Writes to other outer state make the loop a state machine, not a transform.
        let mut writes = OuterWrites { names: Vec::new() };

        writes.visit_statement(it.body);

        let mut written: Vec<&str> = writes.names.into_iter().filter(|identifier| is_outer(identifier)).map(|identifier| identifier.name.as_str()).collect();

        written.sort_unstable();
        written.dedup();

        let mut collector = Accumulations { sites: Vec::new() };

        collector.visit_statement(it.body);

        // Upstream pops a stack of nodes: each node, then its children last
        // to first. That is postorder reversed.
        for site in collector.sites.iter().rev() {
            let target = site.target.name.as_str();

            if written.iter().any(|name| *name != target) {
                return;
            }

            // Index writes inside counted `for` loops are in-place buffer or matrix updates.
            if matches!(site.how, How::Indexed) && inside_counted_loop(nodes, site.node, node.id()) {
                continue;
            }

            let Some(declaration) = declaration_span(semantic, site.target) else {
                continue;
            };

            if contains(loop_span, declaration) {
                continue;
            }

            // A worklist loop that grows the collection it iterates is not a pipeline.
            if it.iterated.is_some_and(|iterated| references_name(iterated, target)) {
                continue;
            }

            let reported_by_outer_loop = ancestors(nodes, node.id())
                .take_while(|ancestor| !contains(ancestor.kind().span(), declaration))
                .any(|ancestor| is_loop(ancestor.kind()) && self.qualifies(nodes, ancestor));

            if reported_by_outer_loop {
                continue;
            }

            let how = match site.how {
                How::Method(method) => format!("calling `.{method}`"),
                How::Indexed => "assigning computed keys".to_owned(),
                How::Spread => "copying it with a spread".to_owned(),
                How::Concat => "calling `.concat`".to_owned(),
            };

            ctx.report(
                loop_span,
                format!(
                    "Possibly Loops: this loop builds `{target}` by {how} on each iteration, so the transformation is hidden inside control flow and mutation. Consider Replace Loop with Pipeline: express it as `map`/`filter`/`flatMap`/`groupBy` over the source (Effect's `Arr` and `Record` modules, or `Object.fromEntries`), or `Effect.forEach`/`Stream` when each iteration runs an effect."
                ),
            );

            return;
        }
    }
}

/// The span of the first identifier declaring the binding `identifier`
/// resolves to by name, from its own scope outward; none for a global.
fn declaration_span(semantic: &Semantic<'_>, identifier: &IdentifierReference<'_>) -> Option<Span> {
    let scope = semantic.nodes().get_node(identifier.node_id.get()).scope_id();
    let symbol = semantic.scoping().find_binding(scope, identifier.name)?;

    Some(binding_span(semantic, symbol))
}

/// `return`, or a `break` that leaves this loop, means the loop is a search,
/// not a transform.
fn has_early_exit(nodes: &AstNodes<'_>, node: &AstNode<'_>, body: &Statement<'_>) -> bool {
    let label = match parent(nodes, node.id()).kind() {
        AstKind::LabeledStatement(labeled) => Some(labeled.label.name.as_str()),
        _ => None,
    };

    let mut exit = EarlyExit { label, breakable: 0, found: false };

    exit.visit_statement(body);
    exit.found
}

struct EarlyExit<'l> {
    label: Option<&'l str>,
    /// How many loops and switches enclose the current node, inside the body.
    breakable: usize,
    found: bool,
}

impl<'a> Visit<'a> for EarlyExit<'_> {
    fn enter_node(&mut self, kind: AstKind<'a>) {
        match kind {
            AstKind::ReturnStatement(_) => self.found = true,
            AstKind::BreakStatement(statement) => {
                let leaves = match &statement.label {
                    None => self.breakable == 0,
                    Some(label) => Some(label.name.as_str()) == self.label || self.breakable == 0,
                };

                self.found |= leaves;
            }
            _ => {}
        }

        if is_loop(kind) || matches!(kind, AstKind::SwitchStatement(_)) {
            self.breakable += 1;
        }
    }

    fn leave_node(&mut self, kind: AstKind<'a>) {
        if is_loop(kind) || matches!(kind, AstKind::SwitchStatement(_)) {
            self.breakable = self.breakable.saturating_sub(1);
        }
    }

    fn visit_function(&mut self, _: &Function<'a>, _: ScopeFlags) {}

    fn visit_arrow_function_expression(&mut self, _: &ArrowFunctionExpression<'a>) {}
}

/// The root identifiers of every assignment and update target in a subtree.
struct OuterWrites<'a> {
    names: Vec<&'a IdentifierReference<'a>>,
}

impl<'a> Visit<'a> for OuterWrites<'a> {
    fn enter_node(&mut self, kind: AstKind<'a>) {
        let target = match kind {
            AstKind::AssignmentExpression(assignment) => assignment.left.as_simple_assignment_target().map(simple_target),
            AstKind::UpdateExpression(update) => Some(simple_target(&update.argument)),
            _ => None,
        };

        if let Some(root) = target.and_then(root_identifier) {
            self.names.push(root);
        }
    }
}

/// The identifier a target's member chain starts from: `a` in `a.b[c] = 1`.
fn root_identifier(mut kind: AstKind<'_>) -> Option<&IdentifierReference<'_>> {
    loop {
        kind = unwrap(kind);

        if let AstKind::IdentifierReference(identifier) = kind {
            return Some(identifier);
        }

        kind = expression(Member::of(kind)?.object);
    }
}

enum How<'a> {
    Method(&'a str),
    Indexed,
    Spread,
    Concat,
}

struct Site<'a> {
    node: NodeId,
    target: &'a IdentifierReference<'a>,
    how: How<'a>,
}

/// Every collection-building write in a subtree, in postorder.
struct Accumulations<'a> {
    sites: Vec<Site<'a>>,
}

impl<'a> Visit<'a> for Accumulations<'a> {
    fn leave_node(&mut self, kind: AstKind<'a>) {
        if let Some(site) = accumulation(kind) {
            self.sites.push(site);
        }
    }
}

fn accumulation<'a>(kind: AstKind<'a>) -> Option<Site<'a>> {
    match kind {
        AstKind::CallExpression(call) => {
            let member = Member::of(unwrap(expression(&call.callee)))?;
            let method = member.identifier_name().filter(|name| COLLECTING_METHODS.contains(name))?;
            let AstKind::IdentifierReference(target) = unwrap(expression(member.object)) else {
                return None;
            };

            Some(Site { node: call.node_id.get(), target, how: How::Method(method) })
        }
        AstKind::AssignmentExpression(assignment) => {
            let node = assignment.node_id.get();

            match &assignment.left {
                AssignmentTarget::ComputedMemberExpression(member) => match unwrap(expression(&member.object)) {
                    AstKind::IdentifierReference(target) => Some(Site { node, target, how: How::Indexed }),
                    _ => None,
                },
                AssignmentTarget::AssignmentTargetIdentifier(target) if assignment.operator == AssignmentOperator::Assign => {
                    let is_self =
                        |argument: &'a Expression<'a>| matches!(unwrap(expression(argument)), AstKind::IdentifierReference(value) if value.name == target.name);

                    let how = match unwrap(expression(&assignment.right)) {
                        AstKind::ArrayExpression(array)
                            if array
                                .elements
                                .iter()
                                .any(|element| matches!(element, ArrayExpressionElement::SpreadElement(spread) if is_self(&spread.argument))) =>
                        {
                            How::Spread
                        }
                        AstKind::ObjectExpression(object)
                            if object
                                .properties
                                .iter()
                                .any(|property| matches!(property, ObjectPropertyKind::SpreadProperty(spread) if is_self(&spread.argument))) =>
                        {
                            How::Spread
                        }
                        AstKind::CallExpression(call) if matches!(strip_parens(&call.callee), Expression::StaticMemberExpression(callee) if callee.property.name == "concat" && is_self(&callee.object)) => {
                            How::Concat
                        }
                        _ => return None,
                    };

                    Some(Site { node, target, how })
                }
                _ => None,
            }
        }
        _ => None,
    }
}

/// Whether a `for (;;)` loop encloses `node` within `loop_id`, the loop
/// itself included.
fn inside_counted_loop(nodes: &AstNodes<'_>, node: NodeId, loop_id: NodeId) -> bool {
    for ancestor in ancestors(nodes, node) {
        if matches!(ancestor.kind(), AstKind::ForStatement(_)) {
            return true;
        }

        if ancestor.id() == loop_id {
            return false;
        }
    }

    false
}

/// Whether any identifier in `expression` (property names too) is `name`.
fn references_name(root: &Expression<'_>, name: &str) -> bool {
    let mut finder = NameFinder { name, found: false };

    finder.visit_expression(root);
    finder.found
}

struct NameFinder<'n> {
    name: &'n str,
    found: bool,
}

impl<'a> Visit<'a> for NameFinder<'_> {
    fn enter_node(&mut self, kind: AstKind<'a>) {
        let name = match kind {
            AstKind::IdentifierReference(identifier) => identifier.name.as_str(),
            AstKind::IdentifierName(identifier) => identifier.name.as_str(),
            AstKind::BindingIdentifier(identifier) => identifier.name.as_str(),
            AstKind::LabelIdentifier(identifier) => identifier.name.as_str(),
            _ => return,
        };

        self.found |= name == self.name;
    }
}
