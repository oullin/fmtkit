//! The module-level type environment and the dictionary and widening
//! classifications built on it, ported from v1's `shared/dictionary-types.ts`.
//!
//! The environment is per-file state shared by every rule that needs it
//! ([`TypeEnvironment::of`]); it records declarations by node id so it can
//! outlive the borrow of the AST.

use std::rc::Rc;

use oxc_ast::AstKind;
use oxc_ast::ast::{
    Declaration, ExportDefaultDeclarationKind, FunctionType, ImportDeclarationSpecifier, Statement, TSInterfaceDeclaration, TSMappedType, TSSignature, TSType,
    TSTypeAliasDeclaration, TSTypeLiteral, TSTypeName, TSTypeOperatorOperator, TSTypeReference,
};
use oxc_semantic::{AstNodes, NodeId};
use rustc_hash::FxHashMap;

use super::super::super::Context;

const BUILT_INS: [&str; 8] = ["Record", "Readonly", "Partial", "Required", "Pick", "Omit", "PropertyKey", "NonNullable"];

const TRANSPARENT_WRAPPERS: [&str; 4] = ["Readonly", "Partial", "Required", "NonNullable"];

/// Top-level type declarations of one file.
#[derive(Debug, Default)]
pub struct TypeEnvironment {
    aliases: FxHashMap<String, Aliases>,
    interfaces: FxHashMap<String, Vec<NodeId>>,
    /// Bit `i` set when `BUILT_INS[i]` is declared or imported locally.
    shadowed: u8,
}

/// The declarations of one alias name. Redeclarations are a semantic error,
/// but v1's rules disagree on which one wins, so all three are kept.
#[derive(Debug, Clone, Copy)]
struct Aliases {
    first: NodeId,
    last: NodeId,
    /// The last declaration without type parameters.
    last_plain: Option<NodeId>,
}

impl TypeEnvironment {
    /// The environment of the file `ctx` is linting, built once per file.
    pub fn of(ctx: &mut Context<'_, '_>) -> Rc<Self> {
        ctx.shared(|semantic| Self::new(semantic.nodes()))
    }

    fn new(nodes: &AstNodes<'_>) -> Self {
        let mut env = Self::default();

        for statement in &nodes.program().body {
            match statement {
                Statement::ImportDeclaration(import) => {
                    for specifier in import.specifiers.iter().flatten() {
                        let local = match specifier {
                            ImportDeclarationSpecifier::ImportSpecifier(it) => &it.local,
                            ImportDeclarationSpecifier::ImportDefaultSpecifier(it) => &it.local,
                            ImportDeclarationSpecifier::ImportNamespaceSpecifier(it) => &it.local,
                        };

                        env.shadow(&local.name);
                    }
                }
                Statement::ExportDeclaration(export) => env.declare(&export.declaration),
                Statement::ExportDefaultDeclaration(export) => match &export.declaration {
                    ExportDefaultDeclarationKind::FunctionDeclaration(function) if function.r#type == FunctionType::FunctionDeclaration => {
                        env.shadow_named(function.id.as_ref().map(|id| id.name.as_str()));
                    }
                    ExportDefaultDeclarationKind::ClassDeclaration(class) => env.shadow_named(class.id.as_ref().map(|id| id.name.as_str())),
                    ExportDefaultDeclarationKind::TSInterfaceDeclaration(interface) => env.declare_interface(interface),
                    _ => {}
                },
                _ => {
                    if let Some(declaration) = statement.as_declaration() {
                        env.declare(declaration);
                    }
                }
            }
        }

        env
    }

    fn declare(&mut self, declaration: &Declaration<'_>) {
        match declaration {
            Declaration::TSTypeAliasDeclaration(alias) => {
                let name = alias.id.name.as_str();
                let id = alias.node_id.get();
                let plain = alias.type_parameters.is_none().then_some(id);

                if let Some(existing) = self.aliases.get_mut(name) {
                    existing.last = id;
                    existing.last_plain = plain.or(existing.last_plain);
                    self.shadow(name);
                } else {
                    self.aliases.insert(name.to_owned(), Aliases { first: id, last: id, last_plain: plain });
                }

                self.shadow(name);
            }
            Declaration::TSInterfaceDeclaration(interface) => self.declare_interface(interface),
            Declaration::TSEnumDeclaration(declaration) => self.shadow(&declaration.id.name),
            Declaration::ClassDeclaration(class) => self.shadow_named(class.id.as_ref().map(|id| id.name.as_str())),
            Declaration::FunctionDeclaration(function) if function.r#type == FunctionType::FunctionDeclaration => {
                self.shadow_named(function.id.as_ref().map(|id| id.name.as_str()));
            }
            _ => {}
        }
    }

    fn declare_interface(&mut self, interface: &TSInterfaceDeclaration<'_>) {
        self.interfaces.entry(interface.id.name.as_str().to_owned()).or_default().push(interface.node_id.get());
        self.shadow(&interface.id.name);
    }

    fn shadow_named(&mut self, name: Option<&str>) {
        if let Some(name) = name {
            self.shadow(name);
        }
    }

    fn shadow(&mut self, name: &str) {
        if let Some(index) = BUILT_INS.iter().position(|built_in| *built_in == name) {
            self.shadowed |= 1 << index;
        }
    }

    /// Whether `name` is one of the utility types v1 understands, not
    /// redeclared or imported in this file.
    fn is_built_in(&self, name: &str) -> bool {
        BUILT_INS.iter().position(|built_in| *built_in == name).is_some_and(|index| self.shadowed & (1 << index) == 0)
    }

    fn is_transparent_wrapper(&self, name: &str) -> bool {
        TRANSPARENT_WRAPPERS.contains(&name) && self.is_built_in(name)
    }

    /// Whether a top-level alias of this name exists.
    pub fn has_alias(&self, name: &str) -> bool {
        self.aliases.contains_key(name)
    }

    /// The node id of the alias declaration that wins for `no-unknown-*`.
    pub fn last_alias_id(&self, name: &str) -> Option<NodeId> {
        self.aliases.get(name).map(|aliases| aliases.last)
    }

    /// Bind the environment to the nodes of its file.
    pub fn with<'s, 'a>(&'s self, nodes: &'s AstNodes<'a>) -> Types<'s, 'a> {
        Types { env: self, nodes }
    }
}

/// A [`TypeEnvironment`] together with the nodes its ids point into.
#[derive(Clone, Copy)]
pub struct Types<'s, 'a> {
    env: &'s TypeEnvironment,
    nodes: &'s AstNodes<'a>,
}

/// What an unsafe dictionary value type is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsafeValue {
    Any,
    EmptyObject,
    Object,
    Union,
    Unknown,
}

impl UnsafeValue {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Any => "any",
            Self::EmptyObject => "empty-object",
            Self::Object => "object",
            Self::Union => "union",
            Self::Unknown => "unknown",
        }
    }
}

/// How broad an explicit target type is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WideningTarget {
    AnonymousObject,
    GenericContainer,
    Object,
    OpenDictionary,
    Unknown,
}

impl WideningTarget {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AnonymousObject => "anonymous object",
            Self::GenericContainer => "generic container",
            Self::Object => "object",
            Self::OpenDictionary => "open dictionary",
            Self::Unknown => "unknown",
        }
    }
}

/// Type parameter names bound to their arguments while resolving an alias.
type Substitutions<'a> = FxHashMap<&'a str, &'a TSType<'a>>;

/// Alias names already being resolved, to stop at cycles.
type Resolving<'a> = Vec<&'a str>;

/// A dictionary value type with the substitutions in force where it appears.
type ValueType<'a> = (&'a TSType<'a>, Substitutions<'a>);

/// `ty` without parentheses and `readonly` operators.
pub fn unwrap_transparent<'a>(mut ty: &'a TSType<'a>) -> &'a TSType<'a> {
    loop {
        match ty {
            TSType::TSParenthesizedType(inner) => ty = &inner.type_annotation,
            TSType::TSTypeOperatorType(operator) if operator.operator == TSTypeOperatorOperator::Readonly => ty = &operator.type_annotation,
            _ => return ty,
        }
    }
}

/// The name of a reference to a plain identifier.
pub fn reference_name<'a>(reference: &TSTypeReference<'a>) -> Option<&'a str> {
    match &reference.type_name {
        TSTypeName::IdentifierReference(identifier) => Some(identifier.name.as_str()),
        _ => None,
    }
}

/// The type arguments of a reference, or none.
pub fn type_arguments<'a>(reference: &'a TSTypeReference<'a>) -> &'a [TSType<'a>] {
    reference.type_arguments.as_ref().map_or(&[], |arguments| arguments.params.as_slice())
}

fn is_unapplied_reference_to(ty: &TSType<'_>, name: &str) -> bool {
    matches!(unwrap_transparent_any(ty), TSType::TSTypeReference(reference) if reference_name(reference) == Some(name) && reference.type_arguments.as_ref().is_none_or(|arguments| arguments.params.is_empty()))
}

/// [`unwrap_transparent`] for borrows that are not tied to the arena.
fn unwrap_transparent_any<'b, 'a>(mut ty: &'b TSType<'a>) -> &'b TSType<'a> {
    loop {
        match ty {
            TSType::TSParenthesizedType(inner) => ty = &inner.type_annotation,
            TSType::TSTypeOperatorType(operator) if operator.operator == TSTypeOperatorOperator::Readonly => ty = &operator.type_annotation,
            _ => return ty,
        }
    }
}

fn is_never(ty: &TSType<'_>) -> bool {
    matches!(unwrap_transparent_any(ty), TSType::TSNeverKeyword(_))
}

fn is_effectively_empty_member(member: &TSSignature<'_>) -> bool {
    matches!(member, TSSignature::TSPropertySignature(property) if property.optional && property.type_annotation.as_ref().is_some_and(|annotation| is_never(&annotation.type_annotation)))
}

fn is_effectively_empty_literal(literal: &TSTypeLiteral<'_>) -> bool {
    literal.members.iter().all(is_effectively_empty_member)
}

impl<'s, 'a> Types<'s, 'a> {
    fn alias(&self, name: &str) -> Option<&'a TSTypeAliasDeclaration<'a>> {
        self.env.aliases.get(name).and_then(|aliases| self.alias_at(aliases.first))
    }

    fn alias_at(&self, id: NodeId) -> Option<&'a TSTypeAliasDeclaration<'a>> {
        match self.nodes.kind(id) {
            AstKind::TSTypeAliasDeclaration(alias) => Some(alias),
            _ => None,
        }
    }

    /// The alias that wins for `no-unknown-returns` and `no-unknown-type-aliases`.
    pub fn last_alias(&self, name: &str) -> Option<&'a TSTypeAliasDeclaration<'a>> {
        self.env.aliases.get(name).and_then(|aliases| self.alias_at(aliases.last))
    }

    /// The alias without type parameters that wins for `no-object-parameters`.
    pub fn last_plain_alias(&self, name: &str) -> Option<&'a TSTypeAliasDeclaration<'a>> {
        self.env.aliases.get(name).and_then(|aliases| aliases.last_plain).and_then(|id| self.alias_at(id))
    }

    pub fn has_alias(&self, name: &str) -> bool {
        self.env.has_alias(name)
    }

    /// The nodes of the file.
    pub fn nodes(&self) -> &'s AstNodes<'a> {
        self.nodes
    }

    fn is_effectively_empty_interface(&self, ids: &[NodeId]) -> bool {
        let [id] = ids else {
            return false;
        };

        match self.nodes.kind(*id) {
            AstKind::TSInterfaceDeclaration(interface) => interface.extends.is_empty() && interface.body.body.iter().all(is_effectively_empty_member),
            _ => false,
        }
    }

    fn resolved_substitution_argument(ty: &'a TSType<'a>, base: &Substitutions<'a>, resolving: &Resolving<'a>) -> &'a TSType<'a> {
        let TSType::TSTypeReference(reference) = unwrap_transparent(ty) else {
            return ty;
        };

        let Some(name) = reference_name(reference).filter(|name| !resolving.contains(name)) else {
            return ty;
        };

        let Some(substitution) = base.get(name) else {
            return ty;
        };

        let mut next = resolving.clone();

        next.push(name);

        Self::resolved_substitution_argument(substitution, base, &next)
    }

    fn alias_substitution(alias: &'a TSTypeAliasDeclaration<'a>, reference: &'a TSTypeReference<'a>, base: &Substitutions<'a>) -> Option<Substitutions<'a>> {
        let arguments = type_arguments(reference);
        let mut next = base.clone();

        for (index, parameter) in alias.type_parameters.iter().flat_map(|parameters| parameters.params.iter()).enumerate() {
            let argument = arguments.get(index).or(parameter.default.as_ref())?;
            let resolved = Self::resolved_substitution_argument(argument, &next, &Vec::new());

            next.insert(parameter.name.name.as_str(), resolved);
        }

        Some(next)
    }

    fn unsafe_direct_value(&self, ty: &'a TSType<'a>, substitutions: &Substitutions<'a>, resolving: &Resolving<'a>) -> Option<UnsafeValue> {
        let reference = match unwrap_transparent(ty) {
            TSType::TSUnknownKeyword(_) => return Some(UnsafeValue::Unknown),
            TSType::TSAnyKeyword(_) => return Some(UnsafeValue::Any),
            TSType::TSObjectKeyword(_) => return Some(UnsafeValue::Object),
            TSType::TSTypeLiteral(literal) => return is_effectively_empty_literal(literal).then_some(UnsafeValue::EmptyObject),
            TSType::TSUnionType(union) => {
                return union.types.iter().any(|member| self.unsafe_direct_value(member, substitutions, resolving).is_some()).then_some(UnsafeValue::Union);
            }
            TSType::TSIntersectionType(intersection) => {
                let members: Vec<_> = intersection.types.iter().map(|member| self.unsafe_direct_value(member, substitutions, resolving)).collect();

                if members.contains(&Some(UnsafeValue::Any)) {
                    return Some(UnsafeValue::Any);
                }

                return if members.iter().all(Option::is_some) { members.first().copied().flatten() } else { None };
            }
            TSType::TSTypeReference(reference) => reference,
            _ => return None,
        };

        let name = reference_name(reference)?;

        if self.env.is_transparent_wrapper(name) {
            return self.unsafe_direct_value(type_arguments(reference).first()?, substitutions, resolving);
        }

        if let Some(substitution) = substitutions.get(name) {
            return if is_unapplied_reference_to(substitution, name) { None } else { self.unsafe_direct_value(substitution, substitutions, resolving) };
        }

        if let Some(ids) = self.env.interfaces.get(name) {
            return self.is_effectively_empty_interface(ids).then_some(UnsafeValue::EmptyObject);
        }

        let alias = self.alias(name).filter(|_| !resolving.contains(&name))?;
        let next = Self::alias_substitution(alias, reference, substitutions)?;
        let mut next_resolving = resolving.clone();

        next_resolving.push(name);

        self.unsafe_direct_value(&alias.type_annotation, &next, &next_resolving)
    }

    fn dictionary_value_types(&self, ty: &'a TSType<'a>, substitutions: &Substitutions<'a>, resolving: &Resolving<'a>) -> Vec<ValueType<'a>> {
        match unwrap_transparent(ty) {
            TSType::TSTypeLiteral(literal) => literal_value_types(literal, substitutions),
            TSType::TSMappedType(mapped) => mapped_value_types(mapped, substitutions),
            TSType::TSTypeReference(reference) => self.reference_value_types(reference, substitutions, resolving),
            _ => Vec::new(),
        }
    }

    fn reference_value_types(&self, reference: &'a TSTypeReference<'a>, substitutions: &Substitutions<'a>, resolving: &Resolving<'a>) -> Vec<ValueType<'a>> {
        let Some(name) = reference_name(reference) else {
            return Vec::new();
        };

        if let Some(substitution) = substitutions.get(name) {
            return if is_unapplied_reference_to(substitution, name) {
                Vec::new()
            } else {
                self.dictionary_value_types(substitution, substitutions, resolving)
            };
        }

        let arguments = type_arguments(reference);

        if self.env.is_transparent_wrapper(name) {
            return arguments.first().map_or_else(Vec::new, |wrapped| self.dictionary_value_types(wrapped, substitutions, resolving));
        }

        if name == "Record" && self.env.is_built_in(name) {
            return arguments.get(1).map_or_else(Vec::new, |value| vec![(value, substitutions.clone())]);
        }

        if (name == "Pick" || name == "Omit") && self.env.is_built_in(name) {
            return arguments.first().map_or_else(Vec::new, |source| self.dictionary_value_types(source, substitutions, resolving));
        }

        let Some(alias) = self.alias(name).filter(|_| !resolving.contains(&name)) else {
            return Vec::new();
        };

        let Some(next) = Self::alias_substitution(alias, reference, substitutions) else {
            return Vec::new();
        };

        let mut next_resolving = resolving.clone();

        next_resolving.push(name);

        self.dictionary_value_types(&alias.type_annotation, &next, &next_resolving)
    }

    /// The first unsafe value type among the dictionary value types of `values`.
    fn first_unsafe(&self, values: Vec<ValueType<'a>>) -> Option<UnsafeValue> {
        values.into_iter().find_map(|(value, substitutions)| self.unsafe_direct_value(value, &substitutions, &Vec::new()))
    }

    /// How a type used as a dictionary leaks an unsafe value type, if it does.
    pub fn classify_unsafe_dictionary(&self, ty: &'a TSType<'a>) -> Option<UnsafeValue> {
        self.first_unsafe(self.dictionary_value_types(ty, &Substitutions::default(), &Vec::new()))
    }

    /// [`Self::classify_unsafe_dictionary`] for a type node met in the walk.
    /// Kinds that are not types, or never dictionaries, give `None`.
    pub fn classify_unsafe_dictionary_kind(&self, kind: AstKind<'a>) -> Option<UnsafeValue> {
        let empty = Substitutions::default();
        let values = match kind {
            AstKind::TSTypeLiteral(literal) => literal_value_types(literal, &empty),
            AstKind::TSMappedType(mapped) => mapped_value_types(mapped, &empty),
            AstKind::TSTypeReference(reference) => self.reference_value_types(reference, &empty, &Vec::new()),
            AstKind::TSTypeOperator(operator) if operator.operator == TSTypeOperatorOperator::Readonly => {
                return self.classify_unsafe_dictionary(&operator.type_annotation);
            }
            AstKind::TSParenthesizedType(parenthesized) => return self.classify_unsafe_dictionary(&parenthesized.type_annotation),
            _ => return None,
        };

        self.first_unsafe(values)
    }

    /// Whether a dictionary value type is an unsafe escape hatch.
    pub fn classify_unsafe_value(&self, ty: &'a TSType<'a>) -> Option<UnsafeValue> {
        self.unsafe_direct_value(ty, &Substitutions::default(), &Vec::new())
    }

    /// How broad an explicit annotation or assertion target is.
    pub fn classify_widening_target(&self, ty: &'a TSType<'a>) -> Option<WideningTarget> {
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

        if self.env.is_transparent_wrapper(name) {
            return self.classify_widening_target(type_arguments(reference).first()?);
        }

        if name == "Record" && self.env.is_built_in(name) {
            return Some(WideningTarget::OpenDictionary);
        }

        let alias = self.alias(name)?;
        let substitutions = Self::alias_substitution(alias, reference, &Substitutions::default())?;
        let resolving = vec![name];

        if alias.type_parameters.as_ref().is_some_and(|parameters| !parameters.params.is_empty()) {
            return (!self.dictionary_value_types(&alias.type_annotation, &substitutions, &resolving).is_empty()).then_some(WideningTarget::GenericContainer);
        }

        self.classify_alias_broad_target(&alias.type_annotation, &substitutions, &resolving)
    }

    fn is_broad_mapped_key(&self, ty: &'a TSType<'a>, substitutions: &Substitutions<'a>) -> bool {
        let reference = match unwrap_transparent(ty) {
            TSType::TSStringKeyword(_) | TSType::TSNumberKeyword(_) | TSType::TSSymbolKeyword(_) => return true,
            TSType::TSUnionType(union) => return union.types.iter().all(|member| self.is_broad_mapped_key(member, substitutions)),
            TSType::TSTypeReference(reference) => reference,
            _ => return false,
        };

        let Some(name) = reference_name(reference) else {
            return false;
        };

        if let Some(substitution) = substitutions.get(name).filter(|substitution| !is_unapplied_reference_to(substitution, name)) {
            return self.is_broad_mapped_key(substitution, substitutions);
        }

        name == "PropertyKey" && self.env.is_built_in(name)
    }

    fn classify_alias_broad_target(&self, ty: &'a TSType<'a>, substitutions: &Substitutions<'a>, resolving: &Resolving<'a>) -> Option<WideningTarget> {
        let reference = match unwrap_transparent(ty) {
            TSType::TSUnknownKeyword(_) => return Some(WideningTarget::Unknown),
            TSType::TSObjectKeyword(_) => return Some(WideningTarget::Object),
            TSType::TSTypeLiteral(literal) => {
                return literal.members.iter().any(|member| matches!(member, TSSignature::TSIndexSignature(_))).then_some(WideningTarget::OpenDictionary);
            }
            TSType::TSMappedType(mapped) => return self.is_broad_mapped_key(&mapped.constraint, substitutions).then_some(WideningTarget::OpenDictionary),
            TSType::TSTypeReference(reference) => reference,
            _ => return None,
        };

        let name = reference_name(reference)?;

        if let Some(substitution) = substitutions.get(name) {
            return if is_unapplied_reference_to(substitution, name) { None } else { self.classify_alias_broad_target(substitution, substitutions, resolving) };
        }

        if self.env.is_transparent_wrapper(name) {
            return self.classify_alias_broad_target(type_arguments(reference).first()?, substitutions, resolving);
        }

        if name == "Record" && self.env.is_built_in(name) {
            return Some(WideningTarget::OpenDictionary);
        }

        let alias = self.alias(name).filter(|_| !resolving.contains(&name))?;
        let next = Self::alias_substitution(alias, reference, substitutions)?;
        let mut next_resolving = resolving.clone();

        next_resolving.push(name);

        self.classify_alias_broad_target(&alias.type_annotation, &next, &next_resolving)
    }
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
