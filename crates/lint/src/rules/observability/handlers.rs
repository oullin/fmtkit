//! Resolve callback declarations in this file through lexical bindings and
//! object members. This does not load imports or execute calls.

use oxc_ast::AstKind;
use oxc_ast::ast::{
    ArrowFunctionBody, ArrowFunctionExpression, Class, ClassElement, Expression, Function, FunctionBody, MethodDefinitionKind, ObjectExpression,
    ObjectPropertyKind, PropertyKind, Statement, ThisExpression,
};
use oxc_ast_visit::Visit;
use oxc_semantic::Semantic;
use oxc_span::GetSpan;
use rustc_hash::FxHashSet;

use super::catches::returns_undefined;
use super::shared::{Contains, member, single_throw, unwrap};

enum Value<'s, 'a> {
    Expression(&'s Expression<'a>),
    Function(&'s Function<'a>),
    Arrow(&'s ArrowFunctionExpression<'a>),
    Object(&'s ObjectExpression<'a>),
    Class { class: &'s Class<'a>, instance: bool },
}

/// A real member path is short. Aliases that grow a member path on every
/// cycle (a = a.x) stop here instead of running to the node bound.
const MAX_PATH: usize = 32;

pub fn swallows<'a>(expression: &Expression<'a>, semantic: &Semantic<'a>) -> bool {
    matches_handler(expression, semantic, |body| swallows_block(body, semantic), |body| returns_undefined(body, semantic))
}

pub fn translates<'a>(expression: &Expression<'a>, semantic: &Semantic<'a>) -> bool {
    matches_handler(expression, semantic, |body| body.directives.is_empty() && single_throw(&body.statements), |_| false)
}

fn matches_handler<'a>(
    expression: &Expression<'a>,
    semantic: &Semantic<'a>,
    block: impl FnOnce(&FunctionBody<'a>) -> bool,
    expression_body: impl FnOnce(Option<&Expression<'a>>) -> bool,
) -> bool {
    let mut value = Value::Expression(expression);
    let mut properties = Vec::new();
    let mut seen = FxHashSet::default();

    // Resolution is iterative even for long alias chains; the node bound stops
    // any cycle that `seen` does not.
    for _ in 0..semantic.nodes().len() {
        let next = match value {
            Value::Expression(expression) => {
                let expression = unwrap(expression);

                if properties.len() > MAX_PATH || !seen.insert((expression.span(), properties.clone())) {
                    return false;
                }

                resolve_expression(expression, semantic, &mut properties)
            }
            Value::Function(function) => return properties.is_empty() && function.body.as_ref().is_some_and(|body| block(body)),
            Value::Arrow(arrow) => {
                return properties.is_empty()
                    && match &arrow.body {
                        ArrowFunctionBody::FunctionBody(body) => block(body),
                        body => expression_body(body.as_expression()),
                    };
            }
            Value::Object(object) => properties.pop().and_then(|name| object_member(object, name)),
            Value::Class { class, instance } => properties.pop().and_then(|name| class_member(class, instance, name, semantic)),
        };
        let Some(next) = next else { return false };

        value = next;
    }

    false
}

fn resolve_expression<'s, 'a>(expression: &'s Expression<'a>, semantic: &'s Semantic<'a>, properties: &mut Vec<&'a str>) -> Option<Value<'s, 'a>> {
    if let Some((owner, name, _)) = member(expression) {
        properties.push(name);

        return Some(Value::Expression(owner));
    }

    match expression {
        Expression::Identifier(id) => {
            let scoping = semantic.scoping();
            let symbol = scoping.get_reference(id.reference_id.get()?).symbol_id()?;

            // A binding written after its declaration may hold any value at the
            // call, so it stays unresolved rather than resolving to its initialiser.
            if scoping.get_resolved_references(symbol).any(oxc_semantic::Reference::is_write) {
                return None;
            }

            match semantic.nodes().kind(scoping.symbol_declaration(symbol)) {
                AstKind::VariableDeclarator(declaration) => Some(Value::Expression(declaration.init.as_ref()?)),
                AstKind::Function(function) => Some(Value::Function(function)),
                AstKind::Class(class) => Some(Value::Class { class, instance: false }),
                _ => None,
            }
        }
        Expression::ArrowFunctionExpression(arrow) => Some(Value::Arrow(arrow)),
        Expression::FunctionExpression(function) => Some(Value::Function(function)),
        Expression::ObjectExpression(object) => Some(Value::Object(object)),
        Expression::ClassExpression(class) => Some(Value::Class { class, instance: false }),
        Expression::NewExpression(new) => Some(Value::Class { class: constructor(&new.callee, semantic)?, instance: true }),
        Expression::ThisExpression(this) => this_owner(this, semantic),
        _ => None,
    }
}

fn object_member<'s, 'a>(object: &'s ObjectExpression<'a>, name: &str) -> Option<Value<'s, 'a>> {
    object.properties.iter().find_map(|property| match property {
        ObjectPropertyKind::ObjectProperty(property)
            if !property.shorthand && property.kind == PropertyKind::Init && property.key.static_name().is_some_and(|key| key == name) =>
        {
            Some(Value::Expression(&property.value))
        }
        _ => None,
    })
}

fn class_member<'s, 'a>(class: &'s Class<'a>, instance: bool, name: &str, semantic: &'s Semantic<'a>) -> Option<Value<'s, 'a>> {
    let mut class = class;
    let mut seen = FxHashSet::default();

    while seen.insert(class.node_id.get()) {
        if let Some(method) = class.body.body.iter().find_map(|element| match element {
            ClassElement::MethodDefinition(method)
                if method.kind == MethodDefinitionKind::Method && method.r#static != instance && method.key.static_name().is_some_and(|key| key == name) =>
            {
                Some(method)
            }
            _ => None,
        }) {
            return Some(Value::Function(&method.value));
        }

        class = constructor(&class.heritage.as_ref()?.expression, semantic)?;
    }

    None
}

fn this_owner<'s, 'a>(this: &ThisExpression, semantic: &'s Semantic<'a>) -> Option<Value<'s, 'a>> {
    let nodes = semantic.nodes();

    for ancestor in nodes.ancestors(this.node_id.get()) {
        match ancestor.kind() {
            AstKind::Function(function) => {
                return match nodes.parent_kind(function.node_id.get()) {
                    AstKind::ObjectProperty(_) => nodes.ancestor_kinds(ancestor.id()).find_map(|kind| match kind {
                        AstKind::ObjectExpression(object) => Some(Value::Object(object)),
                        _ => None,
                    }),
                    AstKind::MethodDefinition(method) => nodes.ancestor_kinds(ancestor.id()).find_map(|kind| match kind {
                        AstKind::Class(class) => Some(Value::Class { class, instance: !method.r#static }),
                        _ => None,
                    }),
                    _ => None,
                };
            }
            AstKind::Class(class) => return Some(Value::Class { class, instance: false }),
            _ => {}
        }
    }

    None
}

fn constructor<'s, 'a>(mut expression: &'s Expression<'a>, semantic: &'s Semantic<'a>) -> Option<&'s Class<'a>> {
    let mut seen = FxHashSet::default();

    loop {
        match unwrap(expression) {
            Expression::ClassExpression(class) => return Some(class),
            Expression::Identifier(id) => {
                let scoping = semantic.scoping();
                let symbol = scoping.get_reference(id.reference_id.get()?).symbol_id()?;

                if !seen.insert(symbol) {
                    return None;
                }

                match semantic.nodes().kind(scoping.symbol_declaration(symbol)) {
                    AstKind::Class(class) => return Some(class),
                    AstKind::VariableDeclarator(declaration) => expression = declaration.init.as_ref()?,
                    _ => return None,
                }
            }
            _ => return None,
        }
    }
}

fn swallows_block<'a>(body: &FunctionBody<'a>, semantic: &Semantic<'a>) -> bool {
    if body.directives.is_empty() && body.statements.iter().all(|statement| matches!(statement, Statement::EmptyStatement(_))) {
        return true;
    }

    let mut returns = Contains::new(|node| matches!(node, AstKind::ReturnStatement(statement) if returns_undefined(statement.argument.as_ref(), semantic)));

    returns.visit_function_body(body);

    returns.found
}
