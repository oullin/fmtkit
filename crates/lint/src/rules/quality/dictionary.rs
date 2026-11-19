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
use oxc_ast::ast::{TSInterfaceDeclaration, TSMappedType, TSSignature, TSType, TSTypeAliasDeclaration, TSTypeLiteral, TSTypeOperatorOperator, TSTypeReference};
use rustc_hash::FxHashMap;

use super::super::anti_slop::shared::types::{UnsafeValue, WideningTarget, reference_name, type_arguments};
use super::bindings::{Binding, Scoped};

const BUILT_INS: [&str; 8] = ["Record", "Readonly", "Partial", "Required", "Pick", "Omit", "PropertyKey", "NonNullable"];

const TRANSPARENT_WRAPPERS: [&str; 4] = ["Readonly", "Partial", "Required", "NonNullable"];

/// Type parameter names bound to their arguments while resolving an alias.
type Substitutions<'a> = FxHashMap<&'a str, &'a TSType<'a>>;

/// Alias names already being resolved, to stop at cycles.
type Resolving<'a> = Vec<&'a str>;

/// A dictionary value type with the substitutions in force where it appears.
type ValueType<'a> = (&'a TSType<'a>, Substitutions<'a>);

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

fn is_unapplied_reference_to(ty: &TSType<'_>, name: &str) -> bool {
    matches!(unwrap_transparent(ty), TSType::TSTypeReference(reference) if reference_name(reference) == Some(name) && type_arguments(reference).is_empty())
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

fn resolved_substitution_argument<'a>(ty: &'a TSType<'a>, base: &Substitutions<'a>, resolving: &mut Resolving<'a>) -> &'a TSType<'a> {
    let TSType::TSTypeReference(reference) = unwrap_transparent(ty) else {
        return ty;
    };

    let Some(name) = reference_name(reference).filter(|name| !resolving.contains(name)) else {
        return ty;
    };

    let Some(substitution) = base.get(name) else {
        return ty;
    };

    resolving.push(name);

    let resolved = resolved_substitution_argument(substitution, base, resolving);

    resolving.pop();

    resolved
}

fn alias_substitution<'a>(alias: &'a TSTypeAliasDeclaration<'a>, reference: &'a TSTypeReference<'a>, base: &Substitutions<'a>) -> Option<Substitutions<'a>> {
    let arguments = type_arguments(reference);
    let mut next = base.clone();

    for (index, parameter) in alias.type_parameters.iter().flat_map(|parameters| parameters.params.iter()).enumerate() {
        let argument = arguments.get(index).or(parameter.default.as_ref())?;
        let resolved = resolved_substitution_argument(argument, &next, &mut Vec::new());

        next.insert(parameter.name.name.as_str(), resolved);
    }

    Some(next)
}

fn with_name<'a>(resolving: &Resolving<'a>, name: &'a str) -> Resolving<'a> {
    let mut next = resolving.clone();

    next.push(name);
    next
}

fn unsafe_direct_value<'a>(scoped: Scoped<'_, 'a>, ty: &'a TSType<'a>, substitutions: &Substitutions<'a>, resolving: &Resolving<'a>) -> Option<UnsafeValue> {
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

    if is_transparent_wrapper(scoped, name, reference) {
        return unsafe_direct_value(scoped, type_arguments(reference).first()?, substitutions, resolving);
    }

    if let Some(substitution) = substitutions.get(name) {
        return if is_unapplied_reference_to(substitution, name) { None } else { unsafe_direct_value(scoped, substitution, substitutions, resolving) };
    }

    let Binding::Declared(id) = scoped.binding(name, reference.node_id.get()) else {
        return None;
    };

    let alias = match scoped.nodes().kind(id) {
        AstKind::TSInterfaceDeclaration(interface) => return is_effectively_empty_interface(interface).then_some(UnsafeValue::EmptyObject),
        AstKind::TSTypeAliasDeclaration(alias) if !resolving.contains(&name) => alias,
        _ => return None,
    };

    let next = alias_substitution(alias, reference, substitutions)?;

    unsafe_direct_value(scoped, &alias.type_annotation, &next, &with_name(resolving, name))
}

fn dictionary_value_types<'a>(scoped: Scoped<'_, 'a>, ty: &'a TSType<'a>, substitutions: &Substitutions<'a>, resolving: &Resolving<'a>) -> Vec<ValueType<'a>> {
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
    substitutions: &Substitutions<'a>,
    resolving: &Resolving<'a>,
) -> Vec<ValueType<'a>> {
    let Some(name) = reference_name(reference) else {
        return Vec::new();
    };

    if let Some(substitution) = substitutions.get(name) {
        return if is_unapplied_reference_to(substitution, name) { Vec::new() } else { dictionary_value_types(scoped, substitution, substitutions, resolving) };
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

    let Some(alias) = scoped.alias(name, reference.node_id.get()).filter(|_| !resolving.contains(&name)) else {
        return Vec::new();
    };

    let Some(next) = alias_substitution(alias, reference, substitutions) else {
        return Vec::new();
    };

    dictionary_value_types(scoped, &alias.type_annotation, &next, &with_name(resolving, name))
}

fn literal_value_types<'a>(literal: &'a TSTypeLiteral<'a>, substitutions: &Substitutions<'a>) -> Vec<ValueType<'a>> {
    literal
        .members
        .iter()
        .filter_map(|member| match member {
            TSSignature::TSIndexSignature(signature) => Some((&signature.type_annotation.type_annotation, substitutions.clone())),
            _ => None,
        })
        .collect()
}

fn mapped_value_types<'a>(mapped: &'a TSMappedType<'a>, substitutions: &Substitutions<'a>) -> Vec<ValueType<'a>> {
    mapped.type_annotation.as_ref().map_or_else(Vec::new, |value| vec![(value, substitutions.clone())])
}

fn first_unsafe<'a>(scoped: Scoped<'_, 'a>, values: Vec<ValueType<'a>>) -> Option<UnsafeValue> {
    values.into_iter().find_map(|(value, substitutions)| unsafe_direct_value(scoped, value, &substitutions, &Vec::new()))
}

/// Whether a dictionary value type is an unsafe escape hatch, as the plugin's
/// `classifyUnsafeDictionaryValue`.
pub fn classify_unsafe_value<'a>(scoped: Scoped<'_, 'a>, ty: &'a TSType<'a>) -> Option<UnsafeValue> {
    unsafe_direct_value(scoped, ty, &Substitutions::default(), &Vec::new())
}

/// How a type node met in the walk leaks an unsafe dictionary value type, as
/// the plugin's `classifyUnsafeDictionary`. Kinds that are never dictionaries
/// give `None`.
pub fn classify_unsafe_dictionary<'a>(scoped: Scoped<'_, 'a>, kind: AstKind<'a>) -> Option<UnsafeValue> {
    let empty = Substitutions::default();
    let values = match kind {
        AstKind::TSTypeLiteral(literal) => literal_value_types(literal, &empty),
        AstKind::TSMappedType(mapped) => mapped_value_types(mapped, &empty),
        AstKind::TSTypeReference(reference) => reference_value_types(scoped, reference, &empty, &Vec::new()),
        AstKind::TSTypeOperator(operator) if operator.operator == TSTypeOperatorOperator::Readonly => {
            dictionary_value_types(scoped, &operator.type_annotation, &empty, &Vec::new())
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

    if name == "Record" && is_built_in(scoped, name, reference) {
        return has_broad_record_key(scoped, reference, &Substitutions::default()).then_some(WideningTarget::OpenDictionary);
    }

    let alias = scoped.alias(name, reference.node_id.get())?;
    let substitutions = alias_substitution(alias, reference, &Substitutions::default())?;
    let resolved = classify_alias_broad_target(scoped, &alias.type_annotation, &substitutions, &vec![name]);

    if alias.type_parameters.as_ref().is_some_and(|parameters| !parameters.params.is_empty()) {
        return (resolved == Some(WideningTarget::OpenDictionary)).then_some(WideningTarget::GenericContainer);
    }

    resolved
}

fn has_broad_record_key<'a>(scoped: Scoped<'_, 'a>, reference: &'a TSTypeReference<'a>, substitutions: &Substitutions<'a>) -> bool {
    type_arguments(reference).first().is_none_or(|key| is_broad_mapped_key(scoped, key, substitutions, &Vec::new()))
}

fn is_broad_mapped_key<'a>(scoped: Scoped<'_, 'a>, ty: &'a TSType<'a>, substitutions: &Substitutions<'a>, visited: &Resolving<'a>) -> bool {
    let reference = match unwrap_transparent(ty) {
        TSType::TSStringKeyword(_) | TSType::TSNumberKeyword(_) | TSType::TSSymbolKeyword(_) => return true,
        TSType::TSUnionType(union) => return union.types.iter().any(|member| is_broad_mapped_key(scoped, member, substitutions, visited)),
        TSType::TSTypeReference(reference) => reference,
        _ => return false,
    };

    let Some(name) = reference_name(reference) else {
        return false;
    };

    if let Some(substitution) = substitutions.get(name).filter(|substitution| !is_unapplied_reference_to(substitution, name)) {
        return is_broad_mapped_key(scoped, substitution, substitutions, visited);
    }

    if name == "PropertyKey" && is_built_in(scoped, name, reference) {
        return true;
    }

    let Some(alias) = scoped.alias(name, reference.node_id.get()) else {
        return false;
    };

    if alias.type_parameters.as_ref().is_some_and(|parameters| !parameters.params.is_empty()) || visited.contains(&name) {
        return false;
    }

    is_broad_mapped_key(scoped, &alias.type_annotation, substitutions, &with_name(visited, name))
}

fn classify_alias_broad_target<'a>(
    scoped: Scoped<'_, 'a>,
    ty: &'a TSType<'a>,
    substitutions: &Substitutions<'a>,
    resolving: &Resolving<'a>,
) -> Option<WideningTarget> {
    let reference = match unwrap_transparent(ty) {
        TSType::TSUnknownKeyword(_) => return Some(WideningTarget::Unknown),
        TSType::TSObjectKeyword(_) => return Some(WideningTarget::Object),
        TSType::TSTypeLiteral(literal) => {
            return literal.members.iter().any(|member| matches!(member, TSSignature::TSIndexSignature(_))).then_some(WideningTarget::OpenDictionary);
        }
        TSType::TSMappedType(mapped) => {
            return is_broad_mapped_key(scoped, &mapped.constraint, substitutions, &Vec::new()).then_some(WideningTarget::OpenDictionary);
        }
        TSType::TSTypeReference(reference) => reference,
        _ => return None,
    };

    let name = reference_name(reference)?;

    if let Some(substitution) = substitutions.get(name) {
        return if is_unapplied_reference_to(substitution, name) { None } else { classify_alias_broad_target(scoped, substitution, substitutions, resolving) };
    }

    if is_transparent_wrapper(scoped, name, reference) {
        return classify_alias_broad_target(scoped, type_arguments(reference).first()?, substitutions, resolving);
    }

    if name == "Record" && is_built_in(scoped, name, reference) {
        return has_broad_record_key(scoped, reference, substitutions).then_some(WideningTarget::OpenDictionary);
    }

    let alias = scoped.alias(name, reference.node_id.get()).filter(|_| !resolving.contains(&name))?;
    let next = alias_substitution(alias, reference, substitutions)?;

    classify_alias_broad_target(scoped, &alias.type_annotation, &next, &with_name(resolving, name))
}
