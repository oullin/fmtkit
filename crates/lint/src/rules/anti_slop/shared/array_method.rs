//! Array-method receivers and the local evidence that a value is an array,
//! as upstream's `shared/array-method.ts`.

use oxc_ast::AstKind;
use oxc_ast::ast::{
    BindingPattern, CallExpression, ChainElement, Expression, MemberExpression, TSType, TSTypeName, TSTypeOperatorOperator, VariableDeclarationKind,
};
use oxc_semantic::{Semantic, SymbolId};

use super::strip_parens;

/// An expression without the syntax-only wrappers upstream looks through:
/// parentheses, optional chains, `as`, `<T>`, `!` and `satisfies`.
pub enum Bare<'b, 'a> {
    Call(&'b CallExpression<'a>),
    Member(&'b MemberExpression<'a>),
    Other(&'b Expression<'a>),
}

/// `expression` as upstream's `unwrapArrayExpression` leaves it.
pub fn unwrap<'b, 'a>(mut expression: &'b Expression<'a>) -> Bare<'b, 'a> {
    loop {
        expression = match expression {
            Expression::ParenthesizedExpression(it) => &it.expression,
            Expression::TSAsExpression(it) => &it.expression,
            Expression::TSTypeAssertion(it) => &it.expression,
            Expression::TSNonNullExpression(it) => &it.expression,
            Expression::TSSatisfiesExpression(it) => &it.expression,
            Expression::ChainExpression(chain) => match &chain.expression {
                ChainElement::CallExpression(call) => return Bare::Call(call),
                ChainElement::TSNonNullExpression(it) => &it.expression,
                element => return element.as_member_expression().map_or(Bare::Other(expression), Bare::Member),
            },
            Expression::CallExpression(call) => return Bare::Call(call),
            _ => return expression.as_member_expression().map_or(Bare::Other(expression), Bare::Member),
        };
    }
}

/// The statically named method and its receiver: `a.b`, `a["b"]`, through
/// wrappers; not `a[b]` or `a.#b`.
pub fn method_target<'b, 'a>(callee: &'b Expression<'a>) -> Option<(&'a str, &'b Expression<'a>)> {
    match unwrap(callee) {
        Bare::Member(MemberExpression::StaticMemberExpression(member)) => Some((member.property.name.as_str(), &member.object)),
        Bare::Member(MemberExpression::ComputedMemberExpression(member)) => match strip_parens(&member.expression) {
            Expression::StringLiteral(key) => key.value.as_str().map(|key| (key, &member.object)),
            _ => None,
        },
        _ => None,
    }
}

/// The binding an identifier names, looked up by name through its scopes as
/// upstream's `resolveArrayBinding`.
pub fn resolve(semantic: &Semantic<'_>, expression: &Expression<'_>) -> Option<SymbolId> {
    let Bare::Other(Expression::Identifier(identifier)) = unwrap(expression) else {
        return None;
    };

    let scope = semantic.nodes().get_node(identifier.node_id.get()).scope_id();

    semantic.scoping().find_binding(scope, identifier.name)
}

/// Whether the binding is ever reassigned after its declaration.
pub fn is_reassigned(semantic: &Semantic<'_>, symbol: SymbolId) -> bool {
    semantic.scoping().get_resolved_references(symbol).any(oxc_semantic::Reference::is_write)
}

/// The initializer of a `const name = init` declaring `symbol`, as the first
/// of upstream's `Variable` definitions it accepts.
pub fn const_initializer<'a>(semantic: &Semantic<'a>, symbol: SymbolId) -> Option<&'a Expression<'a>> {
    let nodes = semantic.nodes();

    semantic.scoping().symbol_declarations(symbol).find_map(|id| {
        let AstKind::VariableDeclarator(declarator) = nodes.kind(id) else {
            return None;
        };

        let is_const = matches!(nodes.parent_kind(id), AstKind::VariableDeclaration(declaration) if declaration.kind == VariableDeclarationKind::Const);

        (is_const && matches!(declarator.id, BindingPattern::BindingIdentifier(_))).then_some(declarator.init.as_ref()).flatten()
    })
}

/// The annotation on the identifier itself, as ESTree places it: a plain
/// variable, parameter or catch binding; not a rest element or a pattern.
fn identifier_annotation<'a>(semantic: &Semantic<'a>, symbol: SymbolId) -> Option<&'a TSType<'a>> {
    let nodes = semantic.nodes();

    semantic.scoping().symbol_declarations(symbol).find_map(|id| {
        let (pattern, annotation) = match nodes.kind(id) {
            AstKind::VariableDeclarator(it) => (&it.id, it.type_annotation.as_deref()),
            AstKind::FormalParameter(it) => (&it.pattern, it.type_annotation.as_deref()),
            AstKind::CatchParameter(it) => (&it.pattern, it.type_annotation.as_deref()),
            _ => return None,
        };

        matches!(pattern, BindingPattern::BindingIdentifier(_)).then_some(annotation).flatten().map(|annotation| &annotation.type_annotation)
    })
}

fn is_array_annotation(ty: &TSType<'_>) -> bool {
    match ty {
        TSType::TSArrayType(_) | TSType::TSTupleType(_) => true,
        TSType::TSParenthesizedType(it) => is_array_annotation(&it.type_annotation),
        TSType::TSTypeOperatorType(it) => it.operator == TSTypeOperatorOperator::Readonly && is_array_annotation(&it.type_annotation),
        TSType::TSTypeReference(it) => matches!(&it.type_name, TSTypeName::IdentifierReference(name) if name.name == "Array" || name.name == "ReadonlyArray"),
        _ => false,
    }
}

/// Methods whose result is an array when their receiver is one.
const ARRAY_RETURNING: [&str; 8] = ["map", "filter", "flatMap", "slice", "concat", "toSorted", "toReversed", "toSpliced"];

/// Local evidence that `expression` is an array, as upstream's
/// `isKnownArrayExpression`: an array literal, an array-returning method of a
/// known array, or an unreassigned binding annotated as an array or bound
/// by `const` to a known array.
pub fn is_known_array(semantic: &Semantic<'_>, expression: &Expression<'_>, visited: &mut Vec<SymbolId>) -> bool {
    match unwrap(expression) {
        Bare::Other(Expression::ArrayExpression(_)) => return true,
        Bare::Call(call) => {
            return method_target(&call.callee).is_some_and(|(name, object)| ARRAY_RETURNING.contains(&name) && is_known_array(semantic, object, visited));
        }
        Bare::Other(Expression::Identifier(_)) => {}
        _ => return false,
    }

    let Some(symbol) = resolve(semantic, expression) else {
        return false;
    };

    if visited.contains(&symbol) {
        return false;
    }

    visited.push(symbol);

    if is_reassigned(semantic, symbol) {
        return false;
    }

    if let Some(annotation) = identifier_annotation(semantic, symbol) {
        return is_array_annotation(annotation);
    }

    const_initializer(semantic, symbol).is_some_and(|init| is_known_array(semantic, init, visited))
}
