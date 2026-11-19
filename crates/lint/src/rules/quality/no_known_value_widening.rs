//! `quality/no-known-value-widening`: no syntactically known value flowing
//! into an explicitly broad or anonymous target type.
//!
//! Runs the k-mistele fork's `no-known-value-widening` check ([`super::widening`]) with two differences:
//! target types resolve lexically ([`classify_widening_target`]), where a
//! `Record` with a narrow key is not an open dictionary and a generic alias
//! is a generic container only when it resolves to an open dictionary; and a
//! known argument passed to a local type predicate whose subject parameter is
//! `unknown` is reported too.

use oxc_ast::AstKind;
use oxc_ast::ast::{Argument, BindingPattern, CallExpression, Expression, TSType, TSTypePredicateName, VariableDeclarationKind};
use oxc_semantic::{AstNode, NodeId, Semantic, SymbolId};
use oxc_span::GetSpan;

use super::super::anti_slop::shared::params::{Param, Signature};
use super::super::anti_slop::shared::{parent, strip_parens, strip_type_parens};
use super::super::{Context, Rule};
use super::bindings::TypeBindings;
use super::dictionary::{classify_unsafe_value, classify_widening_target};
use super::widening::{check, function_name, is_known_evidence, only_declarator, unwrap};

pub const NAME: &str = "quality/no-known-value-widening";

pub struct NoKnownValueWidening;

impl Rule for NoKnownValueWidening {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        if let AstKind::CallExpression(call) = node.kind() {
            check_predicate_call(call, ctx);
            return;
        }

        check(node, ctx, |ctx, target| {
            let bindings = TypeBindings::of(ctx);

            classify_widening_target(bindings.with(ctx.semantic.nodes()), target)
        });
    }
}

/// A known argument for the `unknown` subject parameter of a local type
/// predicate (`input is T` or `asserts input is T`).
fn check_predicate_call<'a>(call: &'a CallExpression<'a>, ctx: &mut Context<'_, 'a>) {
    let semantic = ctx.semantic;
    let source = semantic.source_text();
    let Some(owner) = local_function_for_call(semantic, &call.callee) else {
        return;
    };

    let Some(signature) = Signature::of(semantic.nodes().kind(owner)) else {
        return;
    };

    let Some(TSType::TSTypePredicate(predicate)) = signature.return_type.map(|annotation| &annotation.type_annotation) else {
        return;
    };

    let TSTypePredicateName::Identifier(subject) = &predicate.parameter_name else {
        return;
    };

    // ESTree lists `this` among the parameters, so the index counts it too.
    let Some((index, parameter)) = signature.params().enumerate().find(|(_, parameter)| parameter.binding_display_name(source) == subject.name) else {
        return;
    };

    let Some(argument) = call.arguments.get(index).and_then(Argument::as_expression) else {
        return;
    };

    if !parameter.annotation().is_some_and(|annotation| contains_unknown(&annotation.type_annotation)) {
        return;
    }

    let bindings = TypeBindings::of(ctx);
    let informative = |ty: &'a TSType<'a>| classify_unsafe_value(bindings.with(semantic.nodes()), ty).is_none();

    if !has_known_call_argument_evidence(semantic, argument, &informative, &mut Vec::new()) {
        return;
    }

    let parameter = parameter.binding_display_name(source);
    let owner = function_name(semantic, Some(semantic.nodes().get_node(owner)));

    ctx.report(
        strip_parens(argument).span(),
        format!(
            "The explicit unknown type on argument for parameter `{parameter}` of `{owner}` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract."
        ),
    );
}

/// The plugin's `containsUnknownType`: `unknown`, through parentheses and union members.
fn contains_unknown(ty: &TSType<'_>) -> bool {
    match strip_type_parens(ty) {
        TSType::TSUnknownKeyword(_) => true,
        TSType::TSUnionType(union) => union.types.iter().any(contains_unknown),
        _ => false,
    }
}

/// Whether `id` is a function ESTree types as a function with a body or a
/// declared one: not a type, a signature or a method signature.
fn is_function(semantic: &Semantic<'_>, id: NodeId) -> bool {
    matches!(semantic.nodes().kind(id), AstKind::Function(_) | AstKind::ArrowFunctionExpression(_))
}

fn function_id(expression: &Expression<'_>) -> Option<NodeId> {
    match unwrap(expression) {
        Expression::FunctionExpression(function) => Some(function.node_id.get()),
        Expression::ArrowFunctionExpression(arrow) => Some(arrow.node_id.get()),
        _ => None,
    }
}

/// The local function a call invokes, as the plugin's `localFunctionForCall`:
/// a function expression called in place, a function declared once by name,
/// or one bound once by a variable initialized with it.
fn local_function_for_call(semantic: &Semantic<'_>, callee: &Expression<'_>) -> Option<NodeId> {
    if let Some(id) = function_id(callee) {
        return Some(id);
    }

    let symbol = resolve(semantic, unwrap(callee))?;
    let scoping = semantic.scoping();

    if !scoping.symbol_redeclarations(symbol).is_empty() {
        return None;
    }

    let declaration = scoping.symbol_declaration(symbol);

    match semantic.nodes().kind(declaration) {
        AstKind::Function(_) => Some(declaration),
        AstKind::VariableDeclarator(declarator) => declarator.init.as_ref().and_then(function_id),
        _ => None,
    }
}

/// The binding an identifier expression names, looked up by name from its scope.
fn resolve(semantic: &Semantic<'_>, expression: &Expression<'_>) -> Option<SymbolId> {
    let Expression::Identifier(identifier) = expression else {
        return None;
    };

    let scope = semantic.nodes().get_node(identifier.node_id.get()).scope_id();

    semantic.scoping().find_binding(scope, identifier.name)
}

/// The plugin's `hasKnownCallArgumentEvidence`: known evidence, or a value
/// whose assertion, annotation or local function's return type is informative.
fn has_known_call_argument_evidence<'a>(
    semantic: &Semantic<'a>,
    expression: &'a Expression<'a>,
    informative: &dyn Fn(&'a TSType<'a>) -> bool,
    visited: &mut Vec<SymbolId>,
) -> bool {
    match expression {
        Expression::ParenthesizedExpression(it) => has_known_call_argument_evidence(semantic, &it.expression, informative, visited),
        Expression::TSNonNullExpression(it) => has_known_call_argument_evidence(semantic, &it.expression, informative, visited),
        Expression::TSSatisfiesExpression(it) => has_known_call_argument_evidence(semantic, &it.expression, informative, visited),
        Expression::TSAsExpression(it) => informative(&it.type_annotation),
        Expression::TSTypeAssertion(it) => informative(&it.type_annotation),
        Expression::CallExpression(call) => local_function_for_call(semantic, &call.callee)
            .and_then(|owner| Signature::of(semantic.nodes().kind(owner)))
            .and_then(|signature| signature.return_type)
            .is_some_and(|annotation| informative(&annotation.type_annotation)),
        Expression::Identifier(_) => {
            let Some(symbol) = resolve(semantic, expression) else {
                return false;
            };

            if visited.contains(&symbol) {
                return false;
            }

            if let Some(annotation) = variable_type_annotation(semantic, symbol) {
                return informative(annotation);
            }

            let Some(declarator) = only_declarator(semantic, symbol) else {
                return false;
            };

            let Some(init) = &declarator.init else {
                return false;
            };

            let scoping = semantic.scoping();
            let is_const = matches!(semantic.nodes().parent_kind(declarator.node_id.get()), AstKind::VariableDeclaration(declaration) if declaration.kind == VariableDeclarationKind::Const);

            if !is_const || scoping.get_resolved_references(symbol).any(oxc_semantic::Reference::is_write) {
                return false;
            }

            visited.push(symbol);

            has_known_call_argument_evidence(semantic, init, informative, visited)
        }
        _ => is_known_evidence(expression),
    }
}

/// The plugin's `variableTypeAnnotation`: the annotation of a binding
/// declared once, by a plain variable or by a function parameter.
fn variable_type_annotation<'a>(semantic: &Semantic<'a>, symbol: SymbolId) -> Option<&'a TSType<'a>> {
    let scoping = semantic.scoping();

    if !scoping.symbol_redeclarations(symbol).is_empty() {
        return None;
    }

    let nodes = semantic.nodes();
    let declaration = scoping.symbol_declaration(symbol);

    match nodes.kind(declaration) {
        AstKind::VariableDeclarator(declarator) if matches!(declarator.id, BindingPattern::BindingIdentifier(_)) => {
            declarator.type_annotation.as_ref().map(|annotation| &annotation.type_annotation)
        }
        AstKind::FormalParameter(_) | AstKind::FormalParameterRest(_) => {
            // `parent` skips `FormalParameters`, which ESTree does not have.
            let function = parent(nodes, declaration);
            let signature = Signature::of(function.kind()).filter(|_| is_function(semantic, function.id()))?;
            let name = scoping.symbol_name(symbol);
            let parameter: Param<'a> = signature.params().find(|parameter| parameter.binding_display_name(semantic.source_text()) == name)?;

            parameter.annotation().map(|annotation| &annotation.type_annotation)
        }
        _ => None,
    }
}
