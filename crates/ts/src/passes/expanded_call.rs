//! Put each argument of a plain call on its own line when one of them is a
//! call, an object, or an array.

use fmtkit_core::{Edit, EditSet};
use oxc_ast::AstKind;
use oxc_ast::ast::{Argument, CallExpression, Comment, Expression, Program};
use oxc_ast_visit::Visit;
use oxc_span::{GetSpan, Span};

use crate::syntax::{Chained, call_parens, has_comment_between, indent_unit, line_indent, unwrap_expression};

/// One rewrite of the argument parentheses per expandable call; a call inside
/// another rewritten call is formatted by the outer rewrite.
pub(crate) fn edits<'a>(text: &'a str, program: &'a Program<'a>) -> EditSet {
    let mut collector = Collector { ancestors: Vec::new(), calls: Vec::new(), templates: Vec::new() };

    collector.visit_program(program);

    let writer = Writer { text, unit: indent_unit(text), comments: &program.comments, templates: &collector.templates };
    let mut edits = EditSet::new();

    for call in collector.calls {
        let Some((open, close)) = writer.parens(call) else { continue };

        if has_comment_between(writer.comments, open, close) {
            continue;
        }

        let indent = line_indent(text, call.span.start);
        let Some(replacement) = writer.call_parens(call, indent) else { continue };

        if replacement != text[open as usize..=close as usize] {
            edits.push(Edit::new(open, close + 1, replacement));
        }
    }

    edits.retain_non_overlapping();

    edits
}

fn is_method_call(call: &CallExpression<'_>) -> bool {
    matches!(unwrap_expression(&call.callee), Chained::Member(_))
}

fn is_complex_argument(argument: &Argument<'_>) -> bool {
    let Some(expression) = argument.as_expression() else {
        return false;
    };

    matches!(unwrap_expression(expression), Chained::Call(_) | Chained::Other(Expression::ObjectExpression(_) | Expression::ArrayExpression(_)))
}

fn should_expand(call: &CallExpression<'_>) -> bool {
    !is_method_call(call) && call.arguments.iter().any(is_complex_argument)
}

/// Collects candidate calls and template-literal spans in one traversal.
struct Collector<'a> {
    ancestors: Vec<AstKind<'a>>,
    calls: Vec<&'a CallExpression<'a>>,
    templates: Vec<Span>,
}

impl<'a> Collector<'a> {
    /// A call inside an argument of the nearest enclosing call, when that call
    /// is not expanded itself, keeps its layout. Functions end the search.
    fn nested_in_unexpanded_argument(&self, call: &CallExpression<'_>) -> bool {
        for ancestor in self.ancestors.iter().rev() {
            match ancestor {
                AstKind::Function(_) | AstKind::ArrowFunctionExpression(_) => return false,
                AstKind::CallExpression(outer) => {
                    let inside = outer.arguments.iter().any(|argument| {
                        let span = argument.span();

                        span.start <= call.span.start && call.span.end <= span.end
                    });

                    return inside && !should_expand(outer);
                }
                _ => {}
            }
        }

        false
    }
}

impl<'a> Visit<'a> for Collector<'a> {
    fn enter_node(&mut self, kind: AstKind<'a>) {
        match kind {
            AstKind::TemplateLiteral(template) => self.templates.push(template.span),
            AstKind::CallExpression(call) if should_expand(call) && !self.nested_in_unexpanded_argument(call) => self.calls.push(call),
            _ => {}
        }

        self.ancestors.push(kind);
    }

    fn leave_node(&mut self, _kind: AstKind<'a>) {
        self.ancestors.pop();
    }
}

struct Writer<'t> {
    text: &'t str,
    unit: &'t str,
    comments: &'t [Comment],
    templates: &'t [Span],
}

impl Writer<'_> {
    fn slice(&self, span: Span) -> &str {
        &self.text[span.start as usize..span.end as usize]
    }

    fn parens(&self, call: &CallExpression<'_>) -> Option<(u32, u32)> {
        call_parens(self.text, call, unwrap_expression(&call.callee).span().end)
    }

    /// `(\n<arg>,\n<arg>,\n<indent>)` with arguments one unit deeper than `indent`.
    fn call_parens(&self, call: &CallExpression<'_>, indent: &str) -> Option<String> {
        let (open, close) = self.parens(call)?;

        if call.arguments.is_empty() || has_comment_between(self.comments, open, close) || !should_expand(call) {
            return None;
        }

        let argument_indent = format!("{indent}{}", self.unit);
        let separator = format!(",\n{argument_indent}");
        let arguments: Vec<String> = call.arguments.iter().map(|argument| self.node(argument, &argument_indent)).collect();
        let trailing = if matches!(call.arguments.last(), Some(Argument::SpreadElement(_))) { "" } else { "," };

        Some(format!("(\n{argument_indent}{}{trailing}\n{indent})", arguments.join(&separator)))
    }

    /// An argument placed at `indent`: an expandable call is expanded in turn,
    /// anything else is re-indented from the depth its text came from.
    fn node(&self, argument: &Argument<'_>, indent: &str) -> String {
        if let Argument::CallExpression(call) = argument
            && should_expand(call)
        {
            if let Some(parens) = self.call_parens(call, indent)
                && let Some((open, _)) = self.parens(call)
            {
                return format!("{}{parens}", &self.text[call.span.start as usize..open as usize]);
            }

            return self.rebase(call.span, indent);
        }

        self.rebase(argument.span(), indent)
    }

    fn in_template(&self, offset: usize) -> bool {
        self.templates.iter().any(|span| (span.start as usize) < offset && offset < span.end as usize)
    }

    /// Move a node's continuation lines from its own line indent to `to`.
    /// Template-literal content keeps its whitespace, which is string data.
    fn rebase(&self, span: Span, to: &str) -> String {
        let text = self.slice(span);
        let from = line_indent(self.text, span.start);

        if from == to || !text.contains('\n') {
            return text.to_owned();
        }

        let mut out = String::with_capacity(text.len() + 16);
        let mut line_start = span.start as usize;

        for (index, line) in text.split('\n').enumerate() {
            if index > 0 {
                out.push('\n');

                if self.in_template(line_start) {
                    out.push_str(line);
                } else if line.trim().is_empty() {
                } else if let Some(rest) = line.strip_prefix(from) {
                    out.push_str(to);
                    out.push_str(rest);
                } else {
                    out.push_str(line);
                }
            } else {
                out.push_str(line);
            }

            line_start += line.len() + 1;
        }

        out
    }
}
