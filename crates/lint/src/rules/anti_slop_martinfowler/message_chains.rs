//! `anti-slop-martinfowler/message-chains`: chains of property reads and
//! zero-argument `getX()` calls that navigate through more than `maxDepth`
//! objects (default 3).
//!
//! The depth is the longest run of navigation steps in the chain: a property
//! read, or a getter call (`getX()` with no arguments), adds one; a call with
//! arguments, any other method call or a computed key that is not a literal
//! ends the run, so builders (`db.select().from(t)`) and `a[key]` lookups
//! never add up. A literal index (`rows[0]`) neither adds nor ends a run. The
//! whole chain is reported once, at its outermost expression.
//!
//! Skipped: chains through `.pipe(…)`, chains rooted at `import.meta`,
//! `new.target` or `super`, at an `ignoreRoots` name, or, with
//! `ignorePascalCaseRoots`, at a capitalised name (namespaces and classes such
//! as `Schema.Struct`).
//!
//! Options: `{ ignoreTests?: boolean = true, maxDepth?: integer >= 1 = 3,
//! ignoreRoots?: string[] = [console, document, globalThis, location,
//! navigator, process, self, window], ignorePascalCaseRoots?: boolean = true }`.
//! Test files (upstream's `isTestFile`) are skipped unless `ignoreTests` is
//! `false`.

use oxc_ast::AstKind;
use oxc_semantic::{AstNode, AstNodes};
use oxc_span::GetSpan;
use serde_json::Value;

use super::super::anti_slop::shared::{parent, strip_parens};
use super::super::{Context, Rule};
use super::shared::{Member, Options, Property, expression, is_literal, is_node, is_test_file, is_wrapper, text, truncate, unwrap};

pub const NAME: &str = "anti-slop-martinfowler/message-chains";

const DEFAULT_IGNORE_ROOTS: [&str; 8] = ["console", "document", "globalThis", "location", "navigator", "process", "self", "window"];

pub fn build(options: &[Value]) -> Result<Box<dyn Rule>, String> {
    let options = Options::parse(options, &["ignoreTests", "maxDepth", "ignoreRoots", "ignorePascalCaseRoots"])?;

    Ok(Box::new(MessageChains {
        ignore_tests: options.boolean("ignoreTests", true)?,
        max_depth: options.integer("maxDepth", 1, 3)?,
        ignore_roots: options.strings("ignoreRoots", &DEFAULT_IGNORE_ROOTS)?,
        ignore_pascal_case_roots: options.boolean("ignorePascalCaseRoots", true)?,
    }))
}

struct MessageChains {
    ignore_tests: bool,
    max_depth: i64,
    ignore_roots: Vec<String>,
    ignore_pascal_case_roots: bool,
}

impl Rule for MessageChains {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        if Member::of(node.kind()).is_none() {
            return;
        }

        let Some(top) = chain_top(ctx.semantic.nodes(), node) else {
            return;
        };

        let chain = measure_chain(top.kind());

        if i64::try_from(chain.depth).is_ok_and(|depth| depth <= self.max_depth) || chain.has_pipe || (self.ignore_tests && is_test_file(ctx.rel)) {
            return;
        }

        match chain.root {
            AstKind::ImportMeta(_) | AstKind::NewTarget(_) | AstKind::Super(_) => return,
            AstKind::IdentifierReference(root) => {
                let name = root.name.as_str();

                if self.ignore_roots.iter().any(|ignored| ignored == name)
                    || (self.ignore_pascal_case_roots && name.starts_with(|c: char| c.is_ascii_uppercase()))
                {
                    return;
                }
            }
            _ => {}
        }

        let span = top.kind().span();
        let snippet = truncate(text(ctx.source(), span), 60);
        let (depth, max_depth) = (chain.depth, self.max_depth);

        ctx.report(
            span,
            format!(
                "Possibly Message Chains: `{snippet}` navigates through {depth} objects (limit {max_depth}), so this code depends on the structure of every object along the path. Consider Hide Delegate: ask the first object for what you need through one method (in Effect, a service operation that returns the value), or Extract Function on the chain and Move Function it next to the data it walks."
            ),
        );
    }
}

/// The outermost expression of the chain `node` belongs to, or none when
/// `node` is the object of another member (an inner link).
fn chain_top<'s, 'a>(nodes: &'s AstNodes<'a>, node: &'s AstNode<'a>) -> Option<&'s AstNode<'a>> {
    let mut top = node;

    loop {
        let parent_node = parent(nodes, top.id());

        if parent_node.id() == top.id() {
            return Some(top);
        }

        let kind = parent_node.kind();

        if is_wrapper(kind) {
            top = parent_node;
            continue;
        }

        if Member::of(kind).is_some_and(|member| is_node(member.object, top.id())) {
            return None;
        }

        if let AstKind::CallExpression(call) = kind
            && is_node(&call.callee, top.id())
        {
            top = parent_node;
            continue;
        }

        return Some(top);
    }
}

struct Chain<'a> {
    depth: usize,
    root: AstKind<'a>,
    has_pipe: bool,
}

/// The longest run of property reads and zero-argument `getX()` calls in a
/// chain, as upstream's `measureChain`.
fn measure_chain(top: AstKind<'_>) -> Chain<'_> {
    let mut current = unwrap(top);
    let mut run = 0;
    let mut depth = 0;
    let mut has_pipe = false;

    loop {
        if let Some(member) = Member::of(current) {
            match member.property {
                Property::Computed(key) if !is_literal(expression(strip_parens(key))) => {
                    depth = depth.max(run);
                    run = 0;
                }
                Property::Computed(_) => {}
                Property::Name(_) | Property::Private(_) => run += 1,
            }

            current = unwrap(expression(member.object));
            continue;
        }

        let AstKind::CallExpression(call) = current else {
            break;
        };

        let callee = unwrap(expression(&call.callee));
        let Some(member) = Member::of(callee) else {
            break;
        };

        let name = match member.property {
            Property::Name(name) | Property::Private(name) => Some(name),
            Property::Computed(_) => None,
        };

        has_pipe |= name == Some("pipe");

        let is_getter = call.arguments.is_empty()
            && name.is_some_and(|name| name.strip_prefix("get").is_some_and(|rest| rest.starts_with(|c: char| c.is_ascii_uppercase())));

        if is_getter {
            current = callee;
            continue;
        }

        depth = depth.max(run);
        run = 0;
        current = unwrap(expression(member.object));
    }

    Chain { depth: depth.max(run), root: current, has_pipe }
}
