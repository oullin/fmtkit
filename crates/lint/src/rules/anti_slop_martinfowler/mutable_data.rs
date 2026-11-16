//! `anti-slop-martinfowler/mutable-data`: module-level `let` and `var`
//! bindings that functions reassign, reported once per binding with the number
//! of distinct functions that write it.
//!
//! A write is any assignment, compound assignment, update or destructuring
//! assignment to the binding; its declaration's initialiser is not one.
//! Writes at module level (outside every function) do not count. Exported
//! declarations are left to `global-data`, and `const` bindings are skipped:
//! mutating a `const` object is not reassignment.
//!
//! Options: `{ ignoreTests?: boolean = true }`. Test files (upstream's
//! `isTestFile`) are skipped unless `ignoreTests` is `false`.

use oxc_ast::ast::{Statement, VariableDeclarationKind};
use oxc_semantic::NodeId;
use serde_json::Value;

use super::super::anti_slop::shared::enclosing_function;
use super::super::{Context, Rule};
use super::shared::{Options, binding_span, is_test_file};

pub const NAME: &str = "anti-slop-martinfowler/mutable-data";

pub fn build(options: &[Value]) -> Result<Box<dyn Rule>, String> {
    let options = Options::parse(options, &["ignoreTests"])?;

    Ok(Box::new(MutableData { ignore_tests: options.boolean("ignoreTests", true)? }))
}

struct MutableData {
    ignore_tests: bool,
}

impl Rule for MutableData {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run_once(&self, ctx: &mut Context<'_, '_>) {
        if self.ignore_tests && is_test_file(ctx.rel) {
            return;
        }

        let semantic = ctx.semantic;
        let (nodes, scoping) = (semantic.nodes(), semantic.scoping());

        for statement in &nodes.program().body {
            let Statement::VariableDeclaration(declaration) = statement else {
                continue;
            };

            if !matches!(declaration.kind, VariableDeclarationKind::Let | VariableDeclarationKind::Var) {
                continue;
            }

            for declarator in &declaration.declarations {
                for identifier in declarator.id.get_binding_identifiers() {
                    let Some(symbol) = identifier.symbol_id.get() else {
                        continue;
                    };

                    let mut writers: Vec<NodeId> = Vec::new();

                    for reference in scoping.get_resolved_references(symbol).filter(|reference| reference.is_write()) {
                        if let Some(writer) = enclosing_function(nodes, reference.node_id(), false)
                            && !writers.contains(&writer.id())
                        {
                            writers.push(writer.id());
                        }
                    }

                    if writers.is_empty() {
                        continue;
                    }

                    let name = identifier.name.as_str();
                    let functions = if writers.len() == 1 { "1 function".to_owned() } else { format!("{} functions", writers.len()) };

                    ctx.report(
                        binding_span(semantic, symbol),
                        format!(
                            "Possibly Mutable Data: module-level `{name}` is reassigned inside {functions}, so its value depends on which functions ran before and in what order. Consider Encapsulate Variable so one owner controls every update, or Replace Derived Variable with Query if it can be computed from other data (in Effect, a `Ref` owned by a service, or a value derived where it is needed)."
                        ),
                    );
                }
            }
        }
    }
}
