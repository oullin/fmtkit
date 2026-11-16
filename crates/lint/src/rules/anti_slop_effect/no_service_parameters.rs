//! `anti-slop-effect/no-service-parameters`: outside test and spec files, no
//! function parameter typed as `Context.Context`, `ManagedRuntime.ManagedRuntime`,
//! `Layer.Layer`, or a `Context.Service` class declared in the same file;
//! yield the services an operation needs and let its requirements flow
//! through `R`.
//!
//! Functions, arrows, function types and method signatures are checked;
//! call, construct and overload signatures are not. A `Layer.Layer`
//! parameter is allowed in a layer combinator: a function that declares a
//! `Layer.Layer` return type or is an arrow whose body is a `Layer.*` call or
//! a pipe of them. A callback passed to a `Context.*`, `Effect.*`, `Layer.*`
//! or `ManagedRuntime.*` call is allowed too: the API dictates its parameter
//! types. Modules are recognized by their imports from `effect` or
//! `effect/<Module>`; a service imported from another file is not known to
//! be one. Takes no options.

use oxc_ast::AstKind;
use oxc_ast::ast::{Expression, FunctionType, TSType, TSTypeName, TSTypeReference};
use oxc_semantic::{AstNode, Semantic};
use oxc_span::GetSpan;

use super::super::anti_slop::shared::params::Signature;
use super::super::anti_slop::shared::strip_type_parens;
use super::super::anti_slop::shared::testing::is_test_file;
use super::super::{Context, Rule};
use super::shared::effect_modules::{Module, Modules, call_name, callee_name, estree_parent, has_argument, pipe_receiver, service_class, unwrap};

pub const NAME: &str = "anti-slop-effect/no-service-parameters";

pub struct NoServiceParameters;

/// The names of the `Context.Service` classes the file declares.
struct ServiceNames(Vec<Box<str>>);

impl ServiceNames {
    fn collect(semantic: &Semantic<'_>, modules: &Modules) -> Self {
        let names = semantic
            .nodes()
            .iter()
            .filter_map(|node| match node.kind() {
                AstKind::Class(class) if service_class(class, modules).is_some() => class.id.as_ref().map(|id| id.name.as_str().into()),
                _ => None,
            })
            .collect();

        Self(names)
    }
}

impl Rule for NoServiceParameters {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let kind = node.kind();
        let body = match kind {
            AstKind::ArrowFunctionExpression(arrow) => Some(&arrow.body),
            AstKind::Function(function) if matches!(function.r#type, FunctionType::FunctionDeclaration | FunctionType::FunctionExpression) => None,
            AstKind::TSFunctionType(_) | AstKind::TSMethodSignature(_) => None,
            _ => return,
        };
        let Some(signature) = Signature::of(kind) else {
            return;
        };
        let typed: Vec<_> = signature
            .params()
            .filter_map(|param| match param.annotation().map(|annotation| strip_type_parens(&annotation.type_annotation)) {
                Some(TSType::TSTypeReference(reference)) => Some((param, &**reference)),
                _ => None,
            })
            .collect();

        if typed.is_empty() || is_test_file(ctx.rel) {
            return;
        }

        let modules = Modules::of(ctx);

        if is_effect_api_callback(ctx, node, &modules) {
            return;
        }

        for (param, reference) in typed {
            if let Some(module) = carrier_module(reference, &modules) {
                let returns_layer = signature
                    .return_type
                    .is_some_and(|annotation| matches!(strip_type_parens(&annotation.type_annotation), TSType::TSTypeReference(returned) if carrier_module(returned, &modules) == Some(Module::Layer)))
                    || matches!(body, Some(body) if body.as_expression().is_some_and(|body| is_layer_expression(body, &modules)));

                if module == Module::Layer && returns_layer {
                    continue;
                }

                let span = reference.type_name.span();
                let text = &ctx.source()[span.start as usize..span.end as usize];

                ctx.report(
                    param.estree_span(),
                    format!(
                        "Do not pass a `{text}` as a parameter. `yield*` the services the operation needs inside it (`const store = yield* Store`) and let its requirements flow through R to the runtime that provides them; pass only plain values."
                    ),
                );

                continue;
            }

            let TSTypeName::IdentifierReference(name) = &reference.type_name else {
                continue;
            };
            let services = {
                let modules = modules.clone();

                ctx.shared(move |semantic| ServiceNames::collect(semantic, &modules))
            };

            if services.0.iter().any(|service| **service == *name.name) {
                ctx.report(
                    param.estree_span(),
                    format!(
                        "Do not pass the `{0}` service as a parameter. `yield* {0}` inside the operation and let the requirement flow through R to the layer that provides it; pass only plain values.",
                        name.name
                    ),
                );
            }
        }
    }
}

/// The carrier module of a `Module.Type<...>` reference: `Context.Context`,
/// `ManagedRuntime.ManagedRuntime` or `Layer.Layer` through an Effect import.
fn carrier_module(reference: &TSTypeReference<'_>, modules: &Modules) -> Option<Module> {
    let TSTypeName::QualifiedName(name) = &reference.type_name else {
        return None;
    };
    let TSTypeName::IdentifierReference(left) = &name.left else {
        return None;
    };

    [(Module::Context, "Context"), (Module::ManagedRuntime, "ManagedRuntime"), (Module::Layer, "Layer")]
        .into_iter()
        .find_map(|(module, type_name)| (name.right.name == type_name && modules.has(module, left.name.as_str())).then_some(module))
}

/// Whether an expression is a Layer constructor call or a pipe of Layer combinators.
fn is_layer_expression(expression: &Expression<'_>, modules: &Modules) -> bool {
    let expression = unwrap(expression);

    if call_name(expression, modules, Module::Layer).is_some() {
        return true;
    }

    let (Some(receiver), Expression::CallExpression(call)) = (pipe_receiver(expression), expression) else {
        return false;
    };

    is_layer_expression(receiver, modules)
        || call.arguments.iter().any(|step| step.as_expression().is_some_and(|step| call_name(unwrap(step), modules, Module::Layer).is_some()))
}

/// Whether the function is a callback passed to an Effect API call, whose
/// parameter types the API dictates.
fn is_effect_api_callback(ctx: &Context<'_, '_>, node: &AstNode<'_>, modules: &Modules) -> bool {
    let span = node.kind().span();

    match estree_parent(ctx.semantic.nodes(), node.id()).map(AstNode::kind) {
        Some(AstKind::CallExpression(call)) => has_argument(call, span) && Module::ALL.into_iter().any(|module| callee_name(call, modules, module).is_some()),
        _ => false,
    }
}
