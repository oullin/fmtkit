//! `anti-slop-effect/no-ordie-in-service-construction`: outside test and spec
//! files, no `Effect.orDie` or `Effect.orDieWith` while a service is
//! constructed, and no `Layer.orDie` anywhere; keep construction failures
//! typed in the layer's error channel.
//!
//! Construction is the `make` of a `Context.Service` class (its options'
//! `make` or a static `make`), its static `layer*` members, and the arguments
//! of `Layer.effect`, `Layer.scoped` and `Layer.unwrap*`. It runs into
//! `Effect.gen` generators only through a `yield*`, and into
//! `Effect.suspend`/`Effect.sync` thunks; it stops at other functions,
//! `Effect.fn`/`Effect.fnUntraced` definitions and object-literal properties,
//! which run later as operations or callbacks. `Effect` and `Layer` are
//! recognized by their imports from `effect` or `effect/<Module>`. Takes no
//! options.

use oxc_ast::AstKind;
use oxc_ast::ast::{Expression, MethodDefinitionType, PropertyDefinitionType};
use oxc_semantic::{AstNode, AstNodes};
use oxc_span::GetSpan;

use super::super::anti_slop::shared::strip_parens;
use super::super::anti_slop::shared::testing::is_test_file;
use super::super::{Context, Rule};
use super::shared::effect_modules::{
    Module, Modules, call_name, callee_name, estree_parent, has_argument, is_function, is_layer_member_name, key_name, service_class,
};

pub const NAME: &str = "anti-slop-effect/no-ordie-in-service-construction";

const LAYER_OR_DIE: &str = "`Layer.orDie` hides this layer's construction failures as defects. Leave them in the layer's error channel as Schema.TaggedErrors and handle them where the runtime is built.";

pub struct NoOrDieInServiceConstruction;

impl Rule for NoOrDieInServiceConstruction {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let AstKind::StaticMemberExpression(member) = node.kind() else {
            return;
        };
        let property = member.property.name.as_str();

        if !matches!(property, "orDie" | "orDieWith") {
            return;
        }

        let Expression::Identifier(object) = strip_parens(&member.object) else {
            return;
        };

        if is_test_file(ctx.rel) {
            return;
        }

        let modules = Modules::of(ctx);

        if property == "orDie" && modules.has(Module::Layer, object.name.as_str()) {
            ctx.report(member.span, LAYER_OR_DIE);
            return;
        }

        if modules.has(Module::Effect, object.name.as_str()) && is_in_service_construction(ctx.semantic.nodes(), node, &modules) {
            ctx.report(
                member.span,
                format!(
                    "`Effect.{property}` turns a service construction failure into a defect. Classify the failure as a Schema.TaggedError and let it flow through the layer's error channel (`Layer.Layer<Service, ThatError>`), or handle it here with Effect.catchTag."
                ),
            );
        }
    }
}

/// Whether `node` runs while a service is constructed.
fn is_in_service_construction<'a>(nodes: &AstNodes<'a>, node: &AstNode<'a>, modules: &Modules) -> bool {
    let mut child = node;
    let mut yielded = false;

    while let Some(current) = estree_parent(nodes, child.id()) {
        let kind = current.kind();

        if is_function(kind) {
            if estree_parent(nodes, current.id()).is_some_and(|parent| is_construction_root(nodes, parent, current, modules)) {
                return true;
            }

            match effect_constructor_of(nodes, current, modules) {
                Some("gen") => {
                    if !yielded {
                        return false;
                    }

                    yielded = false;
                }
                Some("suspend" | "sync") => {}
                _ => return false,
            }
        } else if is_construction_root(nodes, current, child, modules) {
            return true;
        } else if is_property(kind) || is_operation_definition(kind, modules) {
            return false;
        } else if let AstKind::YieldExpression(expression) = kind
            && expression.delegate
        {
            yielded = true;
        }

        child = current;
    }

    false
}

/// ESTree's `Property`, in object literals and patterns alike.
fn is_property(kind: AstKind<'_>) -> bool {
    matches!(
        kind,
        AstKind::ObjectProperty(_)
            | AstKind::BindingProperty(_)
            | AstKind::AssignmentTargetPropertyProperty(_)
            | AstKind::AssignmentTargetPropertyIdentifier(_)
    )
}

/// Whether the class `node` (a class body's member) belongs to is a service class.
fn is_in_service_class(nodes: &AstNodes<'_>, member: &AstNode<'_>, modules: &Modules) -> bool {
    let class = estree_parent(nodes, member.id()).and_then(|body| estree_parent(nodes, body.id()));

    matches!(class.map(AstNode::kind), Some(AstKind::Class(class)) if service_class(class, modules).is_some())
}

/// Whether `node`, entered from `child`, is where a service is constructed.
fn is_construction_root<'a>(nodes: &AstNodes<'a>, node: &AstNode<'a>, child: &AstNode<'a>, modules: &Modules) -> bool {
    let child_span = child.kind().span();
    let is_construction_name = |name: Option<&str>| name.is_some_and(|name| name == "make" || is_layer_member_name(name));

    match node.kind() {
        AstKind::PropertyDefinition(property) if property.r#type == PropertyDefinitionType::PropertyDefinition && property.r#static && !property.computed => {
            property.value.as_ref().is_some_and(|value| strip_parens(value).span() == child_span)
                && is_construction_name(key_name(&property.key))
                && is_in_service_class(nodes, node, modules)
        }
        AstKind::MethodDefinition(method) if method.r#type == MethodDefinitionType::MethodDefinition && method.r#static && !method.computed => {
            method.value.span == child_span && is_construction_name(key_name(&method.key)) && is_in_service_class(nodes, node, modules)
        }
        AstKind::ObjectProperty(property)
            if !property.computed && strip_parens(&property.value).span() == child_span && key_name(&property.key) == Some("make") =>
        {
            let Some(options) = estree_parent(nodes, node.id()).filter(|options| matches!(options.kind(), AstKind::ObjectExpression(_))) else {
                return false;
            };
            let Some(super_call) = estree_parent(nodes, options.id()).filter(|call| matches!(call.kind(), AstKind::CallExpression(_))) else {
                return false;
            };

            matches!(estree_parent(nodes, super_call.id()).map(AstNode::kind), Some(AstKind::Class(class)) if service_class(class, modules).is_some())
        }
        AstKind::CallExpression(call) if has_argument(call, child_span) => {
            callee_name(call, modules, Module::Layer).is_some_and(|name| name == "effect" || name == "scoped" || name.starts_with("unwrap"))
        }
        _ => false,
    }
}

/// The Effect constructor a function is passed to: `gen` for `Effect.gen(function* () {})`.
fn effect_constructor_of<'a>(nodes: &AstNodes<'a>, function: &AstNode<'a>, modules: &Modules) -> Option<&'a str> {
    match estree_parent(nodes, function.id()).map(AstNode::kind) {
        Some(AstKind::CallExpression(call)) if has_argument(call, function.kind().span()) => callee_name(call, modules, Module::Effect),
        _ => None,
    }
}

/// `Effect.fn(...)(...)`, `Effect.fn(...)` or `Effect.fnUntraced(...)`: an operation definition.
fn is_operation_definition(kind: AstKind<'_>, modules: &Modules) -> bool {
    let AstKind::CallExpression(call) = kind else {
        return false;
    };
    let is_operation = |name: Option<&str>| matches!(name, Some("fn" | "fnUntraced"));

    is_operation(callee_name(call, modules, Module::Effect)) || is_operation(call_name(&call.callee, modules, Module::Effect))
}
