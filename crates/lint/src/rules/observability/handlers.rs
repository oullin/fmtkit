//! Resolve callback declarations in this file through lexical bindings and
//! object members. This does not load imports or execute calls.

use oxc_ast::AstKind;
use oxc_ast::ast::{
    ArrowFunctionBody, ArrowFunctionExpression, Class, ClassElement, Expression, Function, FunctionBody, MethodDefinitionKind, ObjectExpression,
    ObjectPropertyKind, PropertyKind, Statement, ThisExpression, UnaryOperator,
};
use oxc_ast_visit::Visit;
use oxc_semantic::{AstNodes, NodeId, Semantic, SymbolId};
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
    Import(&'a str),
    NonCallable,
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

pub fn non_callable<'a>(expression: &Expression<'a>, semantic: &Semantic<'a>) -> bool {
    matches!(resolve_value(expression, semantic), Some(Value::NonCallable | Value::Object(_)))
}

/// Follow call receivers and lexical aliases to an import, without inferring
/// return types or exempting an unresolved receiver.
pub fn zod_schema<'s, 'a>(mut expression: &'s Expression<'a>, semantic: &'s Semantic<'a>) -> bool {
    let mut properties = Vec::new();
    let mut seen = FxHashSet::default();

    for _ in 0..semantic.nodes().len() {
        expression = unwrap(expression);

        if !seen.insert(expression.span()) {
            return false;
        }

        if let Expression::CallExpression(call) = expression {
            expression = &call.callee;
            continue;
        }

        match resolve_expression(expression, semantic, &mut properties) {
            Some(Value::Expression(next)) => expression = next,
            Some(Value::Import(source)) => return matches!(source, "zod" | "zod/v4" | "zod/mini"),
            _ => return false,
        }
    }

    false
}

fn matches_handler<'a>(
    expression: &Expression<'a>,
    semantic: &Semantic<'a>,
    block: impl FnOnce(&FunctionBody<'a>) -> bool,
    expression_body: impl FnOnce(Option<&Expression<'a>>) -> bool,
) -> bool {
    match resolve_value(expression, semantic) {
        Some(Value::Function(function)) => function.body.as_ref().is_some_and(|body| block(body)),
        Some(Value::Arrow(arrow)) => match &arrow.body {
            ArrowFunctionBody::FunctionBody(body) => block(body),
            body => expression_body(body.as_expression()),
        },
        _ => false,
    }
}

fn resolve_value<'s, 'a>(expression: &'s Expression<'a>, semantic: &'s Semantic<'a>) -> Option<Value<'s, 'a>> {
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
                    return None;
                }

                resolve_expression(expression, semantic, &mut properties)
            }
            value if properties.is_empty() => return Some(value),
            Value::Object(object) => properties.pop().and_then(|name| object_member(object, name)),
            Value::Class { class, instance } => properties.pop().and_then(|name| class_member(class, instance, name, semantic)),
            _ => return None,
        };
        let next = next?;

        value = next;
    }

    None
}

fn resolve_expression<'s, 'a>(expression: &'s Expression<'a>, semantic: &'s Semantic<'a>, properties: &mut Vec<&'a str>) -> Option<Value<'s, 'a>> {
    if let Some((owner, name, _)) = member(expression) {
        properties.push(name);

        return Some(Value::Expression(owner));
    }

    match expression {
        Expression::Identifier(id) => {
            let scoping = semantic.scoping();
            let symbol = scoping.get_reference(id.reference_id.get()?).symbol_id();

            if id.name == "undefined" && symbol.is_none() {
                return Some(Value::NonCallable);
            }

            let symbol = symbol?;

            if mutated(symbol, semantic) {
                return None;
            }

            let declaration = scoping.symbol_declaration(symbol);

            match semantic.nodes().kind(declaration) {
                AstKind::VariableDeclarator(declaration) => Some(Value::Expression(declaration.init.as_ref()?)),
                AstKind::Function(function) => Some(Value::Function(function)),
                AstKind::Class(class) => Some(Value::Class { class, instance: false }),
                AstKind::ImportSpecifier(_) | AstKind::ImportDefaultSpecifier(_) | AstKind::ImportNamespaceSpecifier(_) => {
                    semantic.nodes().ancestor_kinds(declaration).find_map(|kind| match kind {
                        AstKind::ImportDeclaration(import) => Some(Value::Import(import.source.value.as_str())),
                        _ => None,
                    })
                }
                _ => None,
            }
        }
        Expression::ArrowFunctionExpression(arrow) => Some(Value::Arrow(arrow)),
        Expression::FunctionExpression(function) => Some(Value::Function(function)),
        Expression::ObjectExpression(object) => Some(Value::Object(object)),
        Expression::ClassExpression(class) => Some(Value::Class { class, instance: false }),
        Expression::NewExpression(new) => Some(Value::Class { class: constructor(&new.callee, semantic)?, instance: true }),
        Expression::ThisExpression(this) => this_owner(this, semantic),
        Expression::ArrayExpression(_) | Expression::TemplateLiteral(_) | Expression::UnaryExpression(_) => Some(Value::NonCallable),
        expression if expression.is_literal() => Some(Value::NonCallable),
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

                if !seen.insert(symbol) || mutated(symbol, semantic) {
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

/// A binding written after its declaration, or a member reached through it or
/// through a lexical alias of it that is assigned, updated or deleted, may hold
/// any value at the call. It stays unresolved rather than resolving to its
/// initialiser. Aliases made by calls or parameters are not followed.
fn mutated(symbol: SymbolId, semantic: &Semantic<'_>) -> bool {
    mutated_through(symbol, true, semantic, &mut FxHashSet::default())
}

/// `rebinding` says whether a write to the binding itself counts: it does for
/// the resolved binding, not for an alias, whose own reassignment leaves the
/// original object as it was. Member writes through an alias still count.
fn mutated_through(symbol: SymbolId, rebinding: bool, semantic: &Semantic<'_>, seen: &mut FxHashSet<SymbolId>) -> bool {
    if !seen.insert(symbol) {
        return false;
    }

    let nodes = semantic.nodes();

    semantic.scoping().get_resolved_references(symbol).any(|reference| {
        (rebinding && reference.is_write())
            || member_written(reference.node_id(), nodes)
            || alias_of(reference.node_id(), nodes).is_some_and(|alias| mutated_through(alias, false, semantic, seen))
    })
}

/// The binding an `alias = reference` or `alias = reference.member` declarator
/// makes, through parentheses and TypeScript wrappers.
fn alias_of(reference: NodeId, nodes: &AstNodes<'_>) -> Option<SymbolId> {
    let mut target = reference;

    for ancestor in nodes.ancestors(reference) {
        match ancestor.kind() {
            // An alias of a member reached from the reference is followed too; a reference inside a computed key is not.
            AstKind::ComputedMemberExpression(member) if member.object.span() != nodes.get_node(target).kind().span() => return None,
            AstKind::IdentifierReference(_)
            | AstKind::StaticMemberExpression(_)
            | AstKind::ComputedMemberExpression(_)
            | AstKind::PrivateFieldExpression(_)
            | AstKind::ParenthesizedExpression(_)
            | AstKind::TSAsExpression(_)
            | AstKind::TSSatisfiesExpression(_)
            | AstKind::TSNonNullExpression(_)
            | AstKind::TSTypeAssertion(_) => target = ancestor.id(),
            AstKind::VariableDeclarator(declarator) => {
                if declarator.init.as_ref()?.span() != nodes.get_node(target).kind().span() {
                    return None;
                }

                return declarator.id.get_binding_identifier()?.symbol_id.get();
            }
            _ => return None,
        }
    }

    None
}

fn member_written(reference: NodeId, nodes: &AstNodes<'_>) -> bool {
    let mut target = reference;

    for ancestor in nodes.ancestors(reference) {
        match ancestor.kind() {
            // A member is followed only from its receiver: a reference inside a computed key names nothing written.
            AstKind::ComputedMemberExpression(member) if member.object.span() != nodes.get_node(target).kind().span() => return false,
            AstKind::IdentifierReference(_)
            | AstKind::StaticMemberExpression(_)
            | AstKind::ComputedMemberExpression(_)
            | AstKind::PrivateFieldExpression(_)
            | AstKind::ParenthesizedExpression(_)
            | AstKind::TSAsExpression(_)
            | AstKind::TSSatisfiesExpression(_)
            | AstKind::TSNonNullExpression(_)
            | AstKind::TSTypeAssertion(_) => target = ancestor.id(),
            AstKind::AssignmentExpression(assignment) => return target != reference && assignment.left.span() == nodes.get_node(target).kind().span(),
            AstKind::UpdateExpression(_) => return target != reference,
            AstKind::UnaryExpression(unary) => return target != reference && unary.operator == UnaryOperator::Delete,
            _ => return false,
        }
    }

    false
}

fn swallows_block<'a>(body: &FunctionBody<'a>, semantic: &Semantic<'a>) -> bool {
    if body.directives.is_empty() && body.statements.iter().all(|statement| matches!(statement, Statement::EmptyStatement(_))) {
        return true;
    }

    let mut returns = Contains::new(|node| matches!(node, AstKind::ReturnStatement(statement) if returns_undefined(statement.argument.as_ref(), semantic)));

    returns.visit_function_body(body);

    returns.found
}
