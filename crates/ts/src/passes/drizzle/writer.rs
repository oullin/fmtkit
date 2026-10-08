//! Lay out a Drizzle call's arguments, recursing into option objects, arrays,
//! multiline helpers, and set operations.

use fmtkit_core::Edit;
use oxc_ast::ast::{
    Argument, ArrayExpression, ArrayExpressionElement, CallExpression, Comment, Expression, ObjectExpression, ObjectPropertyKind, PropertyKind,
};
use oxc_span::{GetSpan, Span};

use super::classifier::{Classifier, formats_array, formats_object};
use super::vocabulary;
use crate::syntax::{Chained, call_parens, has_comment_between, line_indent, property_span};

pub(crate) struct Writer<'w, 'a> {
    pub text: &'a str,
    pub unit: &'a str,
    pub comments: &'a [Comment],
    pub classifier: &'w Classifier<'w, 'a>,
}

impl<'a> Writer<'_, 'a> {
    fn slice(&self, span: Span) -> &'a str {
        &self.text[span.start as usize..span.end as usize]
    }

    fn commented(&self, span: Span) -> bool {
        has_comment_between(self.comments, span.start, span.end)
    }

    /// The edit laying out `call`'s arguments one per line.
    pub(crate) fn call(&self, call: &'a CallExpression<'a>) -> Option<Edit> {
        let callee = Chained::of(&call.callee);
        let (open, close) = call_parens(self.text, call, callee.span().end)?;

        if call.arguments.is_empty() || has_comment_between(self.comments, open, close) {
            return None;
        }

        let anchor = match callee {
            Chained::Member(member) => property_span(member).start,
            _ => call.span.start,
        };

        let indent = line_indent(self.text, anchor);
        let replacement = self.list("(", ")", call.arguments.iter().map(|argument| self.argument(argument, &format!("{indent}{}", self.unit))), indent);

        (replacement != self.text[open as usize..=close as usize]).then(|| Edit::new(open, close + 1, replacement))
    }

    /// `<open>\n<item>,\n<item>,\n<indent><close>` with items one unit deeper.
    fn list(&self, open: &str, close: &str, items: impl Iterator<Item = String>, indent: &str) -> String {
        let mut out = String::from(open);

        for item in items {
            out.push('\n');
            out.push_str(indent);
            out.push_str(self.unit);
            out.push_str(&item);
            out.push(',');
        }

        out.push('\n');
        out.push_str(indent);
        out.push_str(close);

        out
    }

    fn argument(&self, argument: &'a Argument<'a>, indent: &str) -> String {
        argument.as_expression().map_or_else(|| self.slice(argument.span()).to_owned(), |expression| self.node(expression, indent))
    }

    fn node(&self, expression: &'a Expression<'a>, indent: &str) -> String {
        match expression {
            Expression::ObjectExpression(object) if formats_object(object) => self.object(object, indent),
            Expression::ArrayExpression(array) if formats_array(array) => self.array(array, indent),
            Expression::CallExpression(call) if self.classifier.is_set_operation(call) => self.helper(call, indent, call.arguments.len() >= 2),
            Expression::CallExpression(call) if self.classifier.is_helper_call(call) => {
                let multiline = self.classifier.callee_name(Chained::of(&call.callee)).is_some_and(vocabulary::is_multiline_helper);

                self.helper(call, indent, multiline && !call.arguments.is_empty())
            }
            _ => self.slice(expression.span()).to_owned(),
        }
    }

    fn array(&self, array: &'a ArrayExpression<'a>, indent: &str) -> String {
        if self.commented(array.span) {
            return self.slice(array.span).to_owned();
        }

        if array.elements.is_empty() {
            return "[]".to_owned();
        }

        let next = format!("{indent}{}", self.unit);
        let elements = array.elements.iter().map(|element| match element {
            ArrayExpressionElement::Elision(_) => String::new(),
            ArrayExpressionElement::SpreadElement(spread) => self.slice(spread.span).to_owned(),
            element => element.as_expression().map_or_else(String::new, |expression| self.node(expression, &next)),
        });

        self.list("[", "]", elements, indent)
    }

    fn object(&self, object: &'a ObjectExpression<'a>, indent: &str) -> String {
        if self.commented(object.span) {
            return self.slice(object.span).to_owned();
        }

        if object.properties.is_empty() {
            return "{}".to_owned();
        }

        let next = format!("{indent}{}", self.unit);
        let properties = object.properties.iter().map(|property| match property {
            // Accessors keep their text: `key: value` would turn `get x() {}` into invalid code.
            ObjectPropertyKind::ObjectProperty(property)
                if !property.computed && !property.method && !property.shorthand && property.kind == PropertyKind::Init =>
            {
                format!("{}: {}", self.slice(property.key.span()), self.node(&property.value, &next))
            }
            property => self.slice(property.span()).to_owned(),
        });

        self.list("{", "}", properties, indent)
    }

    /// A helper or set-operation call, laid out when `expand` holds.
    fn helper(&self, call: &'a CallExpression<'a>, indent: &str, expand: bool) -> String {
        if self.commented(call.span) || !expand {
            return self.slice(call.span).to_owned();
        }

        let next = format!("{indent}{}", self.unit);
        let callee = self.slice(Chained::of(&call.callee).span());
        let arguments = call.arguments.iter().map(|argument| self.argument(argument, &next));

        format!("{callee}{}", self.list("(", ")", arguments, indent))
    }
}
