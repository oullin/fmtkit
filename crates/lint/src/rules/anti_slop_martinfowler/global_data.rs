//! `anti-slop-martinfowler/global-data`: exported module-level state that
//! can change: an exported `let` or `var` (reassigned in this file or not),
//! or an exported object, array, `Map`, `Set`, `WeakMap` or `WeakSet` (a
//! literal, or `new` of one of those constructors) that this file mutates.
//!
//! A binding counts as exported when its declaration is exported, or a local
//! `export { name }` or `export default name` names it. A mutation is a write
//! through any reference: assigning, updating or deleting a property at any
//! depth, calling a mutating method (`push`, `set`, `add`, `delete`, `sort`,
//! …) on it or on something reached through it, or passing it first to
//! `Object.assign`. Destructured declarations and re-exports from another
//! module are skipped, as are `const` values that are not literal containers
//! (`export const config = loadConfig()`).
//!
//! Options: `{ ignoreTests?: boolean = true }`. Test files (upstream's
//! `isTestFile`) are skipped unless `ignoreTests` is `false`.

use oxc_ast::AstKind;
use oxc_ast::ast::{BindingPattern, Declaration, Expression, ModuleExportName, Statement, VariableDeclaration, VariableDeclarationKind, VariableDeclarator};
use oxc_semantic::{AstNode, AstNodes, Semantic};
use oxc_syntax::operator::UnaryOperator;
use serde_json::Value;

use super::super::anti_slop::shared::{parent, strip_parens};
use super::super::{Context, Rule};
use super::shared::{Member, Options, declarator_id_span, expression, is_node, is_test_file, is_wrapper, unwrap};

pub const NAME: &str = "anti-slop-martinfowler/global-data";

const MUTATING_METHODS: [&str; 13] = ["add", "clear", "copyWithin", "delete", "fill", "pop", "push", "reverse", "set", "shift", "sort", "splice", "unshift"];

const CONTAINER_CONSTRUCTORS: [&str; 6] = ["Array", "Map", "Object", "Set", "WeakMap", "WeakSet"];

pub fn build(options: &[Value]) -> Result<Box<dyn Rule>, String> {
    let options = Options::parse(options, &["ignoreTests"])?;

    Ok(Box::new(GlobalData { ignore_tests: options.boolean("ignoreTests", true)? }))
}

struct GlobalData {
    ignore_tests: bool,
}

/// One top-level declarator with an identifier binding: its name, node and
/// declaration kind.
struct Declared<'a> {
    name: &'a str,
    declarator: &'a VariableDeclarator<'a>,
    kind: VariableDeclarationKind,
}

impl Rule for GlobalData {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run_once(&self, ctx: &mut Context<'_, '_>) {
        if self.ignore_tests && is_test_file(ctx.rel) {
            return;
        }

        let semantic = ctx.semantic;
        let mut declared: Vec<Declared<'_>> = Vec::new();
        let mut exported: Vec<usize> = Vec::new();
        let mut exported_names: Vec<&str> = Vec::new();

        for statement in &semantic.nodes().program().body {
            match statement {
                Statement::VariableDeclaration(declaration) => remember_all(declaration, false, &mut declared, &mut exported),
                Statement::ExportDeclaration(export) => {
                    if let Declaration::VariableDeclaration(declaration) = &export.declaration {
                        remember_all(declaration, true, &mut declared, &mut exported);
                    }
                }
                Statement::ExportNamedDeclaration(export) => {
                    for specifier in &export.specifiers {
                        match &specifier.local {
                            ModuleExportName::IdentifierReference(local) => exported_names.push(local.name.as_str()),
                            ModuleExportName::IdentifierName(local) => exported_names.push(local.name.as_str()),
                            ModuleExportName::StringLiteral(_) => {}
                        }
                    }
                }
                Statement::ExportDefaultDeclaration(export) => {
                    if let Some(Expression::Identifier(identifier)) = export.declaration.as_expression().map(strip_parens) {
                        exported_names.push(identifier.name.as_str());
                    }
                }
                _ => {}
            }
        }

        for name in exported_names {
            // The last declarator of a name wins, as upstream's `Map`.
            if let Some(index) = declared.iter().rposition(|entry| entry.name == name)
                && !exported.contains(&index)
            {
                exported.push(index);
            }
        }

        for index in exported {
            let Some(entry) = declared.get(index) else {
                continue;
            };

            let Some(how) = how_it_changes(semantic, entry) else {
                continue;
            };

            let name = entry.name;

            ctx.report(
                declarator_id_span(entry.declarator),
                format!(
                    "Possibly Global Data: exported `{name}` is module-level state that {how}, so any importer can read it while code anywhere changes it. Consider Encapsulate Variable: keep the state private and expose functions that own every read and write (in Effect, a `Ref` created in a service's layer and reached only through its operations)."
                ),
            );
        }
    }
}

fn remember_all<'a>(declaration: &'a VariableDeclaration<'a>, is_exported: bool, declared: &mut Vec<Declared<'a>>, exported: &mut Vec<usize>) {
    for declarator in &declaration.declarations {
        let BindingPattern::BindingIdentifier(id) = &declarator.id else {
            continue;
        };

        declared.push(Declared { name: id.name.as_str(), declarator, kind: declaration.kind });

        if is_exported {
            exported.push(declared.len() - 1);
        }
    }
}

/// What makes an exported binding changeable state, if anything.
fn how_it_changes(semantic: &Semantic<'_>, entry: &Declared<'_>) -> Option<String> {
    let BindingPattern::BindingIdentifier(id) = &entry.declarator.id else {
        return None;
    };

    let symbol = id.symbol_id.get()?;
    let scoping = semantic.scoping();
    let nodes = semantic.nodes();

    let kind = match entry.kind {
        VariableDeclarationKind::Let => Some("let"),
        VariableDeclarationKind::Var => Some("var"),
        _ => None,
    };

    if let Some(kind) = kind {
        let reassigned = scoping.get_resolved_references(symbol).any(oxc_semantic::Reference::is_write);

        return Some(if reassigned {
            format!("is declared with `{kind}` and reassigned in this file")
        } else {
            format!("is declared with `{kind}`, so it can be reassigned")
        });
    }

    let is_container = entry.declarator.init.as_ref().is_some_and(is_mutable_container);
    let mutated =
        is_container && scoping.get_resolved_references(symbol).any(|reference| mutates_through_reference(nodes, nodes.get_node(reference.node_id())));

    mutated.then(|| "is mutated in this file".to_owned())
}

fn is_mutable_container(init: &Expression<'_>) -> bool {
    match unwrap(expression(init)) {
        AstKind::ObjectExpression(_) | AstKind::ArrayExpression(_) => true,
        AstKind::NewExpression(new) => {
            matches!(strip_parens(&new.callee), Expression::Identifier(callee) if CONTAINER_CONSTRUCTORS.contains(&callee.name.as_str()))
        }
        _ => false,
    }
}

/// `node`, climbed past the wrappers around it.
fn climb_wrappers<'s, 'a>(nodes: &'s AstNodes<'a>, mut node: &'s AstNode<'a>) -> &'s AstNode<'a> {
    loop {
        let parent_node = parent(nodes, node.id());

        if parent_node.id() == node.id() || !is_wrapper(parent_node.kind()) {
            return node;
        }

        node = parent_node;
    }
}

/// Whether a reference writes into the referenced object or anything
/// reachable through it, as upstream's `mutatesThroughReference`.
fn mutates_through_reference<'a>(nodes: &AstNodes<'a>, identifier: &AstNode<'a>) -> bool {
    let mut current = climb_wrappers(nodes, identifier);

    if let AstKind::CallExpression(call) = parent(nodes, current.id()).kind()
        && call.arguments.first().and_then(|argument| argument.as_expression()).is_some_and(|argument| is_node(argument, current.id()))
        && let Expression::StaticMemberExpression(callee) = strip_parens(&call.callee)
        && matches!(strip_parens(&callee.object), Expression::Identifier(object) if object.name == "Object")
        && callee.property.name == "assign"
    {
        return true;
    }

    loop {
        let member_node = parent(nodes, current.id());

        let Some(member) = Member::of(member_node.kind()).filter(|member| member_node.id() != current.id() && is_node(member.object, current.id())) else {
            return false;
        };

        let outer = climb_wrappers(nodes, member_node);
        let parent_node = parent(nodes, outer.id());

        if parent_node.id() == outer.id() {
            return false;
        }

        match parent_node.kind() {
            AstKind::AssignmentExpression(assignment) if assignment.left.node_id() == outer.id() => return true,
            AstKind::UpdateExpression(_) => return true,
            AstKind::UnaryExpression(unary) if unary.operator == UnaryOperator::Delete => return true,
            AstKind::CallExpression(call)
                if is_node(&call.callee, outer.id()) && member.identifier_name().is_some_and(|name| MUTATING_METHODS.contains(&name)) =>
            {
                return true;
            }
            _ => {}
        }

        current = outer;
    }
}
