//! `anti-slop/no-widen-then-assert`: no local `const` that widens a known value
//! and is later asserted back to a narrower type.

use oxc_ast::AstKind;
use oxc_ast::ast::{BindingPattern, Expression, IdentifierReference, TSSignature, TSType, TSTypeOperatorOperator, VariableDeclarationKind, VariableDeclarator};
use oxc_semantic::{AstNode, NodeId, Semantic, SymbolId};
use oxc_span::{GetSpan, Span};

use super::super::{Context, Rule};
use super::shared::types::{reference_name, type_arguments};
use super::shared::{enclosing_function, is_js_space, strip_parens, strip_type_parens};

pub const NAME: &str = "anti-slop/no-widen-then-assert";

pub struct NoWidenThenAssert;

impl Rule for NoWidenThenAssert {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let (span, expression, asserted) = match node.kind() {
            AstKind::TSAsExpression(it) => (it.span, &it.expression, &it.type_annotation),
            AstKind::TSTypeAssertion(it) => (it.span, &it.expression, &it.type_annotation),
            _ => return,
        };

        let semantic = ctx.semantic;
        let Expression::Identifier(identifier) = strip_parens(expression) else {
            return;
        };

        let Some(widened) = resolve(semantic, identifier).and_then(|symbol| widened_binding(semantic, symbol)) else {
            return;
        };

        if span.start <= widened.declared_at
            || function_boundary(semantic, node.id()) != widened.boundary
            || !is_narrower(semantic.source_text(), widened.kind, widened.evidence, asserted)
        {
            return;
        }

        let name = identifier.name;

        ctx.report(
            span,
            format!(
                "Binding \"{name}\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once."
            ),
        );
    }
}

/// How broad a widening type is.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Broad {
    Top,
    Object,
    Record,
}

/// What is known of a value: its asserted or declared type, if it has one.
type Evidence<'a> = Option<&'a TSType<'a>>;

struct Widened<'a> {
    kind: Broad,
    evidence: Evidence<'a>,
    declared_at: u32,
    boundary: Option<NodeId>,
}

fn resolve(semantic: &Semantic<'_>, identifier: &IdentifierReference<'_>) -> Option<SymbolId> {
    let scoping = semantic.scoping();

    scoping.get_reference(identifier.reference_id()).symbol_id()
}

/// The nearest function of any kind around `id`, bodyless ones included.
fn function_boundary(semantic: &Semantic<'_>, id: NodeId) -> Option<NodeId> {
    enclosing_function(semantic.nodes(), id, true).map(AstNode::id)
}

fn is_top(ty: &TSType<'_>) -> bool {
    matches!(strip_type_parens(ty), TSType::TSUnknownKeyword(_) | TSType::TSAnyKeyword(_))
}

fn is_broad_record_key(ty: &TSType<'_>) -> bool {
    match strip_type_parens(ty) {
        TSType::TSStringKeyword(_) | TSType::TSNumberKeyword(_) | TSType::TSSymbolKeyword(_) => true,
        TSType::TSUnionType(union) => union.types.iter().all(is_broad_record_key),
        TSType::TSTypeReference(reference) => reference_name(reference) == Some("PropertyKey"),
        _ => false,
    }
}

fn is_broad_record(ty: &TSType<'_>) -> bool {
    match strip_type_parens(ty) {
        TSType::TSTypeReference(reference) => match (reference_name(reference), type_arguments(reference)) {
            (Some("Readonly"), [inner, ..]) => is_broad_record(inner),
            (Some("Record"), [key, value]) => is_broad_record_key(key) && is_top(value),
            _ => false,
        },
        TSType::TSTypeLiteral(literal) => match literal.members.as_slice() {
            [TSSignature::TSIndexSignature(signature)] => {
                is_broad_record_key(&signature.parameter.type_annotation.type_annotation) && is_top(&signature.type_annotation.type_annotation)
            }
            _ => false,
        },
        _ => false,
    }
}

fn broad_kind(ty: &TSType<'_>) -> Option<Broad> {
    match strip_type_parens(ty) {
        TSType::TSUnknownKeyword(_) | TSType::TSAnyKeyword(_) => Some(Broad::Top),
        TSType::TSObjectKeyword(_) => Some(Broad::Object),
        ty => is_broad_record(ty).then_some(Broad::Record),
    }
}

/// The operand and target of an `as` or angle-bracket assertion, through parentheses.
fn assertion<'b, 'a>(expression: &'b Expression<'a>) -> Option<(&'b Expression<'a>, &'b TSType<'a>)> {
    match strip_parens(expression) {
        Expression::TSAsExpression(it) => Some((strip_parens(&it.expression), &it.type_annotation)),
        Expression::TSTypeAssertion(it) => Some((strip_parens(&it.expression), &it.type_annotation)),
        _ => None,
    }
}

/// The symbol's `const` declarator, when it is never written after it.
fn stable_const<'a>(semantic: &Semantic<'a>, symbol: SymbolId) -> Option<&'a VariableDeclarator<'a>> {
    let scoping = semantic.scoping();
    let nodes = semantic.nodes();
    let declarator = scoping
        .symbol_declarations(symbol)
        .chain(scoping.symbol_redeclarations(symbol).iter().map(|redeclaration| redeclaration.declaration))
        .find_map(|id| match nodes.kind(id) {
            AstKind::VariableDeclarator(declarator) => Some((id, declarator)),
            _ => None,
        });

    let (id, declarator) = declarator?;
    let is_const = matches!(nodes.parent_kind(id), AstKind::VariableDeclaration(declaration) if declaration.kind == VariableDeclarationKind::Const);

    (is_const && declarator.init.is_some() && !scoping.get_resolved_references(symbol).any(oxc_semantic::Reference::is_write)).then_some(declarator)
}

fn known_value_evidence<'a>(
    semantic: &Semantic<'a>,
    expression: &'a Expression<'a>,
    boundary: Option<NodeId>,
    visited: &mut Vec<SymbolId>,
) -> Option<Evidence<'a>> {
    let identifier = match strip_parens(expression) {
        Expression::TSAsExpression(it) => return broad_kind(&it.type_annotation).is_none().then_some(Some(&it.type_annotation)),
        Expression::TSTypeAssertion(it) => return broad_kind(&it.type_annotation).is_none().then_some(Some(&it.type_annotation)),
        Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::NumericLiteral(_)
        | Expression::BigIntLiteral(_)
        | Expression::RegExpLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::TemplateLiteral(_)
        | Expression::ArrayExpression(_)
        | Expression::ArrowFunctionExpression(_)
        | Expression::ClassExpression(_)
        | Expression::FunctionExpression(_)
        | Expression::NewExpression(_)
        | Expression::ObjectExpression(_) => return Some(None),
        Expression::Identifier(identifier) => identifier,
        _ => return None,
    };

    let symbol = resolve(semantic, identifier)?;

    if visited.contains(&symbol) {
        return None;
    }

    if let Some((id, annotation)) = annotated_identifier(semantic, symbol) {
        return (function_boundary(semantic, id) == boundary && broad_kind(annotation).is_none()).then_some(Some(annotation));
    }

    let declarator = stable_const(semantic, symbol)?;
    let declarator_id = declarator.node_id.get();

    if function_boundary(semantic, declarator_id) != boundary {
        return None;
    }

    visited.push(symbol);

    known_value_evidence(semantic, declarator.init.as_ref()?, boundary, visited)
}

/// The first declaration of `symbol` whose identifier carries a type
/// annotation in ESTree: a variable, a parameter or a catch clause binding.
fn annotated_identifier<'a>(semantic: &Semantic<'a>, symbol: SymbolId) -> Option<(NodeId, &'a TSType<'a>)> {
    let scoping = semantic.scoping();
    let nodes = semantic.nodes();
    let mut declarations =
        scoping.symbol_declarations(symbol).chain(scoping.symbol_redeclarations(symbol).iter().map(|redeclaration| redeclaration.declaration));

    declarations.find_map(|id| {
        let (pattern, annotation) = match nodes.kind(id) {
            AstKind::VariableDeclarator(it) => (&it.id, it.type_annotation.as_deref()),
            AstKind::FormalParameter(it) => (&it.pattern, it.type_annotation.as_deref()),
            AstKind::CatchParameter(it) => (&it.pattern, it.type_annotation.as_deref()),
            _ => return None,
        };

        matches!(pattern, BindingPattern::BindingIdentifier(_)).then_some(())?;
        annotation.map(|annotation| (id, &annotation.type_annotation))
    })
}

fn widened_binding<'a>(semantic: &Semantic<'a>, symbol: SymbolId) -> Option<Widened<'a>> {
    let declarator = stable_const(semantic, symbol)?;

    if !matches!(declarator.id, BindingPattern::BindingIdentifier(_)) {
        return None;
    }

    let init = declarator.init.as_ref()?;
    let boundary = function_boundary(semantic, declarator.node_id.get());
    let initializer = assertion(init);
    let initializer_kind = initializer.and_then(|(_, ty)| broad_kind(ty));
    let kind = declarator.type_annotation.as_ref().and_then(|annotation| broad_kind(&annotation.type_annotation)).or(initializer_kind)?;
    let original = match initializer {
        Some((operand, _)) if initializer_kind.is_some() => operand,
        _ => init,
    };

    let evidence = known_value_evidence(semantic, original, boundary, &mut vec![symbol])?;

    Some(Widened { kind, evidence, declared_at: declarator.span.end, boundary })
}

fn is_definitely_object(ty: &TSType<'_>) -> bool {
    match strip_type_parens(ty) {
        TSType::TSArrayType(_)
        | TSType::TSConstructorType(_)
        | TSType::TSFunctionType(_)
        | TSType::TSMappedType(_)
        | TSType::TSObjectKeyword(_)
        | TSType::TSTupleType(_) => true,
        TSType::TSTypeLiteral(literal) => !literal.members.is_empty(),
        TSType::TSIntersectionType(intersection) => intersection.types.iter().all(is_definitely_object),
        TSType::TSTypeOperatorType(operator) => operator.operator == TSTypeOperatorOperator::Readonly && is_definitely_object(&operator.type_annotation),
        _ => false,
    }
}

fn is_definitely_narrower_record(ty: &TSType<'_>) -> bool {
    match strip_type_parens(ty) {
        TSType::TSTypeLiteral(literal) => literal.members.iter().any(|member| !matches!(member, TSSignature::TSIndexSignature(_))),
        TSType::TSTypeReference(reference) => match (reference_name(reference), type_arguments(reference)) {
            (Some("Readonly"), [inner, ..]) => is_definitely_narrower_record(inner),
            (Some("Record"), [_, value]) => !is_top(value),
            _ => false,
        },
        _ => false,
    }
}

/// The source text of a type without any whitespace.
fn normalized(source: &str, ty: &TSType<'_>) -> String {
    let span: Span = strip_type_parens(ty).span();

    span.source_text(source).chars().filter(|&c| !is_js_space(c)).collect()
}

fn is_narrower(source: &str, kind: Broad, evidence: Evidence<'_>, asserted: &TSType<'_>) -> bool {
    if broad_kind(asserted).is_some() {
        return false;
    }

    if kind == Broad::Top || evidence.is_some_and(|evidence| normalized(source, evidence) == normalized(source, asserted)) {
        return true;
    }

    match kind {
        Broad::Object => is_definitely_object(asserted),
        _ => is_definitely_narrower_record(asserted),
    }
}
