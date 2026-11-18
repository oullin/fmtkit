//! `naming/max-name-words`: a declared name with more words than `max`.
//!
//! Words are counted the way `naming/no-redundant-name-prefix` splits names:
//! kebab-case, snake_case, camelCase and PascalCase all split, and an acronym
//! run ends before the word it leads, so `HTTPServer` is two words. A long
//! name usually carries context that a module, namespace or class could carry
//! instead.
//!
//! A name is judged where it is declared, never where it is used: functions,
//! classes, interfaces, type aliases, enums and their members, namespaces,
//! variables and constants at any depth, parameters, class methods and
//! properties, and the file name without its extensions.
//!
//! Names the author does not choose freely are left alone: imported
//! bindings, shorthand destructuring (`const { bindingName } = x`, whose name
//! the property dictates), object-literal keys, interface and type-literal
//! members (the shape of data), type parameters, members of a class that
//! `extends` or `implements` something (the rule has no types, so it cannot
//! tell which members the parent requires), ambient declarations (`declare`,
//! `declare module`, `declare global`) and declaration files. A file name is
//! judged only for scripts: a Vue component's name follows Vue's own
//! multi-word convention.
//!
//! Options: `{ max?: integer >= 1 = 2, allow?: [pattern], exempt?: [glob],
//! ignoreTests?: boolean = true }`.

use oxc_ast::AstKind;
use oxc_ast::ast::{Class, PropertyKey, TSEnumMemberName};
use oxc_semantic::{AstNode, AstNodes};
use oxc_span::Span;
use rustc_hash::FxHashSet;
use serde_json::Value;

use super::super::anti_slop_martinfowler::shared::{Options, is_test_file};
use super::super::boundaries::Paths;
use super::super::{Context, Rule};
use super::words;

pub const NAME: &str = "naming/max-name-words";

/// The script extensions whose file names are judged.
const SCRIPTS: [&str; 8] = ["ts", "tsx", "mts", "cts", "js", "jsx", "mjs", "cjs"];

pub fn build(options: &[Value]) -> Result<Box<dyn Rule>, String> {
    let parsed = Options::parse(options, &["max", "allow", "exempt", "ignoreTests"])?;
    let allow = parsed.strings("allow", &[])?;
    let exempt = parsed.strings("exempt", &[])?;

    if allow.iter().chain(&exempt).any(|item| item.trim().is_empty()) {
        return Err("allow and exempt must be arrays of non-blank strings".into());
    }

    Ok(Box::new(MaxNameWords {
        max: usize::try_from(parsed.integer("max", 1, 2)?).map_err(|_| "\"max\" is too large".to_owned())?,
        allow,
        exempt: Paths::parse("exempt", &exempt)?,
        ignore_tests: parsed.boolean("ignoreTests", true)?,
    }))
}

struct MaxNameWords {
    max: usize,
    /// Names kept on purpose, exactly or with `*` for any run of characters.
    allow: Vec<String>,
    exempt: Paths,
    ignore_tests: bool,
}

impl Rule for MaxNameWords {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run_once(&self, ctx: &mut Context<'_, '_>) {
        let rel = ctx.rel;

        if self.exempt.matches(rel) || declaration_file(rel) || (self.ignore_tests && is_test_file(rel)) {
            return;
        }

        let file = rel.rsplit('/').next().unwrap_or(rel);

        if file.rsplit_once('.').is_some_and(|(_, extension)| SCRIPTS.contains(&extension)) {
            let stem = &file[..file.find('.').unwrap_or(file.len())];

            if let Some(count) = self.excess(stem) {
                ctx.report(Span::new(0, 0), message(&format!("file name {file}"), count, self.max));
            }
        }

        let nodes = ctx.semantic.nodes();
        let mut judged_symbols = FxHashSet::default();
        // Overloads and accessor pairs declare one member more than once.
        let mut judged_members = FxHashSet::default();
        let mut found = Vec::new();

        for node in nodes.iter() {
            let (name, span) = match node.kind() {
                AstKind::BindingIdentifier(identifier) => {
                    if !chosen(nodes, node) || identifier.symbol_id.get().is_some_and(|symbol| !judged_symbols.insert(symbol)) {
                        continue;
                    }

                    (identifier.name.as_str(), identifier.span)
                }
                AstKind::MethodDefinition(method) if !method.computed => match member(nodes, node, &method.key, &mut judged_members) {
                    Some(found) => found,
                    None => continue,
                },
                AstKind::PropertyDefinition(property) if !property.computed && !property.declare => {
                    match member(nodes, node, &property.key, &mut judged_members) {
                        Some(found) => found,
                        None => continue,
                    }
                }
                AstKind::AccessorProperty(property) if !property.computed => match member(nodes, node, &property.key, &mut judged_members) {
                    Some(found) => found,
                    None => continue,
                },
                AstKind::TSEnumMember(enum_member) => match &enum_member.id {
                    TSEnumMemberName::Identifier(identifier) if !ambient(nodes, node) => (identifier.name.as_str(), identifier.span),
                    _ => continue,
                },
                _ => continue,
            };

            if let Some(count) = self.excess(name) {
                found.push((span, message(&format!("\"{name}\""), count, self.max)));
            }
        }

        for (span, message) in found {
            ctx.report(span, message);
        }
    }
}

impl MaxNameWords {
    /// The word count of `name` when it is over `max` and not allowed.
    fn excess(&self, name: &str) -> Option<usize> {
        let count = words::split(name).len();

        (count > self.max && !self.allow.iter().any(|pattern| words::wildcard(pattern, name))).then_some(count)
    }
}

fn message(what: &str, count: usize, max: usize) -> String {
    format!(
        "{what} has {count} words, more than {max}; use a shorter name, or an abstraction that carries the context: a module, namespace or class whose name makes the long name unnecessary"
    )
}

/// Whether `rel` is a declaration file: `.d.ts`, `.d.mts` or `.d.cts`.
fn declaration_file(rel: &str) -> bool {
    [".d.ts", ".d.mts", ".d.cts"].iter().any(|suffix| rel.ends_with(suffix))
}

/// Whether the binding at `node` declares a name its author chose: not an
/// import, a type parameter, a shorthand destructuring or anything ambient.
fn chosen(nodes: &AstNodes<'_>, node: &AstNode<'_>) -> bool {
    let mut parents = nodes.ancestors(node.id());
    let Some(parent) = parents.next() else {
        return false;
    };

    let dictated = match parent.kind() {
        AstKind::ImportSpecifier(_)
        | AstKind::ImportDefaultSpecifier(_)
        | AstKind::ImportNamespaceSpecifier(_)
        | AstKind::TSImportEqualsDeclaration(_)
        | AstKind::TSTypeParameter(_) => true,
        AstKind::BindingProperty(property) => property.shorthand,
        // `const { name = fallback } = x` is shorthand too.
        AstKind::AssignmentPattern(_) => parents.next().is_some_and(|grand| matches!(grand.kind(), AstKind::BindingProperty(property) if property.shorthand)),
        // A parameter property of a class whose parent may require it.
        AstKind::FormalParameter(parameter) => {
            (parameter.accessibility.is_some() || parameter.readonly || parameter.r#override) && enclosing_class(nodes, parent).is_some_and(inherits)
        }
        _ => false,
    };

    !dictated && !ambient(nodes, node)
}

/// A class member's name and span, when the member is judged: its key is an
/// identifier or a private name, its class inherits nothing, and it is the
/// first declaration of that name in its class.
fn member<'a>(nodes: &AstNodes<'a>, node: &AstNode<'a>, key: &PropertyKey<'a>, judged: &mut FxHashSet<(u32, &'a str)>) -> Option<(&'a str, Span)> {
    let (name, span) = match key {
        PropertyKey::StaticIdentifier(identifier) => (identifier.name.as_str(), identifier.span),
        PropertyKey::PrivateIdentifier(identifier) => (identifier.name.as_str(), identifier.span),
        _ => return None,
    };
    let class = enclosing_class(nodes, node)?;

    (!inherits(class) && !ambient(nodes, node) && judged.insert((class.span.start, name))).then_some((name, span))
}

/// The nearest class around `node`.
fn enclosing_class<'a>(nodes: &AstNodes<'a>, node: &AstNode<'a>) -> Option<&'a Class<'a>> {
    nodes.ancestors(node.id()).find_map(|ancestor| match ancestor.kind() {
        AstKind::Class(class) => Some(class),
        _ => None,
    })
}

/// Whether `class` extends or implements something.
fn inherits(class: &Class<'_>) -> bool {
    class.heritage.is_some() || !class.implements.is_empty()
}

/// Whether `node` is inside an ambient declaration: anything marked
/// `declare`, a `declare module` or `module "name"` block, or `declare global`.
fn ambient(nodes: &AstNodes<'_>, node: &AstNode<'_>) -> bool {
    nodes.ancestors(node.id()).any(|ancestor| match ancestor.kind() {
        AstKind::Function(function) => function.declare,
        AstKind::Class(class) => class.declare,
        AstKind::VariableDeclaration(declaration) => declaration.declare,
        AstKind::TSInterfaceDeclaration(interface) => interface.declare,
        AstKind::TSTypeAliasDeclaration(alias) => alias.declare,
        AstKind::TSEnumDeclaration(declaration) => declaration.declare,
        AstKind::TSNamespaceDeclaration(namespace) => namespace.declare,
        AstKind::TSExternalModuleDeclaration(_) | AstKind::TSGlobalDeclaration(_) => true,
        _ => false,
    })
}
