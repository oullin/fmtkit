//! `anti-slop-effect/require-cause-when-narrowing`: an `Effect.mapError`,
//! `Effect.catch`, `Effect.catchAll`, `Effect.catchCause` or `Effect.orElse`
//! handler that ignores the error it receives must not construct a new error
//! without a `cause`; name the parameter and pass it on, or log it first.
//!
//! The handler is the call's last argument, an arrow or function. It ignores
//! the error when it has no parameter, or one plain identifier parameter that
//! is never read. A constructed error is `new X(...)` or `X.make(...)` for a
//! capitalized `X` that is not an Effect module, returned, failed with
//! `Effect.fail`, or `yield*`ed, also inside an `Effect.gen` generator the
//! handler runs but not in other nested functions. The first such
//! construction without an object argument holding `cause` or a spread is
//! reported. A `tapError`, `tapErrorCause`, `tapCause` or `tapErrorTag` step
//! earlier in the pipe, or in the data-first source, logs the error first and
//! allows the narrowing. `Effect` is recognized by its imports from `effect`
//! or `effect/Effect`. Takes no options.

use oxc_ast::AstKind;
use oxc_ast::ast::{Argument, Expression, FormalParameter, ObjectPropertyKind};
use oxc_semantic::{AstNode, AstNodes, Semantic};
use oxc_span::{GetSpan, Span};

use super::super::anti_slop::shared::strip_parens;
use super::super::{Context, Rule};
use super::shared::effect_modules::{
    FunctionNode, Module, Modules, Step, callee_name, estree_parent, gen_function, is_function, key_name, member_name, preceding_steps, returned_expressions,
    unwrap, walk,
};

pub const NAME: &str = "anti-slop-effect/require-cause-when-narrowing";

const NARROWING_OPERATORS: [&str; 5] = ["mapError", "catch", "catchAll", "catchCause", "orElse"];
const TAP_OPERATORS: [&str; 4] = ["tapError", "tapErrorCause", "tapCause", "tapErrorTag"];

pub struct RequireCauseWhenNarrowing;

impl Rule for RequireCauseWhenNarrowing {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let AstKind::CallExpression(call) = node.kind() else {
            return;
        };

        if !matches!(unwrap(&call.callee), Expression::StaticMemberExpression(member) if NARROWING_OPERATORS.contains(&member.property.name.as_str())) {
            return;
        }

        let modules = Modules::of(ctx);
        let Some(operator) = callee_name(call, &modules, Module::Effect).filter(|operator| NARROWING_OPERATORS.contains(operator)) else {
            return;
        };
        let Some(handler) = call.arguments.last().and_then(Argument::as_expression).and_then(FunctionNode::of) else {
            return;
        };
        let semantic = ctx.semantic;

        if !ignores_error(semantic, handler) {
            return;
        }

        let nodes = semantic.nodes();
        let Some(constructed) = constructed_errors(nodes, handler, &modules).into_iter().find(|construction| !passes_cause(construction)) else {
            return;
        };
        let earlier = match call.arguments.first().filter(|_| call.arguments.len() > 1) {
            Some(source) => source.as_expression().into_iter().collect(),
            None => preceding_steps(nodes, node),
        };

        if earlier.iter().any(|step| is_tap_step(step, &modules)) {
            return;
        }

        let callee = unwrap(constructed.callee);
        let error = match callee {
            Expression::StaticMemberExpression(member) => strip_parens(&member.object).span(),
            Expression::ComputedMemberExpression(member) => strip_parens(&member.object).span(),
            Expression::PrivateFieldExpression(member) => strip_parens(&member.object).span(),
            _ => callee.span(),
        };
        let source = ctx.source();
        let error = &source[error.start as usize..error.end as usize];

        ctx.report(
            constructed.span,
            format!(
                "This Effect.{operator} handler discards the original error and constructs a new one without it. Name the handler's parameter and pass it as `cause` (`(error) => new {error}({{ ..., cause: error }})`), or log it first with `Effect.tapError(Effect.logError)` before narrowing."
            ),
        );
    }
}

/// Whether the handler never reads the error it receives.
fn ignores_error(semantic: &Semantic<'_>, handler: FunctionNode<'_>) -> bool {
    let (this_param, params) = match handler {
        FunctionNode::Arrow(arrow) => (false, &*arrow.params),
        FunctionNode::Function(function) => (function.this_param.is_some(), &*function.params),
    };

    // ESTree lists a `this` parameter first, and its variable never has
    // references, so upstream counts such a handler as ignoring the error.
    if this_param {
        return true;
    }

    let Some(parameter) = params.items.first() else {
        return params.rest.is_none();
    };

    let Some(identifier) = plain_identifier(parameter) else {
        return false;
    };

    identifier.symbol_id.get().is_some_and(|symbol| semantic.scoping().get_resolved_reference_ids(symbol).is_empty())
}

/// The identifier of a parameter ESTree shows as a bare `Identifier`.
fn plain_identifier<'b, 'a>(parameter: &'b FormalParameter<'a>) -> Option<&'b oxc_ast::ast::BindingIdentifier<'a>> {
    if parameter.initializer.is_some() || parameter.accessibility.is_some() || parameter.readonly || parameter.r#override {
        return None;
    }

    match &parameter.pattern {
        oxc_ast::ast::BindingPattern::BindingIdentifier(identifier) => Some(identifier),
        _ => None,
    }
}

/// A `new X(...)` or `X.make(...)` error construction.
struct Construction<'a> {
    span: Span,
    callee: &'a Expression<'a>,
    arguments: &'a [Argument<'a>],
}

/// `new X(...)` for an identifier or member `X`, or `X.make(...)` for a
/// capitalized `X` that is not an Effect module.
fn error_construction<'a>(expression: &'a Expression<'a>, modules: &Modules) -> Option<Construction<'a>> {
    match expression {
        Expression::NewExpression(new) => matches!(
            unwrap(&new.callee),
            Expression::Identifier(_) | Expression::StaticMemberExpression(_) | Expression::ComputedMemberExpression(_) | Expression::PrivateFieldExpression(_)
        )
        .then(|| Construction { span: new.span, callee: &new.callee, arguments: &new.arguments }),
        Expression::CallExpression(call) => {
            let Expression::StaticMemberExpression(callee) = unwrap(&call.callee) else {
                return None;
            };

            if callee.property.name != "make" {
                return None;
            }

            let owner = match unwrap(&callee.object) {
                Expression::Identifier(owner) => owner.name.as_str(),
                Expression::StaticMemberExpression(owner) => owner.property.name.as_str(),
                Expression::ComputedMemberExpression(owner) => match strip_parens(&owner.expression) {
                    Expression::Identifier(property) => property.name.as_str(),
                    _ => return None,
                },
                _ => return None,
            };

            if !owner.starts_with(|c: char| c.is_ascii_uppercase()) {
                return None;
            }

            let callee_expression = unwrap(&call.callee);

            if Module::ALL.into_iter().any(|module| member_name(callee_expression, modules, module).is_some()) {
                return None;
            }

            Some(Construction { span: call.span, callee: &call.callee, arguments: &call.arguments })
        }
        _ => None,
    }
}

/// Whether an argument is an object literal with a `cause` property or a
/// spread that may supply one.
fn passes_cause(construction: &Construction<'_>) -> bool {
    construction.arguments.iter().any(|argument| {
        let Some(Expression::ObjectExpression(object)) = argument.as_expression().map(unwrap) else {
            return false;
        };

        object.properties.iter().any(|property| match property {
            ObjectPropertyKind::SpreadProperty(_) => true,
            ObjectPropertyKind::ObjectProperty(property) => !property.computed && key_name(&property.key) == Some("cause"),
        })
    })
}

/// The errors a handler constructs, in upstream's order: its returned
/// expressions first, then its body in source order, without descending into
/// nested functions other than an `Effect.gen` generator it runs.
fn constructed_errors<'a>(nodes: &AstNodes<'a>, handler: FunctionNode<'a>, modules: &Modules) -> Vec<Construction<'a>> {
    let mut found: Vec<Construction<'a>> = Vec::new();
    let mut consider = |expression: &'a Expression<'a>| {
        let mut expression = unwrap(expression);

        while let Expression::YieldExpression(yielded) = expression {
            match &yielded.argument {
                Some(argument) => expression = unwrap(argument),
                None => return,
            }
        }

        if let Some(construction) = error_construction(expression, modules)
            && !found.iter().any(|seen| seen.span == construction.span)
        {
            found.push(construction);
        }
    };

    for returned in returned_expressions(nodes, handler) {
        consider(returned);
    }

    let body = match handler {
        FunctionNode::Arrow(arrow) => Some(arrow.body.node_id()),
        FunctionNode::Function(function) => function.body.as_ref().map(|body| body.node_id()),
    };

    if let Some(body) = body {
        walk(nodes, body, |node| match node.kind() {
            kind if is_function(kind) => {
                let is_generator = match (kind, estree_parent(nodes, node.id()).map(AstNode::kind)) {
                    (AstKind::Function(function), Some(AstKind::CallExpression(call))) => {
                        gen_function(call, modules).is_some_and(|generator| generator.span == function.span)
                    }
                    _ => false,
                };

                if is_generator { Step::Into } else { Step::Over }
            }
            AstKind::CallExpression(call) => {
                if callee_name(call, modules, Module::Effect) == Some("fail")
                    && let Some(argument) = call.arguments.first().and_then(Argument::as_expression)
                {
                    consider(argument);
                }

                Step::Into
            }
            AstKind::YieldExpression(yielded) => {
                if yielded.delegate
                    && let Some(argument) = &yielded.argument
                {
                    consider(argument);
                }

                Step::Into
            }
            _ => Step::Into,
        });
    }

    found
}

/// Whether `step` is a call of a tap operator, or the operator itself.
fn is_tap_step(step: &Expression<'_>, modules: &Modules) -> bool {
    let step = unwrap(step);
    let member = match step {
        Expression::CallExpression(call) => callee_name(call, modules, Module::Effect),
        _ => None,
    }
    .or_else(|| member_name(step, modules, Module::Effect));

    member.is_some_and(|member| TAP_OPERATORS.contains(&member))
}
