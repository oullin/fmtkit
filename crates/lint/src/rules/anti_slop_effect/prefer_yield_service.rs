//! `anti-slop-effect/prefer-yield-service`: no `const services = yield*
//! Effect.context()` whose result is passed back to `Effect.provide`,
//! `Effect.provideContext`, `Layer.provide` or `Layer.succeedContext`; yield
//! each service the operation uses instead.
//!
//! The declarator is reported once, naming the first providing call among the
//! variable's references, through parentheses and type assertions. A captured
//! context used any other way, such as `Effect.runForkWith(context)` or
//! `Context.get(services, Store)`, is left alone. `Effect` and `Layer` are
//! recognized by their imports from `effect` or `effect/<Module>`. Takes no
//! options.

use oxc_ast::AstKind;
use oxc_ast::ast::{BindingPattern, Expression};
use oxc_semantic::{AstNode, AstNodes, NodeId};
use oxc_span::GetSpan;

use super::super::{Context, Rule};
use super::shared::effect_modules::{Module, Modules, call_name, callee_name, is_wrapper, unwrap};

pub const NAME: &str = "anti-slop-effect/prefer-yield-service";

pub struct PreferYieldService;

impl Rule for PreferYieldService {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let AstKind::VariableDeclarator(declarator) = node.kind() else {
            return;
        };
        let (BindingPattern::BindingIdentifier(identifier), Some(init)) = (&declarator.id, &declarator.init) else {
            return;
        };
        let Expression::YieldExpression(yielded) = unwrap(init) else {
            return;
        };
        let Some(argument) = yielded.argument.as_ref().filter(|_| yielded.delegate) else {
            return;
        };
        let modules = Modules::of(ctx);

        if call_name(unwrap(argument), &modules, Module::Effect) != Some("context") {
            return;
        }

        let Some(symbol) = identifier.symbol_id.get() else {
            return;
        };
        let semantic = ctx.semantic;
        let scoping = semantic.scoping();
        let nodes = semantic.nodes();

        for &reference in scoping.get_resolved_reference_ids(symbol) {
            let Some(provide) = providing_callee(nodes, scoping.get_reference(reference).node_id(), &modules) else {
                continue;
            };
            let text = &ctx.source()[provide.start as usize..provide.end as usize];

            ctx.report(
                declarator.span,
                format!(
                    "Do not capture the context with `Effect.context` and provide it back with {text}. `yield*` each service the operation uses (`const store = yield* Store`) and let the nested effect's requirements flow through its R channel to the layer or runtime that provides them."
                ),
            );

            return;
        }
    }
}

/// The span of the unwrapped callee of the `Effect.provide`,
/// `Effect.provideContext`, `Layer.provide` or `Layer.succeedContext` call the
/// reference at `id` is an argument of.
fn providing_callee(nodes: &AstNodes<'_>, id: NodeId, modules: &Modules) -> Option<oxc_span::Span> {
    let mut child = nodes.get_node(id);
    let mut parent = nodes.parent_node(child.id());

    while is_wrapper(parent.kind()) {
        child = parent;
        parent = nodes.parent_node(parent.id());
    }

    let AstKind::CallExpression(call) = parent.kind() else {
        return None;
    };

    let span = child.kind().span();

    if !call.arguments.iter().any(|argument| argument.span() == span) {
        return None;
    }

    let provides = matches!(callee_name(call, modules, Module::Effect), Some("provide" | "provideContext"))
        || matches!(callee_name(call, modules, Module::Layer), Some("provide" | "succeedContext"));

    provides.then(|| unwrap(&call.callee).span())
}
