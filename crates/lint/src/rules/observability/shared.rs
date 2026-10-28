//! Syntax wrappers and token-backed escape comments.

use oxc_ast::AstKind;
use oxc_ast::CommentKind;
use oxc_ast::ast::{ArrowFunctionExpression, CatchClause, Class, Expression, Function, MethodDefinition, ObjectProperty, Statement, TSType};
use oxc_ast_visit::{Visit, walk};
use oxc_semantic::Semantic;
use oxc_span::Span;
use oxc_syntax::scope::ScopeFlags;
use rustc_hash::FxHashSet;

use super::super::Context;
use super::super::anti_slop::shared::is_js_space;

pub fn unwrap<'s, 'a>(mut expression: &'s Expression<'a>) -> &'s Expression<'a> {
    loop {
        expression = match expression {
            Expression::ParenthesizedExpression(inner) => &inner.expression,
            Expression::TSAsExpression(inner) => &inner.expression,
            Expression::TSSatisfiesExpression(inner) => &inner.expression,
            Expression::TSNonNullExpression(inner) => &inner.expression,
            Expression::TSTypeAssertion(inner) => &inner.expression,
            _ => return expression,
        };
    }
}

/// A subtree search whose nested functions, classes and catches own their own
/// handling. Types do not provide value references or handling evidence.
pub struct Contains<F> {
    predicate: F,
    pub found: bool,
}

impl<F> Contains<F> {
    pub fn new(predicate: F) -> Self {
        Self { predicate, found: false }
    }
}

impl<'a, F: FnMut(AstKind<'a>) -> bool> Visit<'a> for Contains<F> {
    fn enter_node(&mut self, kind: AstKind<'a>) {
        self.found |= (self.predicate)(kind);
    }

    fn visit_function(&mut self, _: &Function<'a>, _: ScopeFlags) {}
    fn visit_arrow_function_expression(&mut self, _: &ArrowFunctionExpression<'a>) {}
    fn visit_class(&mut self, _: &Class<'a>) {}
    fn visit_catch_clause(&mut self, _: &CatchClause<'a>) {}
    fn visit_method_definition(&mut self, _: &MethodDefinition<'a>) {}
    fn visit_ts_type(&mut self, _: &TSType<'a>) {}

    fn visit_object_property(&mut self, property: &ObjectProperty<'a>) {
        if !property.method && property.kind == oxc_ast::ast::PropertyKind::Init {
            walk::walk_object_property(self, property);
        }
    }
}

/// String-literal computed keys count; dynamic keys and private fields do not.
pub fn member<'s, 'a>(expression: &'s Expression<'a>) -> Option<(&'s Expression<'a>, &'a str, Span)> {
    match unwrap(expression) {
        Expression::StaticMemberExpression(member) => Some((&member.object, member.property.name.as_str(), member.property.span)),
        Expression::ComputedMemberExpression(member) => match &member.expression {
            Expression::StringLiteral(key) => Some((&member.object, key.value.as_str(), key.span)),
            _ => None,
        },
        _ => None,
    }
}

struct Escapes {
    starts: Vec<usize>,
    lines: FxHashSet<usize>,
}

impl Escapes {
    fn new(semantic: &Semantic<'_>) -> Self {
        let source = semantic.source_text();
        let mut starts = vec![0];
        let mut chars = source.char_indices().peekable();

        while let Some((at, c)) = chars.next() {
            if c == '\r' && chars.peek().is_some_and(|&(_, next)| next == '\n') {
                chars.next();
                starts.push(at + 2);
            } else if matches!(c, '\r' | '\n' | '\u{2028}' | '\u{2029}') {
                starts.push(at + c.len_utf8());
            }
        }

        let mut lines = FxHashSet::default();

        for comment in semantic.comments().iter().filter(|comment| comment.kind == CommentKind::Line) {
            let at = comment.span.start as usize;
            let line = starts.partition_point(|&start| start <= at).saturating_sub(1);
            let prefix = source.get(starts[line]..at).unwrap_or_default();
            let text = source.get(at..comment.span.end as usize).unwrap_or_default();
            let Some(reason) = text.strip_prefix("// observability: swallowed because") else { continue };

            if prefix.chars().all(is_js_space)
                && reason.starts_with([' ', '\t'])
                && reason.trim_start_matches([' ', '\t']).chars().next().is_some_and(|c| !is_js_space(c))
            {
                lines.insert(line + 1);
            }
        }

        Self { starts, lines }
    }
}

pub fn escaped(ctx: &mut Context<'_, '_>, at: u32) -> bool {
    let escapes = ctx.shared(Escapes::new);
    let line = escapes.starts.partition_point(|&start| start <= at as usize).saturating_sub(1);

    escapes.lines.contains(&line)
}

pub fn single_throw(statements: &[Statement<'_>]) -> bool {
    matches!(statements, [Statement::ThrowStatement(_)])
}
