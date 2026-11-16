//! `anti-slop-effect/require-operation-span`: outside test and spec files,
//! every operation a `Context.Service` class's make returns must be traced,
//! with `Effect.fn("name")` or `Effect.withSpan`.
//!
//! The make is the class options' `make` or a static `make`: an
//! `Effect.gen(function* () {...})`, optionally piped, or a function returning
//! one. The generator's own returns are read for object literals, directly,
//! through `X.of({...})`, or through a same-file variable. An operation is a
//! non-computed property whose value (or the single declaration a shorthand
//! or identifier names) is an `Effect.*` call, a pipe with an `Effect.*`
//! step, or a function returning one; plain helpers and streams are not. A
//! span anywhere inside the operation counts, as does a call of a same-file
//! function that names one. `Context` and `Effect` are recognized by their
//! imports from `effect` or `effect/<Module>`. Takes no options.

use oxc_ast::AstKind;
use oxc_ast::ast::{Argument, Expression, Function, ObjectExpression, ObjectPropertyKind};
use oxc_semantic::{AstNode, AstNodes, Semantic};
use oxc_span::GetSpan;

use super::super::anti_slop::shared::strip_parens;
use super::super::anti_slop::shared::testing::is_test_file;
use super::super::{Context, Rule};
use super::shared::effect_modules::{
    FunctionNode, Module, Modules, Value, callee_name, contains, make_generator, member_name, own_returns, pipe_receiver, resolve_initializer,
    returned_expressions, service_class, service_makes, unwrap,
};

pub const NAME: &str = "anti-slop-effect/require-operation-span";

pub struct RequireOperationSpan;

impl Rule for RequireOperationSpan {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let AstKind::Class(class) = node.kind() else {
            return;
        };

        if class.heritage.is_none() || is_test_file(ctx.rel) {
            return;
        }

        let modules = Modules::of(ctx);
        let Some(service) = service_class(class, &modules) else {
            return;
        };
        let semantic = ctx.semantic;
        let nodes = semantic.nodes();
        let service_name = service.name.unwrap_or("Service");
        let source = ctx.source();

        for make in service_makes(&service) {
            let Some(generator) = make_generator(nodes, make, &modules) else {
                continue;
            };

            for object in returned_objects(semantic, generator) {
                for property in &object.properties {
                    let ObjectPropertyKind::ObjectProperty(property) = property else {
                        continue;
                    };

                    if property.computed {
                        continue;
                    }

                    let Some(operation) = operation_of(semantic, &property.value, &modules) else {
                        continue;
                    };

                    if has_span(semantic, operation, &modules) {
                        continue;
                    }

                    let key = property.key.span();
                    let operation = &source[key.start as usize..key.end as usize];

                    ctx.report(
                        property.span,
                        format!(
                            "Operation \"{operation}\" of service \"{service_name}\" has no span. Define it with `Effect.fn(\"{service_name}.{operation}\")(function* (input) {{ ... }})`, or end its pipe with `Effect.withSpan(\"{service_name}.{operation}\", {{ attributes: {{ ... }} }})`."
                        ),
                    );
                }
            }
        }
    }
}

/// `Effect.fn("name")` with a literal span name, or `Effect.withSpan(...)`.
fn is_span_call(kind: AstKind<'_>, modules: &Modules) -> bool {
    let AstKind::CallExpression(call) = kind else {
        return false;
    };

    match callee_name(call, modules, Module::Effect) {
        Some("withSpan") => true,
        Some("fn") => matches!(
            call.arguments.first().and_then(Argument::as_expression).map(strip_parens),
            Some(Expression::StringLiteral(_) | Expression::TemplateLiteral(_))
        ),
        _ => false,
    }
}

/// Whether `operation` names a span itself, or calls a same-file function that does.
fn has_span(semantic: &Semantic<'_>, operation: Value<'_>, modules: &Modules) -> bool {
    let nodes = semantic.nodes();

    contains(nodes, operation.node_id(), |node| {
        if is_span_call(node.kind(), modules) {
            return true;
        }

        let AstKind::CallExpression(call) = node.kind() else {
            return false;
        };
        let Expression::Identifier(callee) = unwrap(&call.callee) else {
            return false;
        };

        resolve_initializer(semantic, callee).is_some_and(|initializer| contains(nodes, initializer.node_id(), |inner| is_span_call(inner.kind(), modules)))
    })
}

/// Whether an expression evaluates to an Effect: an `Effect.*` call, or a pipe
/// with an `Effect.*` step.
fn is_effect_call(expression: &Expression<'_>, modules: &Modules) -> bool {
    let mut expression = unwrap(expression);

    while let Some(receiver) = pipe_receiver(expression) {
        if let Expression::CallExpression(call) = expression
            && call.arguments.iter().any(|step| {
                step.as_expression().map(unwrap).is_some_and(|step| {
                    matches!(step, Expression::CallExpression(step) if callee_name(step, modules, Module::Effect).is_some())
                        || member_name(step, modules, Module::Effect).is_some()
                })
            })
        {
            return true;
        }

        expression = receiver;
    }

    let Expression::CallExpression(call) = expression else {
        return false;
    };

    if callee_name(call, modules, Module::Effect).is_some() {
        return true;
    }

    // `Effect.fn("name")(function* () {})` is a call of a call.
    matches!(unwrap(&call.callee), Expression::CallExpression(callee) if callee_name(callee, modules, Module::Effect).is_some())
}

/// Whether a function evidently returns an Effect.
fn returns_effect<'a>(nodes: &AstNodes<'a>, function: FunctionNode<'a>, modules: &Modules) -> bool {
    returned_expressions(nodes, function).into_iter().any(|returned| is_effect_call(returned, modules))
}

fn is_operation<'a>(nodes: &AstNodes<'a>, value: Value<'a>, modules: &Modules) -> bool {
    match value {
        Value::Function(function) => returns_effect(nodes, FunctionNode::Function(function), modules),
        Value::Expression(expression) => match FunctionNode::of(expression) {
            Some(function) => returns_effect(nodes, function, modules),
            None => is_effect_call(expression, modules),
        },
    }
}

/// The operation a property holds, resolving a same-file identifier one level.
fn operation_of<'a>(semantic: &Semantic<'a>, value: &'a Expression<'a>, modules: &Modules) -> Option<Value<'a>> {
    let nodes = semantic.nodes();
    let expression = unwrap(value);

    if is_operation(nodes, Value::Expression(expression), modules) {
        return Some(Value::Expression(expression));
    }

    let Expression::Identifier(identifier) = expression else {
        return None;
    };
    let resolved = match resolve_initializer(semantic, identifier)? {
        Value::Expression(initializer) => Value::Expression(unwrap(initializer)),
        function @ Value::Function(_) => function,
    };

    is_operation(nodes, resolved, modules).then_some(resolved)
}

/// The object literals a make generator returns at its top level, directly,
/// through `X.of({...})`, or through a same-file variable.
fn returned_objects<'a>(semantic: &Semantic<'a>, generator: &'a Function<'a>) -> Vec<&'a ObjectExpression<'a>> {
    let Some(body) = &generator.body else {
        return Vec::new();
    };
    let mut objects = Vec::new();

    for statement in own_returns(semantic.nodes(), body.node_id()) {
        let Some(argument) = &statement.argument else {
            continue;
        };
        let mut expression = unwrap(argument);

        if let Expression::CallExpression(call) = expression
            && let Expression::StaticMemberExpression(callee) = unwrap(&call.callee)
            && callee.property.name == "of"
            && let Some(argument) = call.arguments.first().and_then(Argument::as_expression)
        {
            expression = unwrap(argument);
        }

        if let Expression::Identifier(identifier) = expression
            && let Some(Value::Expression(initializer)) = resolve_initializer(semantic, identifier)
        {
            expression = unwrap(initializer);
        }

        if let Expression::ObjectExpression(object) = expression {
            objects.push(&**object);
        }
    }

    objects
}
