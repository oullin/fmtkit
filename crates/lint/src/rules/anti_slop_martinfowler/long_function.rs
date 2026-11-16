//! `anti-slop-martinfowler/long-function`: functions whose own body has more
//! than `maxStatements` statements (default 30) or spans more than `maxLines`
//! lines (default 80).
//!
//! A function's own size leaves out what nested functions hold: their
//! statements are not counted, and every line of a nested function but its
//! first is taken off the function's line span. Statements are counted as
//! upstream does on ESTree: every node whose type ends in `Statement` (but
//! blocks and empty statements), directives, variable declarations (`for`
//! heads too), function and class declarations. Lines are counted from the
//! function's first to its last line, blank lines and comments included.
//! Arrows with an expression body and bodyless signatures are skipped.
//!
//! Options: `{ ignoreTests?: boolean = true, maxStatements?: integer >= 1 = 30,
//! maxLines?: integer >= 1 = 80 }`. Test files (upstream's `isTestFile`) are
//! skipped unless `ignoreTests` is `false`.

use oxc_ast::AstKind;
use oxc_ast::ast::{ArrowFunctionBody, ArrowFunctionExpression, Class, Function, FunctionType};
use oxc_ast_visit::{Visit, walk};
use oxc_semantic::AstNode;
use oxc_span::GetSpan;
use oxc_syntax::scope::ScopeFlags;
use serde_json::Value;

use super::super::{Context, Rule};
use super::shared::{Options, describe_function, function_name, function_report_span, is_function_node, is_test_file, line_span};

pub const NAME: &str = "anti-slop-martinfowler/long-function";

pub fn build(options: &[Value]) -> Result<Box<dyn Rule>, String> {
    let options = Options::parse(options, &["ignoreTests", "maxStatements", "maxLines"])?;

    Ok(Box::new(LongFunction {
        ignore_tests: options.boolean("ignoreTests", true)?,
        max_statements: options.integer("maxStatements", 1, 30)?,
        max_lines: options.integer("maxLines", 1, 80)?,
    }))
}

struct LongFunction {
    ignore_tests: bool,
    max_statements: i64,
    max_lines: i64,
}

impl Rule for LongFunction {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let body = match node.kind() {
            AstKind::Function(function) if is_function_node(node.kind()) => function.body.as_deref(),
            AstKind::ArrowFunctionExpression(arrow) => match &arrow.body {
                ArrowFunctionBody::FunctionBody(body) => Some(&**body),
                _ => None,
            },
            _ => None,
        };

        let Some(body) = body else {
            return;
        };

        if self.ignore_tests && is_test_file(ctx.rel) {
            return;
        }

        let source = ctx.source();
        let mut measure = Measure { source, statements: 0, nested_lines: 0 };

        measure.visit_function_body(body);

        let lines = line_span(source, node.kind().span()).saturating_sub(measure.nested_lines);
        let statements = measure.statements;
        let (max_statements, max_lines) = (self.max_statements, self.max_lines);

        if i64::try_from(statements).is_ok_and(|statements| statements <= max_statements) && i64::try_from(lines).is_ok_and(|lines| lines <= max_lines) {
            return;
        }

        let semantic = ctx.semantic;
        let subject = describe_function(function_name(semantic, node).as_deref());

        ctx.report(
            function_report_span(semantic.nodes(), node),
            format!(
                "Possibly Long Function: {subject} has {statements} statements over {lines} lines of its own (limits {max_statements} statements, {max_lines} lines), so a reader must hold every step in mind at once. Consider Extract Function: move each step into a function named for what it does (in Effect, a named `Effect.fn(\"domain.operation\")` step that this function yields)."
            ),
        );
    }
}

/// Statements and nested-function lines inside a body, stopping at nested
/// function boundaries, as upstream's `measure`.
struct Measure<'s> {
    source: &'s str,
    statements: usize,
    nested_lines: usize,
}

impl<'a> Visit<'a> for Measure<'_> {
    fn enter_node(&mut self, kind: AstKind<'a>) {
        let counted = matches!(
            kind,
            AstKind::ExpressionStatement(_)
                | AstKind::Directive(_)
                | AstKind::IfStatement(_)
                | AstKind::ForStatement(_)
                | AstKind::ForInStatement(_)
                | AstKind::ForOfStatement(_)
                | AstKind::WhileStatement(_)
                | AstKind::DoWhileStatement(_)
                | AstKind::ReturnStatement(_)
                | AstKind::BreakStatement(_)
                | AstKind::ContinueStatement(_)
                | AstKind::ThrowStatement(_)
                | AstKind::TryStatement(_)
                | AstKind::SwitchStatement(_)
                | AstKind::LabeledStatement(_)
                | AstKind::DebuggerStatement(_)
                | AstKind::WithStatement(_)
                | AstKind::VariableDeclaration(_)
        );

        if counted {
            self.statements += 1;
        }
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        if it.is_declaration() {
            self.statements += 1;
        }

        walk::walk_class(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        let is_function_node = matches!(it.r#type, FunctionType::FunctionDeclaration | FunctionType::FunctionExpression) && it.body.is_some();

        if !is_function_node {
            walk::walk_function(self, it, flags);
            return;
        }

        if it.r#type == FunctionType::FunctionDeclaration {
            self.statements += 1;
        }

        self.nested_lines += line_span(self.source, it.span) - 1;
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        self.nested_lines += line_span(self.source, it.span) - 1;
    }
}
