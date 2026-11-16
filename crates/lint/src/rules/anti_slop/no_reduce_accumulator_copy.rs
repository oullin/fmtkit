//! `anti-slop/no-reduce-accumulator-copy`: no copying a reducer's growing
//! accumulator on every iteration.
//!
//! Inside the callback of `.reduce(callback, init?)` or `.reduceRight(...)`,
//! reports `Object.assign({}, ..., acc, ...)`, `Array.from(acc)`, and
//! `acc.concat/slice/toSpliced/toSorted/toReversed/with(...)` when the initial
//! value is a known array. `acc` is the callback's first parameter, directly
//! or through unreassigned `const` aliases. Only the nearest function counts:
//! nested functions and function declarations are skipped, as are shadowed
//! `Object` and `Array`, and spreads (oxc's `no-accumulating-spread` covers
//! them). No options.

use oxc_ast::AstKind;
use oxc_ast::ast::{Argument, BindingPattern, CallExpression, Expression, FunctionType};
use oxc_semantic::{AstNode, NodeId, Semantic, SymbolId};
use oxc_span::{GetSpan, Span};

use super::super::{Context, Rule};
use super::shared::ancestors;
use super::shared::array_method::{self, Bare};

pub const NAME: &str = "anti-slop/no-reduce-accumulator-copy";

pub struct NoReduceAccumulatorCopy;

/// Array methods that return a copy of their receiver.
const COPY_METHODS: [&str; 6] = ["concat", "slice", "toSpliced", "toSorted", "toReversed", "with"];

/// The reducer callback a call sits in: its accumulator and the reduce
/// call's initial value.
struct Reducer<'a> {
    accumulator: SymbolId,
    initial_value: Option<&'a Argument<'a>>,
}

impl Rule for NoReduceAccumulatorCopy {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let AstKind::CallExpression(call) = node.kind() else {
            return;
        };

        let Some((name, object)) = array_method::method_target(&call.callee) else {
            return;
        };

        if !matches!(name, "assign" | "from") && !COPY_METHODS.contains(&name) {
            return;
        }

        let semantic = ctx.semantic;
        let Some(reducer) = enclosing_reducer(semantic, node.id()) else {
            return;
        };

        let is_accumulator =
            |argument: &Argument<'_>| argument.as_expression().is_some_and(|expression| references(semantic, expression, reducer.accumulator, &mut Vec::new()));

        let copies = if name == "assign" && is_global_owner(semantic, object, "Object") {
            call.arguments
                .first()
                .and_then(Argument::as_expression)
                .is_some_and(|target| matches!(array_method::unwrap(target), Bare::Other(Expression::ObjectExpression(_))))
                && call.arguments.iter().skip(1).any(is_accumulator)
        } else if name == "from" && is_global_owner(semantic, object, "Array") {
            call.arguments.first().is_some_and(is_accumulator)
        } else if COPY_METHODS.contains(&name) {
            reducer.initial_value.and_then(Argument::as_expression).is_some_and(|initial| array_method::is_known_array(semantic, initial, &mut Vec::new()))
                && references(semantic, object, reducer.accumulator, &mut Vec::new())
        } else {
            false
        };

        if copies {
            ctx.report(
                call.span,
                "Do not copy the reducer accumulator on every iteration; growing copies can cause quadratic work. Mutate a fresh, locally owned accumulator and return it, or use an iterator pipeline/flatMap.",
            );
        }
    }
}

/// The reducer whose callback is the nearest function around `id`, as
/// upstream's `enclosingReducer`.
fn enclosing_reducer<'a>(semantic: &Semantic<'a>, id: NodeId) -> Option<Reducer<'a>> {
    let nodes = semantic.nodes();
    let mut walk = ancestors(nodes, id);
    let (span, accumulator) = loop {
        match walk.next()?.kind() {
            AstKind::Function(function) => match function.r#type {
                FunctionType::FunctionDeclaration | FunctionType::TSDeclareFunction => return None,
                FunctionType::FunctionExpression => {
                    // ESTree lists `this` as the first parameter, which declares no variable.
                    let first = if function.this_param.is_some() { None } else { function.params.items.first() };

                    break (function.span, first.map(|param| &param.pattern));
                }
                FunctionType::TSEmptyBodyFunctionExpression => {}
            },
            AstKind::ArrowFunctionExpression(arrow) => break (arrow.span, arrow.params.items.first().map(|param| &param.pattern)),
            _ => {}
        }
    };

    let owner = walk.find(|node| {
        !matches!(node.kind(), AstKind::TSAsExpression(_) | AstKind::TSSatisfiesExpression(_) | AstKind::TSTypeAssertion(_) | AstKind::TSNonNullExpression(_))
    })?;
    let AstKind::CallExpression(owner) = owner.kind() else {
        return None;
    };

    if !is_reduce_of(owner, span) {
        return None;
    }

    let Some(BindingPattern::BindingIdentifier(accumulator)) = accumulator else {
        return None;
    };

    Some(Reducer { accumulator: accumulator.symbol_id.get()?, initial_value: owner.arguments.get(1) })
}

/// Whether `call` is `.reduce(callback, init?)` or `.reduceRight(...)` with the
/// function at `callback` as its first argument.
fn is_reduce_of(call: &CallExpression<'_>, callback: Span) -> bool {
    array_method::method_target(&call.callee).is_some_and(|(name, _)| name == "reduce" || name == "reduceRight")
        && call.arguments.len() <= 2
        && call
            .arguments
            .first()
            .and_then(Argument::as_expression)
            .is_some_and(|first| matches!(array_method::unwrap(first), Bare::Other(expression) if expression.span() == callback))
}

/// Whether `expression` names `accumulator`, directly or through unreassigned
/// `const` aliases, as upstream's `referencesAccumulator`.
fn references(semantic: &Semantic<'_>, expression: &Expression<'_>, accumulator: SymbolId, visited: &mut Vec<SymbolId>) -> bool {
    let Some(symbol) = array_method::resolve(semantic, expression) else {
        return false;
    };

    if visited.contains(&symbol) {
        return false;
    }

    if symbol == accumulator {
        return true;
    }

    visited.push(symbol);

    !array_method::is_reassigned(semantic, symbol)
        && array_method::const_initializer(semantic, symbol).is_some_and(|init| references(semantic, init, accumulator, visited))
}

/// Whether `expression` is the identifier `name` with no local declaration.
fn is_global_owner(semantic: &Semantic<'_>, expression: &Expression<'_>, name: &str) -> bool {
    matches!(array_method::unwrap(expression), Bare::Other(Expression::Identifier(identifier)) if identifier.name == name)
        && array_method::resolve(semantic, expression).is_none()
}
