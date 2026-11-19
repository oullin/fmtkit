//! `anti-slop-effect/require-service-make-layer`: outside test and spec
//! files, a `Context.Service` class owns its constructor as `make` and its
//! layer as `static layer = Layer.effect(this, this.make)`, and its variant
//! layers derive from them; a `Layer.effect` outside a service class builds a
//! service only from that service's `make`.
//!
//! A service without a make is reported when it has static `layer*` members,
//! or always with `allowTagOnly: false`. A service with a make needs a static
//! `layer` built only from `Layer.effect(this|Name, this.make|Name.make[(...)])`,
//! through pipes, functions returning it, or a sibling `layer*` member. A
//! variant `layer*` member that builds this service with `Layer.effect`,
//! `Layer.scoped`, `Layer.succeed` or `Layer.sync` must refer to `make` or a
//! layer member. A `Layer.effect(Target, make)` elsewhere is allowed when
//! `make` is `Target.make` by source text, or `Target` is imported from an
//! Effect package (`effect`, `effect/*`, `@effect/*`). Modules are recognized
//! by their imports from `effect` or `effect/<Module>`.
//!
//! Options: `[{ allowTagOnly?: boolean }]`, default `true`.

use oxc_ast::AstKind;
use oxc_ast::ast::{Argument, Expression};
use oxc_semantic::{AstNode, AstNodes, Semantic};
use oxc_span::GetSpan;
use serde_json::Value as Json;

use super::super::anti_slop::shared::strip_parens;
use super::super::anti_slop::shared::testing::is_test_file;
use super::super::{Context, Rule};
use super::shared::effect_modules::{
    FunctionNode, Module, Modules, Service, Value, call_pipe_receiver, callee_name, class_member_name, contains, enclosing_class, is_layer_member_name,
    resolve, returned_expressions, service_class, service_makes, static_member_of_class, static_members, unwrap,
};

pub const NAME: &str = "anti-slop-effect/require-service-make-layer";

/// The Layer constructors that build a service from its tag.
const LAYER_CONSTRUCTORS: [&str; 4] = ["effect", "scoped", "succeed", "sync"];

/// `[{ allowTagOnly?: boolean }]`, validated like upstream's schema.
pub fn build(options: &[Json]) -> Result<Box<dyn Rule>, String> {
    let mut rule = RequireServiceMakeLayer { allow_tag_only: true };

    match options {
        [] => {}
        [Json::Object(object)] => {
            for (key, value) in object {
                match key.as_str() {
                    "allowTagOnly" => rule.allow_tag_only = value.as_bool().ok_or("\"allowTagOnly\" must be a boolean")?,
                    other => return Err(format!("unknown option \"{other}\"")),
                }
            }
        }
        [_] => return Err("expected an options object".into()),
        _ => return Err("expected at most one options object".into()),
    }

    Ok(Box::new(rule))
}

struct RequireServiceMakeLayer {
    allow_tag_only: bool,
}

impl Rule for RequireServiceMakeLayer {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        match node.kind() {
            AstKind::Class(class) if class.heritage.is_some() => {
                if is_test_file(ctx.rel) {
                    return;
                }

                let modules = Modules::of(ctx);

                if let Some(service) = service_class(class, &modules) {
                    self.check_class(ctx, &service, &modules);
                }
            }
            AstKind::CallExpression(call) if matches!(unwrap(&call.callee), Expression::StaticMemberExpression(member) if member.property.name == "effect") => {
                if is_test_file(ctx.rel) {
                    return;
                }

                let modules = Modules::of(ctx);

                if callee_name(call, &modules, Module::Layer) != Some("effect") {
                    return;
                }

                let semantic = ctx.semantic;

                if enclosing_class(semantic.nodes(), node.id()).is_some_and(|class| service_class(class, &modules).is_some()) {
                    return;
                }

                let (Some(target), Some(make)) =
                    (call.arguments.first().and_then(Argument::as_expression), call.arguments.get(1).and_then(Argument::as_expression))
                else {
                    return;
                };
                let source = ctx.source();
                let text = |expression: &Expression<'_>| {
                    let span = expression.span();

                    &source[span.start as usize..span.end as usize]
                };

                if is_target_make(text, target, make) || import_source(semantic, target).is_some_and(is_effect_package) {
                    return;
                }

                let service = text(unwrap(target));

                ctx.report(
                    call.span,
                    format!(
                        "This Layer.effect builds `{service}` from an inline constructor. Move the constructor into the service class as `make: Effect.gen(...)`, add `static readonly layer = Layer.effect(this, this.make)`, and use `{service}.layer` here."
                    ),
                );
            }
            _ => {}
        }
    }
}

impl RequireServiceMakeLayer {
    fn check_class(&self, ctx: &mut Context<'_, '_>, service: &Service<'_>, modules: &Modules) {
        let nodes = ctx.semantic.nodes();
        let class = service.class;
        let name = service.name.unwrap_or("this service");
        let name_span = class.id.as_ref().map_or(class.span, |id| id.span);
        let members = static_members(class);
        let layers: Vec<_> = members.iter().filter(|member| is_layer_member_name(member.name)).collect();

        if service_makes(service).is_empty() {
            if self.allow_tag_only && layers.is_empty() {
                return;
            }

            ctx.report(
                name_span,
                format!(
                    "Service \"{name}\" has no `make`. Add `make: Effect.gen(function* () {{ ...yield* dependencies...; return {{ ...operations }} }})` to its Context.Service options (or `static readonly make = (options: Options) => Effect.gen(...)` when construction needs plain options), then add `static readonly layer = Layer.effect(this, this.make)`."
                ),
            );
            return;
        }

        let layer = layers.iter().position(|member| member.name == "layer");

        match layer.and_then(|index| layers.get(index)) {
            None => ctx.report(
                name_span,
                format!(
                    "Service \"{name}\" has a `make` but no `static layer`. Add `static readonly layer = Layer.effect(this, this.make)`, or `static readonly layer = (options: Options) => Layer.effect(this, this.make(options))` when make takes options."
                ),
            ),
            Some(member) if !member.value.is_some_and(|value| is_make_layer(nodes, value, service, modules)) => ctx.report(
                member.span,
                format!(
                    "`{name}.layer` must be built from `{name}.make`. Write `static readonly layer = Layer.effect(this, this.make)` (add `.pipe(Layer.provide(...))` for local wiring), or `(options: Options) => Layer.effect(this, this.make(options))`; move any construction code into make."
                ),
            ),
            Some(_) => {}
        }

        for (index, member) in layers.iter().enumerate() {
            if Some(index) == layer {
                continue;
            }

            let Some(value) = member.value else {
                continue;
            };

            if !builds_service(nodes, value, service, modules) || derives_from_service(nodes, value, service) {
                continue;
            }

            ctx.report(
                member.span,
                format!(
                    "`{name}.{0}` does not derive from `{name}.layer` or `{name}.make`. Write it as `{name}.layer.pipe(Layer.provide(...))` or `Layer.effect(this, this.make(options))`. A separate implementation belongs in its own service, or in test/ when it is a fake.",
                    member.name
                ),
            );
        }
    }
}

/// `this` or the class name.
fn is_target(expression: &Expression<'_>, service: &Service<'_>) -> bool {
    match unwrap(expression) {
        Expression::ThisExpression(_) => true,
        Expression::Identifier(identifier) => service.name == Some(identifier.name.as_str()),
        _ => false,
    }
}

/// `this.make`, `ClassName.make`, or a call of either.
fn is_make_reference(make: Option<&Argument<'_>>, service: &Service<'_>) -> bool {
    let Some(make) = make.and_then(Argument::as_expression) else {
        return false;
    };
    let target = match unwrap(make) {
        Expression::CallExpression(call) => &call.callee,
        expression => expression,
    };

    class_member_name(target, service) == Some("make")
}

/// Whether `value` is `Layer.effect(this, this.make)` or a form built only from it.
fn is_make_layer<'a>(nodes: &AstNodes<'a>, value: Value<'a>, service: &Service<'_>, modules: &Modules) -> bool {
    let expression = match value {
        Value::Function(function) => return is_make_layer_function(nodes, FunctionNode::Function(function), service, modules),
        Value::Expression(expression) => unwrap(expression),
    };

    if let Some(function) = FunctionNode::of(expression) {
        return is_make_layer_function(nodes, function, service, modules);
    }

    if let Expression::CallExpression(call) = expression {
        if let Some(receiver) = call_pipe_receiver(call) {
            return is_make_layer(nodes, Value::Expression(receiver), service, modules);
        }

        if class_member_name(&call.callee, service).is_some_and(is_layer_member_name) {
            return true;
        }

        if callee_name(call, modules, Module::Layer) != Some("effect") {
            return false;
        }

        return call.arguments.first().and_then(Argument::as_expression).is_some_and(|target| is_target(target, service))
            && is_make_reference(call.arguments.get(1), service);
    }

    class_member_name(expression, service).is_some_and(|sibling| sibling != "layer" && is_layer_member_name(sibling))
}

fn is_make_layer_function<'a>(nodes: &AstNodes<'a>, function: FunctionNode<'a>, service: &Service<'_>, modules: &Modules) -> bool {
    let returned = returned_expressions(nodes, function);

    !returned.is_empty() && returned.into_iter().all(|value| is_make_layer(nodes, Value::Expression(value), service, modules))
}

/// Whether a static member builds this service with a Layer constructor,
/// rather than a layer for another tag.
fn builds_service(nodes: &AstNodes<'_>, value: Value<'_>, service: &Service<'_>, modules: &Modules) -> bool {
    contains(nodes, value.node_id(), |node| match node.kind() {
        AstKind::CallExpression(call) => {
            callee_name(call, modules, Module::Layer).is_some_and(|constructor| LAYER_CONSTRUCTORS.contains(&constructor))
                && call.arguments.first().and_then(Argument::as_expression).is_some_and(|target| is_target(target, service))
        }
        _ => false,
    })
}

/// Whether a variant layer refers to the service's make or one of its layers.
fn derives_from_service(nodes: &AstNodes<'_>, value: Value<'_>, service: &Service<'_>) -> bool {
    contains(nodes, value.node_id(), |node| match node.kind() {
        AstKind::StaticMemberExpression(member) => {
            static_member_of_class(member, service).is_some_and(|member| member == "make" || is_layer_member_name(member))
        }
        _ => false,
    })
}

/// `X.make` or `X.make(...)` where `X` has the source text of the layer target.
fn is_target_make<'s>(text: impl Fn(&Expression<'_>) -> &'s str, target: &Expression<'_>, make: &Expression<'_>) -> bool {
    let expression = unwrap(make);
    let reference = unwrap(match expression {
        Expression::CallExpression(call) => &call.callee,
        expression => expression,
    });

    matches!(reference, Expression::StaticMemberExpression(member) if member.property.name == "make" && text(strip_parens(&member.object)) == text(unwrap(target)))
}

/// The module an expression's root identifier is imported from.
fn import_source<'a>(semantic: &Semantic<'a>, expression: &Expression<'_>) -> Option<&'a str> {
    let mut root = unwrap(expression);

    loop {
        root = match root {
            Expression::StaticMemberExpression(member) => unwrap(&member.object),
            Expression::ComputedMemberExpression(member) => unwrap(&member.object),
            Expression::PrivateFieldExpression(member) => unwrap(&member.object),
            _ => break,
        };
    }

    let Expression::Identifier(identifier) = root else {
        return None;
    };
    let symbol = resolve(semantic, identifier)?;
    let nodes = semantic.nodes();
    let declaration = semantic.scoping().symbol_declaration(symbol);

    if !matches!(nodes.kind(declaration), AstKind::ImportSpecifier(_) | AstKind::ImportDefaultSpecifier(_) | AstKind::ImportNamespaceSpecifier(_)) {
        return None;
    }

    match nodes.parent_kind(declaration) {
        AstKind::ImportDeclaration(import) => import.source.value.as_str(),
        _ => None,
    }
}

/// Tags from the Effect packages, whose classes a project cannot give a make.
fn is_effect_package(source: &str) -> bool {
    source == "effect" || source.starts_with("effect/") || source.starts_with("@effect/")
}
