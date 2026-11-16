//! `anti-slop-martinfowler/mysterious-name`: top-level functions named with a
//! bare generic verb (`run`, `handle`, `process`, …) or a generic verb plus a
//! vague noun (`processData`, `doWork`), and `const` bindings that only alias
//! another function.
//!
//! A top-level function is a function declaration, or a variable initialised
//! with a function, an arrow, or `Effect.fn(…)`/`Effect.fnUntraced(…)` (with
//! or without a name argument) around one; exported or not. Nested functions'
//! names are not checked, and `allowNames` are never generic. An alias is a
//! `const` in any scope whose initialiser is just the name of a function
//! declared in scope as a function declaration or a variable holding a
//! function value as above; imports, parameters and other values are not
//! aliases.
//!
//! Options: `{ ignoreTests?: boolean = true, verbs?: string[] = [apply, do,
//! doWork, ensure, execute, handle, init, initialize, manage, perform,
//! prepare, process, resolve, run, sync, update], vagueNouns?: string[] =
//! [All, Data, Info, It, Item, Items, Logic, Stuff, Thing, Things, Value,
//! Values, Work], allowNames?: string[] = [fetch, layer, lookup, make, of],
//! checkAliases?: boolean = true }`. Test files (upstream's `isTestFile`) are
//! skipped unless `ignoreTests` is `false`.

use oxc_ast::AstKind;
use oxc_ast::ast::{
    BindingPattern, Declaration, ExportDefaultDeclarationKind, Expression, Function, FunctionType, Statement, VariableDeclaration, VariableDeclarationKind,
};
use oxc_semantic::{AstNode, Semantic};
use oxc_span::Span;
use serde_json::Value;

use super::super::anti_slop::shared::parent;
use super::super::{Context, Rule};
use super::shared::{Member, Options, declarator_id_span, expression, is_function_node, is_test_file, unwrap};

pub const NAME: &str = "anti-slop-martinfowler/mysterious-name";

const DEFAULT_VERBS: [&str; 16] = [
    "apply",
    "do",
    "doWork",
    "ensure",
    "execute",
    "handle",
    "init",
    "initialize",
    "manage",
    "perform",
    "prepare",
    "process",
    "resolve",
    "run",
    "sync",
    "update",
];

const DEFAULT_VAGUE_NOUNS: [&str; 13] = ["All", "Data", "Info", "It", "Item", "Items", "Logic", "Stuff", "Thing", "Things", "Value", "Values", "Work"];

const DEFAULT_ALLOW_NAMES: [&str; 5] = ["fetch", "layer", "lookup", "make", "of"];

pub fn build(options: &[Value]) -> Result<Box<dyn Rule>, String> {
    let options = Options::parse(options, &["ignoreTests", "verbs", "vagueNouns", "allowNames", "checkAliases"])?;

    Ok(Box::new(MysteriousName {
        ignore_tests: options.boolean("ignoreTests", true)?,
        verbs: options.strings("verbs", &DEFAULT_VERBS)?,
        vague_nouns: options.strings("vagueNouns", &DEFAULT_VAGUE_NOUNS)?,
        allow_names: options.strings("allowNames", &DEFAULT_ALLOW_NAMES)?,
        check_aliases: options.boolean("checkAliases", true)?,
    }))
}

struct MysteriousName {
    ignore_tests: bool,
    verbs: Vec<String>,
    vague_nouns: Vec<String>,
    allow_names: Vec<String>,
    check_aliases: bool,
}

impl MysteriousName {
    fn is_verb(&self, name: &str) -> bool {
        self.verbs.iter().any(|verb| verb == name)
    }

    fn is_generic(&self, name: &str) -> bool {
        if self.allow_names.iter().any(|allowed| allowed == name) {
            return false;
        }

        self.is_verb(name) || self.vague_nouns.iter().any(|noun| name.strip_suffix(noun.as_str()).is_some_and(|verb| self.is_verb(verb)))
    }

    fn check_name(&self, ctx: &mut Context<'_, '_>, name: &str, span: Span) {
        if self.is_generic(name) {
            ctx.report(
                span,
                format!(
                    "Possibly Mysterious Name: `{name}` is a generic verb with no object, so callers cannot tell which steps it performs or what it reads, writes, or calls. Consider Rename Function: name each caller-visible step it performs (for example `resolveConfiguration` → `loadAndReconcileConfigurationSources`)."
                ),
            );
        }
    }

    fn check_function(&self, ctx: &mut Context<'_, '_>, function: &Function<'_>) {
        if function.r#type == FunctionType::FunctionDeclaration
            && function.body.is_some()
            && let Some(id) = &function.id
        {
            self.check_name(ctx, id.name.as_str(), id.span);
        }
    }

    fn check_variables(&self, ctx: &mut Context<'_, '_>, declaration: &VariableDeclaration<'_>) {
        for declarator in &declaration.declarations {
            if let BindingPattern::BindingIdentifier(id) = &declarator.id
                && declarator.init.as_ref().is_some_and(is_function_value)
            {
                self.check_name(ctx, id.name.as_str(), declarator_id_span(declarator));
            }
        }
    }
}

impl Rule for MysteriousName {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run_once(&self, ctx: &mut Context<'_, '_>) {
        if self.ignore_tests && is_test_file(ctx.rel) {
            return;
        }

        for statement in &ctx.semantic.nodes().program().body {
            match statement {
                Statement::ExportDefaultDeclaration(export) => {
                    if let ExportDefaultDeclarationKind::FunctionDeclaration(function) = &export.declaration {
                        self.check_function(ctx, function);
                    }
                }
                Statement::ExportDeclaration(export) => match &export.declaration {
                    Declaration::FunctionDeclaration(function) => self.check_function(ctx, function),
                    Declaration::VariableDeclaration(declaration) => self.check_variables(ctx, declaration),
                    _ => {}
                },
                Statement::FunctionDeclaration(function) => self.check_function(ctx, function),
                Statement::VariableDeclaration(declaration) => self.check_variables(ctx, declaration),
                _ => {}
            }
        }
    }

    // Aliases are checked in every scope: a local `const executeMutation =
    // refreshAuthentication` hides behavior as much as a top-level one.
    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let AstKind::VariableDeclarator(declarator) = node.kind() else {
            return;
        };

        if !self.check_aliases {
            return;
        }

        let is_const = matches!(parent(ctx.semantic.nodes(), node.id()).kind(), AstKind::VariableDeclaration(declaration) if declaration.kind == VariableDeclarationKind::Const);

        let (BindingPattern::BindingIdentifier(id), Some(init), true) = (&declarator.id, &declarator.init, is_const) else {
            return;
        };

        if self.ignore_tests && is_test_file(ctx.rel) {
            return;
        }

        let Some(target) = aliased_function_name(ctx.semantic, init) else {
            return;
        };

        let name = id.name.as_str();

        if target == name {
            return;
        }

        ctx.report(
            declarator.span,
            format!(
                "Possibly Mysterious Name: `{name}` only aliases `{target}`, so one behavior has two names and readers must check whether they differ. Consider Rename Function: delete the alias and call `{target}` directly, or rename the one function so its name states what it does."
            ),
        );
    }
}

/// `Effect.fn("x")(function* () {})`, `Effect.fn(function* () {})`, or a
/// plain function, as upstream's `isFunctionValue`.
fn is_function_value(init: &Expression<'_>) -> bool {
    let value = unwrap(expression(init));

    if is_function_node(value) {
        return true;
    }

    let AstKind::CallExpression(call) = value else {
        return false;
    };

    let has_function_argument =
        call.arguments.iter().any(|argument| argument.as_expression().is_some_and(|argument| is_function_node(unwrap(expression(argument)))));

    if !has_function_argument {
        return false;
    }

    let callee = unwrap(expression(&call.callee));
    let constructor = match callee {
        AstKind::CallExpression(inner) => unwrap(expression(&inner.callee)),
        _ => callee,
    };

    Member::of(constructor).and_then(Member::identifier_name).is_some_and(|name| name == "fn" || name == "fnUntraced")
}

/// The name of the function `init` aliases: a bare identifier bound by a
/// function declaration, or by a variable holding a function value.
fn aliased_function_name<'a>(semantic: &Semantic<'a>, init: &'a Expression<'a>) -> Option<&'a str> {
    let AstKind::IdentifierReference(value) = unwrap(expression(init)) else {
        return None;
    };

    let nodes = semantic.nodes();
    let scoping = semantic.scoping();
    let symbol = scoping.find_binding(nodes.get_node(value.node_id.get()).scope_id(), value.name)?;

    let is_function = match nodes.kind(scoping.symbol_declaration(symbol)) {
        AstKind::Function(function) => function.r#type == FunctionType::FunctionDeclaration && function.body.is_some(),
        AstKind::VariableDeclarator(declarator) => declarator.init.as_ref().is_some_and(is_function_value),
        _ => false,
    };

    is_function.then_some(value.name.as_str())
}
