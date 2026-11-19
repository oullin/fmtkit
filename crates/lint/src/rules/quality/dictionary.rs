//! Dictionary and widening classifications over lexically scoped bindings,
//! ported from the plugin's `shared/dictionary-types.js`.
//!
//! The logic follows anti-slop's [`Types`], with three differences: names
//! resolve lexically ([`Scoped`]), a built-in utility type counts as shadowed
//! only by a type declaration or type parameter in scope (not a function),
//! and `Record` widens to an open dictionary only with a broad key.
//!
//! [`Types`]: super::super::anti_slop::shared::types::Types

use oxc_ast::AstKind;
use oxc_ast::ast::{TSInterfaceDeclaration, TSMappedType, TSSignature, TSType, TSTypeLiteral, TSTypeOperatorOperator, TSTypeReference};
use oxc_semantic::NodeId;

use super::super::anti_slop::shared::types::{UnsafeValue, WideningTarget, reference_name, type_arguments};
use super::bindings::{Binding, Scope, Scoped, Substitution, alias_substitutions};

const BUILT_INS: [&str; 8] = ["Record", "Readonly", "Partial", "Required", "Pick", "Omit", "PropertyKey", "NonNullable"];

const TRANSPARENT_WRAPPERS: [&str; 4] = ["Readonly", "Partial", "Required", "NonNullable"];

/// The alias declarations being expanded, to stop at cycles. Keyed by
/// declaration, so two aliases that share a name in different scopes are
/// told apart.
type Resolving = Vec<NodeId>;

/// A dictionary value type with the substitutions in force where it appears.
type ValueType<'a> = (&'a TSType<'a>, Scope<'a>);

/// `ty` without parentheses and `readonly` operators.
fn unwrap_transparent<'b, 'a>(mut ty: &'b TSType<'a>) -> &'b TSType<'a> {
    loop {
        match ty {
            TSType::TSParenthesizedType(inner) => ty = &inner.type_annotation,
            TSType::TSTypeOperatorType(operator) if operator.operator == TSTypeOperatorOperator::Readonly => ty = &operator.type_annotation,
            _ => return ty,
        }
    }
}

/// Whether the utility type `name` is the built-in one where `reference` uses it.
fn is_built_in(scoped: Scoped<'_, '_>, name: &str, reference: &TSTypeReference<'_>) -> bool {
    BUILT_INS.contains(&name) && scoped.binding(name, reference.node_id.get()) == Binding::Unbound
}

fn is_transparent_wrapper(scoped: Scoped<'_, '_>, name: &str, reference: &TSTypeReference<'_>) -> bool {
    TRANSPARENT_WRAPPERS.contains(&name) && is_built_in(scoped, name, reference)
}

fn is_never(ty: &TSType<'_>) -> bool {
    matches!(unwrap_transparent(ty), TSType::TSNeverKeyword(_))
}

fn is_effectively_empty_member(member: &TSSignature<'_>) -> bool {
    matches!(member, TSSignature::TSPropertySignature(property) if property.optional && property.type_annotation.as_ref().is_some_and(|annotation| is_never(&annotation.type_annotation)))
}

fn is_effectively_empty_literal(literal: &TSTypeLiteral<'_>) -> bool {
    literal.members.iter().all(is_effectively_empty_member)
}

fn is_effectively_empty_interface(interface: &TSInterfaceDeclaration<'_>) -> bool {
    interface.extends.is_empty() && interface.body.body.iter().all(is_effectively_empty_member)
}

/// What a type parameter reference `name` stands for in `substitutions`. A
/// reference with type arguments is not a type parameter.
fn substitution<'a>(substitutions: &Scope<'a>, name: &str, reference: &TSTypeReference<'a>) -> Option<Substitution<'a>> {
    substitutions.get(name).filter(|_| type_arguments(reference).is_empty()).cloned()
}

/// Run `f` with `alias` marked as being expanded.
fn expanding<T>(resolving: &mut Resolving, alias: NodeId, f: impl FnOnce(&mut Resolving) -> T) -> T {
    resolving.push(alias);

    let result = f(resolving);

    resolving.pop();
    result
}

fn unsafe_direct_value<'a>(scoped: Scoped<'_, 'a>, ty: &'a TSType<'a>, substitutions: &Scope<'a>, resolving: &mut Resolving) -> Option<UnsafeValue> {
    let reference = match unwrap_transparent(ty) {
        TSType::TSUnknownKeyword(_) => return Some(UnsafeValue::Unknown),
        TSType::TSAnyKeyword(_) => return Some(UnsafeValue::Any),
        TSType::TSObjectKeyword(_) => return Some(UnsafeValue::Object),
        TSType::TSTypeLiteral(literal) => return is_effectively_empty_literal(literal).then_some(UnsafeValue::EmptyObject),
        TSType::TSUnionType(union) => {
            return union.types.iter().any(|member| unsafe_direct_value(scoped, member, substitutions, resolving).is_some()).then_some(UnsafeValue::Union);
        }
        TSType::TSIntersectionType(intersection) => {
            let members: Vec<_> = intersection.types.iter().map(|member| unsafe_direct_value(scoped, member, substitutions, resolving)).collect();

            if members.contains(&Some(UnsafeValue::Any)) {
                return Some(UnsafeValue::Any);
            }

            return if members.iter().all(Option::is_some) { members.first().copied().flatten() } else { None };
        }
        TSType::TSTypeReference(reference) => reference,
        _ => return None,
    };

    let name = reference_name(reference)?;

    if let Some(substitution) = substitution(substitutions, name, reference) {
        return unsafe_direct_value(scoped, substitution.ty, &substitution.scope, resolving);
    }

    if is_transparent_wrapper(scoped, name, reference) {
        return unsafe_direct_value(scoped, type_arguments(reference).first()?, substitutions, resolving);
    }

    let Binding::Declared(id) = scoped.binding(name, reference.node_id.get()) else {
        return None;
    };

    let alias = match scoped.nodes().kind(id) {
        AstKind::TSInterfaceDeclaration(interface) => return is_effectively_empty_interface(interface).then_some(UnsafeValue::EmptyObject),
        AstKind::TSTypeAliasDeclaration(alias) if !resolving.contains(&id) => alias,
        _ => return None,
    };

    let next = alias_substitutions(alias, reference, substitutions)?;

    expanding(resolving, id, |resolving| unsafe_direct_value(scoped, &alias.type_annotation, &next, resolving))
}

fn dictionary_value_types<'a>(scoped: Scoped<'_, 'a>, ty: &'a TSType<'a>, substitutions: &Scope<'a>, resolving: &mut Resolving) -> Vec<ValueType<'a>> {
    match unwrap_transparent(ty) {
        TSType::TSTypeLiteral(literal) => literal_value_types(literal, substitutions),
        TSType::TSMappedType(mapped) => mapped_value_types(mapped, substitutions),
        TSType::TSTypeReference(reference) => reference_value_types(scoped, reference, substitutions, resolving),
        _ => Vec::new(),
    }
}

fn reference_value_types<'a>(
    scoped: Scoped<'_, 'a>,
    reference: &'a TSTypeReference<'a>,
    substitutions: &Scope<'a>,
    resolving: &mut Resolving,
) -> Vec<ValueType<'a>> {
    let Some(name) = reference_name(reference) else {
        return Vec::new();
    };

    if let Some(substitution) = substitution(substitutions, name, reference) {
        return dictionary_value_types(scoped, substitution.ty, &substitution.scope, resolving);
    }

    let arguments = type_arguments(reference);

    if is_transparent_wrapper(scoped, name, reference) {
        return arguments.first().map_or_else(Vec::new, |wrapped| dictionary_value_types(scoped, wrapped, substitutions, resolving));
    }

    if name == "Record" && is_built_in(scoped, name, reference) {
        return arguments.get(1).map_or_else(Vec::new, |value| vec![(value, substitutions.clone())]);
    }

    if (name == "Pick" || name == "Omit") && is_built_in(scoped, name, reference) {
        return arguments.first().map_or_else(Vec::new, |source| dictionary_value_types(scoped, source, substitutions, resolving));
    }

    let Some(alias) = scoped.alias(name, reference.node_id.get()).filter(|alias| !resolving.contains(&alias.node_id.get())) else {
        return Vec::new();
    };

    let Some(next) = alias_substitutions(alias, reference, substitutions) else {
        return Vec::new();
    };

    expanding(resolving, alias.node_id.get(), |resolving| dictionary_value_types(scoped, &alias.type_annotation, &next, resolving))
}

fn literal_value_types<'a>(literal: &'a TSTypeLiteral<'a>, substitutions: &Scope<'a>) -> Vec<ValueType<'a>> {
    literal
        .members
        .iter()
        .filter_map(|member| match member {
            TSSignature::TSIndexSignature(signature) => Some((&signature.type_annotation.type_annotation, substitutions.clone())),
            _ => None,
        })
        .collect()
}

fn mapped_value_types<'a>(mapped: &'a TSMappedType<'a>, substitutions: &Scope<'a>) -> Vec<ValueType<'a>> {
    mapped.type_annotation.as_ref().map_or_else(Vec::new, |value| vec![(value, substitutions.clone())])
}

fn first_unsafe<'a>(scoped: Scoped<'_, 'a>, values: Vec<ValueType<'a>>) -> Option<UnsafeValue> {
    values.into_iter().find_map(|(value, substitutions)| unsafe_direct_value(scoped, value, &substitutions, &mut Vec::new()))
}

/// Whether a dictionary value type is an unsafe escape hatch, as the plugin's
/// `classifyUnsafeDictionaryValue`.
pub fn classify_unsafe_value<'a>(scoped: Scoped<'_, 'a>, ty: &'a TSType<'a>) -> Option<UnsafeValue> {
    unsafe_direct_value(scoped, ty, &Scope::default(), &mut Vec::new())
}

/// How a type node met in the walk leaks an unsafe dictionary value type, as
/// the plugin's `classifyUnsafeDictionary`. Kinds that are never dictionaries
/// give `None`.
pub fn classify_unsafe_dictionary<'a>(scoped: Scoped<'_, 'a>, kind: AstKind<'a>) -> Option<UnsafeValue> {
    let empty = Scope::default();
    let values = match kind {
        AstKind::TSTypeLiteral(literal) => literal_value_types(literal, &empty),
        AstKind::TSMappedType(mapped) => mapped_value_types(mapped, &empty),
        AstKind::TSTypeReference(reference) => reference_value_types(scoped, reference, &empty, &mut Vec::new()),
        AstKind::TSTypeOperator(operator) if operator.operator == TSTypeOperatorOperator::Readonly => {
            dictionary_value_types(scoped, &operator.type_annotation, &empty, &mut Vec::new())
        }
        // Not `TSParenthesizedType`: ESTree has no such node, so it never
        // encloses another type there.
        _ => return None,
    };

    first_unsafe(scoped, values)
}

/// How broad an explicit annotation or assertion target is, as the plugin's
/// `classifyWideningTarget`.
pub fn classify_widening_target<'a>(scoped: Scoped<'_, 'a>, ty: &'a TSType<'a>) -> Option<WideningTarget> {
    let reference = match unwrap_transparent(ty) {
        TSType::TSUnknownKeyword(_) => return Some(WideningTarget::Unknown),
        TSType::TSObjectKeyword(_) => return Some(WideningTarget::Object),
        TSType::TSTypeLiteral(literal) => {
            return if literal.members.iter().any(|member| matches!(member, TSSignature::TSIndexSignature(_))) {
                Some(WideningTarget::OpenDictionary)
            } else if literal.members.is_empty() {
                None
            } else {
                Some(WideningTarget::AnonymousObject)
            };
        }
        TSType::TSMappedType(_) => return Some(WideningTarget::OpenDictionary),
        TSType::TSTypeReference(reference) => reference,
        _ => return None,
    };

    let name = reference_name(reference)?;

    if is_transparent_wrapper(scoped, name, reference) {
        return classify_widening_target(scoped, type_arguments(reference).first()?);
    }

    let empty = Scope::default();

    if name == "Record" && is_built_in(scoped, name, reference) {
        return has_broad_record_key(scoped, reference, &empty).then_some(WideningTarget::OpenDictionary);
    }

    let alias = scoped.alias(name, reference.node_id.get())?;
    let substitutions = alias_substitutions(alias, reference, &empty)?;
    let resolved = classify_alias_broad_target(scoped, &alias.type_annotation, &substitutions, &mut vec![alias.node_id.get()]);

    if alias.type_parameters.as_ref().is_some_and(|parameters| !parameters.params.is_empty()) {
        return (resolved == Some(WideningTarget::OpenDictionary)).then_some(WideningTarget::GenericContainer);
    }

    resolved
}

fn has_broad_record_key<'a>(scoped: Scoped<'_, 'a>, reference: &'a TSTypeReference<'a>, substitutions: &Scope<'a>) -> bool {
    type_arguments(reference).first().is_none_or(|key| is_broad_mapped_key(scoped, key, substitutions, &mut Vec::new()))
}

fn is_broad_mapped_key<'a>(scoped: Scoped<'_, 'a>, ty: &'a TSType<'a>, substitutions: &Scope<'a>, visited: &mut Resolving) -> bool {
    let reference = match unwrap_transparent(ty) {
        TSType::TSStringKeyword(_) | TSType::TSNumberKeyword(_) | TSType::TSSymbolKeyword(_) => return true,
        TSType::TSUnionType(union) => return union.types.iter().any(|member| is_broad_mapped_key(scoped, member, substitutions, visited)),
        TSType::TSTypeReference(reference) => reference,
        _ => return false,
    };

    let Some(name) = reference_name(reference) else {
        return false;
    };

    if let Some(substitution) = substitution(substitutions, name, reference) {
        return is_broad_mapped_key(scoped, substitution.ty, &substitution.scope, visited);
    }

    if name == "PropertyKey" && is_built_in(scoped, name, reference) {
        return true;
    }

    let Some(alias) = scoped.alias(name, reference.node_id.get()) else {
        return false;
    };

    if alias.type_parameters.as_ref().is_some_and(|parameters| !parameters.params.is_empty()) || visited.contains(&alias.node_id.get()) {
        return false;
    }

    // A non-generic alias body sees no substitution.
    expanding(visited, alias.node_id.get(), |visited| is_broad_mapped_key(scoped, &alias.type_annotation, &Scope::default(), visited))
}

fn classify_alias_broad_target<'a>(scoped: Scoped<'_, 'a>, ty: &'a TSType<'a>, substitutions: &Scope<'a>, resolving: &mut Resolving) -> Option<WideningTarget> {
    let reference = match unwrap_transparent(ty) {
        TSType::TSUnknownKeyword(_) => return Some(WideningTarget::Unknown),
        TSType::TSObjectKeyword(_) => return Some(WideningTarget::Object),
        TSType::TSTypeLiteral(literal) => {
            return literal.members.iter().any(|member| matches!(member, TSSignature::TSIndexSignature(_))).then_some(WideningTarget::OpenDictionary);
        }
        TSType::TSMappedType(mapped) => {
            return is_broad_mapped_key(scoped, &mapped.constraint, substitutions, &mut Vec::new()).then_some(WideningTarget::OpenDictionary);
        }
        TSType::TSTypeReference(reference) => reference,
        _ => return None,
    };

    let name = reference_name(reference)?;

    if let Some(substitution) = substitution(substitutions, name, reference) {
        return classify_alias_broad_target(scoped, substitution.ty, &substitution.scope, resolving);
    }

    if is_transparent_wrapper(scoped, name, reference) {
        return classify_alias_broad_target(scoped, type_arguments(reference).first()?, substitutions, resolving);
    }

    if name == "Record" && is_built_in(scoped, name, reference) {
        return has_broad_record_key(scoped, reference, substitutions).then_some(WideningTarget::OpenDictionary);
    }

    let alias = scoped.alias(name, reference.node_id.get()).filter(|alias| !resolving.contains(&alias.node_id.get()))?;
    let next = alias_substitutions(alias, reference, substitutions)?;

    expanding(resolving, alias.node_id.get(), |resolving| classify_alias_broad_target(scoped, &alias.type_annotation, &next, resolving))
}
