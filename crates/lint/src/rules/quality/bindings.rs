//! Lexically scoped type bindings and alias resolution, ported from the
//! plugin's `shared/type-alias-resolution.js` and
//! `shared/lexical-type-parameters.js`.
//!
//! Unlike anti-slop's module-level [`TypeEnvironment`], a name resolves to the
//! declaration in the nearest enclosing block, namespace, static block or
//! `switch` that declares it, and a type parameter, mapped-type key or `infer`
//! binder on the way there hides it. Two declarations of one name in one
//! scope (merged interfaces, a class and an interface) resolve to nothing.
//!
//! [`TypeEnvironment`]: super::super::anti_slop::shared::types::TypeEnvironment

use std::rc::Rc;

use oxc_ast::AstKind;
use oxc_ast::ast::{TSInferType, TSType, TSTypeAliasDeclaration, TSTypeName, TSTypeReference};
use oxc_ast_visit::{Visit, walk};
use oxc_semantic::{AstNodes, NodeId};
use oxc_span::{GetSpan, Span};
use rustc_hash::FxHashMap;

use super::super::Context;
use super::super::anti_slop::shared::lexical::type_parameters;
use super::super::anti_slop::shared::types::{reference_name, type_arguments};

/// Every type declaration of one file, by name and by the scope that holds it.
#[derive(Debug, Default)]
pub struct TypeBindings {
    /// Name to `(scope, declaration)`, where a declaration of `None` means the
    /// scope declares the name more than once.
    by_name: FxHashMap<String, Vec<(NodeId, Option<NodeId>)>>,
}

/// What a type name means where it is used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Binding {
    /// No declaration and no type parameter: a global or a built-in.
    Unbound,
    /// A type parameter, or a name declared more than once in its scope.
    Hidden,
    /// The one declaration in the nearest scope that has the name.
    Declared(NodeId),
}

impl TypeBindings {
    /// The bindings of the file `ctx` is linting, built once per file.
    pub fn of(ctx: &mut Context<'_, '_>) -> Rc<Self> {
        ctx.shared(|semantic| Self::new(semantic.nodes()))
    }

    fn new(nodes: &AstNodes<'_>) -> Self {
        let mut bindings = Self::default();

        for node in nodes.iter() {
            let name = match node.kind() {
                AstKind::Class(class) => class.id.as_ref().map(|id| id.name.as_str()),
                AstKind::ImportSpecifier(it) => Some(it.local.name.as_str()),
                AstKind::ImportDefaultSpecifier(it) => Some(it.local.name.as_str()),
                AstKind::ImportNamespaceSpecifier(it) => Some(it.local.name.as_str()),
                AstKind::TSEnumDeclaration(it) => Some(it.id.name.as_str()),
                AstKind::TSInterfaceDeclaration(it) => Some(it.id.name.as_str()),
                AstKind::TSTypeAliasDeclaration(it) => Some(it.id.name.as_str()),
                _ => None,
            };

            let Some(name) = name else {
                continue;
            };

            let scope = enclosing_type_scope(nodes, node.id());
            let scopes = bindings.by_name.entry(name.to_owned()).or_default();

            if let Some(entry) = scopes.iter_mut().find(|(id, _)| *id == scope) {
                entry.1 = None;
            } else {
                scopes.push((scope, Some(node.id())));
            }
        }

        bindings
    }

    /// Bind the declarations to the nodes of their file.
    pub fn with<'s, 'a>(&'s self, nodes: &'s AstNodes<'a>) -> Scoped<'s, 'a> {
        Scoped { bindings: self, nodes }
    }
}

/// The nearest ancestor that holds block-scoped declarations, as ESTree
/// spells them: the program, a block or function body, a namespace body, a
/// static block or a `switch`.
fn enclosing_type_scope(nodes: &AstNodes<'_>, id: NodeId) -> NodeId {
    nodes
        .ancestors(id)
        .find(|node| {
            matches!(
                node.kind(),
                AstKind::Program(_)
                    | AstKind::BlockStatement(_)
                    | AstKind::FunctionBody(_)
                    | AstKind::TSModuleBlock(_)
                    | AstKind::StaticBlock(_)
                    | AstKind::SwitchStatement(_)
            )
        })
        .map_or(id, oxc_semantic::AstNode::id)
}

/// [`TypeBindings`] together with the nodes their ids point into.
#[derive(Clone, Copy)]
pub struct Scoped<'s, 'a> {
    bindings: &'s TypeBindings,
    nodes: &'s AstNodes<'a>,
}

impl<'s, 'a> Scoped<'s, 'a> {
    pub fn nodes(&self) -> &'s AstNodes<'a> {
        self.nodes
    }

    /// What `name` means at the node `at`, as the plugin's `visibleTypeBinding`.
    pub fn binding(&self, name: &str, at: NodeId) -> Binding {
        let scopes = self.bindings.by_name.get(name);
        let at_span = self.nodes.kind(at).span();

        for node in std::iter::once(self.nodes.get_node(at)).chain(self.nodes.ancestors(at)) {
            if let Some((_, declaration)) = scopes.and_then(|scopes| scopes.iter().find(|(scope, _)| *scope == node.id())) {
                return declaration.map_or(Binding::Hidden, Binding::Declared);
            }

            if binds_type_parameter(node.kind(), at_span, name) {
                return Binding::Hidden;
            }
        }

        Binding::Unbound
    }

    /// The alias `name` resolves to at `at`, as the plugin's `visibleTypeAlias`.
    pub fn alias(&self, name: &str, at: NodeId) -> Option<&'a TSTypeAliasDeclaration<'a>> {
        if !self.bindings.by_name.contains_key(name) {
            return None;
        }

        match self.binding(name, at) {
            Binding::Declared(id) => match self.nodes.kind(id) {
                AstKind::TSTypeAliasDeclaration(alias) => Some(alias),
                _ => None,
            },
            _ => None,
        }
    }

    /// Whether `ty`, after resolving the aliases visible where they are used
    /// and substituting their type parameters, matches `top`, as the plugin's
    /// `resolvedTypeMatches`.
    pub fn resolves_to(&self, ty: &'a TSType<'a>, top: Top) -> bool {
        Resolver { scoped: *self, top, resolving: Vec::new() }.evaluate(ty, &Rc::new(Substitutions::default()))
    }
}

/// Whether `kind` binds the type parameter `name` for a use at `at`: its own
/// type parameters, a mapped type's key in its name and value types, and an
/// `infer` binder in a conditional type's true branch.
fn binds_type_parameter(kind: AstKind<'_>, at: Span, name: &str) -> bool {
    if type_parameters(kind).is_some_and(|parameters| parameters.params.iter().any(|parameter| parameter.name.name == name)) {
        return true;
    }

    match kind {
        AstKind::TSMappedType(mapped) => {
            mapped.key.name == name
                && [mapped.name_type.as_ref(), mapped.type_annotation.as_ref()].into_iter().flatten().any(|ty| ty.span().contains_inclusive(at))
        }
        AstKind::TSConditionalType(conditional) => conditional.true_type.span().contains_inclusive(at) && infers(&conditional.extends_type, name),
        _ => false,
    }
}

/// Whether `ty` holds an `infer name` anywhere.
fn infers(ty: &TSType<'_>, name: &str) -> bool {
    struct Infers<'n> {
        name: &'n str,
        found: bool,
    }

    impl<'a> Visit<'a> for Infers<'_> {
        fn visit_ts_infer_type(&mut self, it: &TSInferType<'a>) {
            self.found |= it.type_parameter.name.name == self.name;
            walk::walk_ts_infer_type(self, it);
        }
    }

    let mut visitor = Infers { name, found: false };

    visitor.visit_ts_type(ty);
    visitor.found
}

/// The type a type parameter stands for, with the substitutions in force
/// where that type was written.
#[derive(Clone)]
struct Substitution<'a> {
    ty: &'a TSType<'a>,
    scope: Rc<Substitutions<'a>>,
}

type Substitutions<'a> = FxHashMap<&'a str, Substitution<'a>>;

/// What [`Scoped::resolves_to`] looks for once aliases are resolved: the
/// keyword itself, through parentheses and union members.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Top {
    Object,
    Unknown,
    /// `unknown`, or a `Promise` or `PromiseLike` of it.
    UnknownOrPromise,
}

struct Resolver<'s, 'a> {
    scoped: Scoped<'s, 'a>,
    top: Top,
    /// The aliases being resolved, to stop at cycles.
    resolving: Vec<NodeId>,
}

impl<'a> Resolver<'_, 'a> {
    fn evaluate(&mut self, ty: &'a TSType<'a>, substitutions: &Rc<Substitutions<'a>>) -> bool {
        if let TSType::TSTypeReference(reference) = ty
            && let Some(name) = reference_name(reference)
        {
            if let Some(substitution) = substitutions.get(name).filter(|_| type_arguments(reference).is_empty()) {
                let substitution = substitution.clone();

                return self.evaluate(substitution.ty, &substitution.scope);
            }

            if let Some(alias) = self.scoped.alias(name, reference.node_id.get())
                && !self.resolving.contains(&alias.node_id.get())
                && let Some(next) = alias_substitutions(alias, reference, substitutions)
            {
                self.resolving.push(alias.node_id.get());

                let resolved = self.evaluate(&alias.type_annotation, &Rc::new(next));

                self.resolving.pop();

                return resolved;
            }
        }

        self.matches(ty, substitutions)
    }

    fn matches(&mut self, ty: &'a TSType<'a>, substitutions: &Rc<Substitutions<'a>>) -> bool {
        match ty {
            TSType::TSObjectKeyword(_) => self.top == Top::Object,
            TSType::TSUnknownKeyword(_) => self.top != Top::Object,
            TSType::TSParenthesizedType(inner) => self.evaluate(&inner.type_annotation, substitutions),
            TSType::TSUnionType(union) => union.types.iter().any(|member| self.evaluate(member, substitutions)),
            TSType::TSTypeReference(reference) if self.top == Top::UnknownOrPromise && is_promise(reference) => {
                type_arguments(reference).first().is_some_and(|value| self.evaluate(value, substitutions))
            }
            _ => false,
        }
    }
}

fn is_promise(reference: &TSTypeReference<'_>) -> bool {
    matches!(&reference.type_name, TSTypeName::IdentifierReference(name) if name.name == "Promise" || name.name == "PromiseLike")
}

/// The substitutions inside `alias` for a `reference` to it: the outer ones,
/// plus each type parameter bound to its argument (resolved where the
/// reference is) or else its default (resolved inside the alias). `None`
/// when a parameter has neither.
fn alias_substitutions<'a>(
    alias: &'a TSTypeAliasDeclaration<'a>,
    reference: &'a TSTypeReference<'a>,
    base: &Rc<Substitutions<'a>>,
) -> Option<Substitutions<'a>> {
    let arguments = type_arguments(reference);
    let mut next = (**base).clone();

    for (index, parameter) in alias.type_parameters.iter().flat_map(|parameters| parameters.params.iter()).enumerate() {
        let explicit = arguments.get(index);
        let ty = explicit.or(parameter.default.as_ref())?;
        let scope = if explicit.is_some() { Rc::clone(base) } else { Rc::new(next.clone()) };

        next.insert(parameter.name.name.as_str(), Substitution { ty, scope });
    }

    Some(next)
}
