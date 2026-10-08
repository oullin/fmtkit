//! Parsing and the text queries every pass shares.
//!
//! Offsets are UTF-8 byte offsets, as in oxc spans and `fmtkit_core::Edit`.

use std::path::Path;

use fmtkit_core::{Lang, is_declaration};
use oxc_allocator::Allocator;
use oxc_ast::ast::{CallExpression, ChainElement, Comment, Expression, MemberExpression, Program, TSNonNullExpression};
use oxc_parser::{ParseOptions, Parser};
use oxc_span::{GetSpan, SourceType, Span};

/// The parser's first complaint about a text.
#[derive(Debug)]
pub(crate) struct ParseFailure {
    pub offset: u32,
    pub message: String,
}

/// The source type for a script, equivalent to `SourceType::from_path` on a
/// file with the language's extension. `None` for non-script languages.
pub(crate) fn source_type(lang: Lang, rel: &str) -> Option<SourceType> {
    let extension = match lang {
        Lang::Ts => "ts",
        Lang::Tsx => "tsx",
        Lang::Mts => "mts",
        Lang::Cts => "cts",
        Lang::Js => "js",
        Lang::Jsx => "jsx",
        Lang::Mjs => "mjs",
        Lang::Cjs => "cjs",
        Lang::Go | Lang::Vue | Lang::Html | Lang::Markdown => return None,
    };

    let source_type = SourceType::from_extension(extension).ok()?;
    let definition = source_type.is_typescript() && is_declaration(Path::new(rel));

    Some(source_type.with_typescript_definition(definition))
}

/// Parse `text` with oxc's default options, so parentheses are preserved as
/// `ParenthesizedExpression` nodes exactly as the v1 ESTree reader saw them.
/// Any diagnostic is a failure.
pub(crate) fn parse<'a>(allocator: &'a Allocator, text: &'a str, source_type: SourceType) -> Result<&'a Program<'a>, ParseFailure> {
    let parsed = Parser::new(allocator, text, source_type).with_options(ParseOptions::default()).parse();

    if let Some(diagnostic) = parsed.diagnostics.first() {
        let offset = diagnostic.labels.first().map_or(0, oxc_span::LabeledSpan::offset);

        return Err(ParseFailure { offset, message: diagnostic.message.to_string() });
    }

    Ok(allocator.alloc(parsed.program))
}

/// The offset where the line containing `position` starts.
pub(crate) fn line_start(text: &str, position: u32) -> u32 {
    text[..position as usize].rfind('\n').map_or(0, |i| to_u32(i + 1))
}

/// The leading spaces and tabs of the line containing `position`, up to `position`.
pub(crate) fn line_indent(text: &str, position: u32) -> &str {
    let start = line_start(text, position) as usize;
    let line = &text[start..position as usize];
    let width = line.bytes().take_while(|b| *b == b' ' || *b == b'\t').count();

    &line[..width]
}

/// The file's per-level indent unit, read relative to its baseline.
///
/// The baseline is the shortest leading whitespace across non-blank lines that
/// do not continue a block comment (`*`); the unit is the first deeper indent
/// with the baseline removed. Falls back to a tab.
pub(crate) fn indent_unit(text: &str) -> &str {
    let mut baseline: Option<&str> = None;

    for indent in content_indents(text) {
        if baseline.is_none_or(|shortest| indent.len() < shortest.len()) {
            baseline = Some(indent);
        }
    }

    let Some(baseline) = baseline else {
        return "\t";
    };

    content_indents(text).find(|indent| indent.len() > baseline.len()).map_or("\t", |indent| &indent[baseline.len()..])
}

fn content_indents(text: &str) -> impl Iterator<Item = &str> {
    text.split('\n').filter_map(|line| {
        let width = line.bytes().take_while(|b| *b == b' ' || *b == b'\t').count();
        let next = line[width..].chars().next()?;

        (!next.is_whitespace() && next != '*').then_some(&line[..width])
    })
}

/// Whether a comment lies entirely within `[from, to]`.
pub(crate) fn has_comment_between(comments: &[Comment], from: u32, to: u32) -> bool {
    let index = comments.partition_point(|comment| comment.span.start < from);

    comments.get(index).is_some_and(|comment| comment.span.end <= to)
}

/// Whether `text` holds a `//` or `/*` comment opener.
pub(crate) fn contains_comment_opener(text: &str) -> bool {
    text.contains("//") || text.contains("/*")
}

/// The opening and closing offsets of a call's argument parentheses.
///
/// The opening parenthesis is the first one after the callee and after any
/// call-site type arguments, so `f<() => void>(x)` does not match the
/// parenthesis inside the type argument.
pub(crate) fn call_parens(text: &str, call: &CallExpression<'_>, callee_end: u32) -> Option<(u32, u32)> {
    let type_arguments_end = call.type_arguments.as_ref().map_or(0, |args| args.span.end);
    let from = callee_end.max(type_arguments_end) as usize;
    let open = to_u32(from + text.get(from..)?.find('(')?);
    let close = call.span.end.checked_sub(1)?;

    (open < call.span.end && text.as_bytes().get(close as usize) == Some(&b')')).then_some((open, close))
}

/// An expression seen through one `ChainExpression` wrapper, as the v1
/// `unwrapChainExpression` did.
#[derive(Clone, Copy)]
pub(crate) enum Chained<'a> {
    Call(&'a CallExpression<'a>),
    Member(&'a MemberExpression<'a>),
    NonNull(&'a TSNonNullExpression<'a>),
    Other(&'a Expression<'a>),
}

impl<'a> Chained<'a> {
    pub(crate) fn of(expression: &'a Expression<'a>) -> Self {
        match expression {
            Expression::CallExpression(call) => Self::Call(call),
            Expression::ChainExpression(chain) => match &chain.expression {
                ChainElement::CallExpression(call) => Self::Call(call),
                ChainElement::TSNonNullExpression(non_null) => Self::NonNull(non_null),
                element => element.as_member_expression().map_or(Self::Other(expression), Self::Member),
            },
            _ => expression.as_member_expression().map_or(Self::Other(expression), Self::Member),
        }
    }

    pub(crate) fn span(self) -> Span {
        match self {
            Self::Call(call) => call.span,
            Self::Member(member) => member.span(),
            Self::NonNull(non_null) => non_null.span,
            Self::Other(expression) => expression.span(),
        }
    }
}

/// An expression with chain, parenthesis, and TypeScript assertion wrappers
/// removed, as the v1 expanded-call pass unwrapped it.
pub(crate) fn unwrap_expression<'a>(expression: &'a Expression<'a>) -> Chained<'a> {
    let mut current = Chained::of(expression);

    loop {
        current = match current {
            Chained::NonNull(non_null) => Chained::of(&non_null.expression),
            Chained::Other(Expression::ParenthesizedExpression(inner)) => Chained::of(&inner.expression),
            Chained::Other(Expression::TSAsExpression(inner)) => Chained::of(&inner.expression),
            Chained::Other(Expression::TSSatisfiesExpression(inner)) => Chained::of(&inner.expression),
            Chained::Other(Expression::TSNonNullExpression(inner)) => Chained::of(&inner.expression),
            Chained::Other(Expression::TSTypeAssertion(inner)) => Chained::of(&inner.expression),
            done => return done,
        };
    }
}

/// The non-computed identifier property name of a member expression.
pub(crate) fn static_property_name<'a>(member: &'a MemberExpression<'a>) -> Option<&'a str> {
    match member {
        MemberExpression::StaticMemberExpression(member) => Some(member.property.name.as_str()),
        _ => None,
    }
}

/// The span ESTree reports as a member's `property`.
pub(crate) fn property_span(member: &MemberExpression<'_>) -> Span {
    match member {
        MemberExpression::StaticMemberExpression(member) => member.property.span,
        MemberExpression::ComputedMemberExpression(member) => member.expression.span(),
        MemberExpression::PrivateFieldExpression(member) => member.field.span,
    }
}

pub(crate) fn to_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use oxc_allocator::Allocator;
    use oxc_ast::ast::{Expression, Statement};
    use oxc_span::SourceType;

    use super::*;

    fn offset(text: &str, needle: &str) -> u32 {
        to_u32(text.find(needle).unwrap())
    }

    #[test]
    fn line_start_returns_the_offset_after_the_preceding_newline() {
        let text = "if (x) {\n\t\tcall();\n}\n";

        assert_eq!(line_start(text, offset(text, "call")), offset(text, "\n") + 1);
        assert_eq!(line_start(text, 0), 0);
    }

    #[test]
    fn line_indent_returns_the_leading_whitespace_of_the_position_line() {
        let text = "if (x) {\n\t\tcall();\n}\n";

        assert_eq!(line_indent(text, offset(text, "call")), "\t\t");
        assert_eq!(line_indent(text, 0), "");
    }

    #[test]
    fn indent_unit_reads_the_unit_from_the_first_indented_line() {
        assert_eq!(indent_unit("function run() {\n    return go();\n}\n"), "    ");
        assert_eq!(indent_unit("function run() {\n\treturn go();\n}\n"), "\t");
        assert_eq!(indent_unit("const value = {\n  key: 1,\n};\n"), "  ");
    }

    #[test]
    fn indent_unit_falls_back_to_a_tab_for_unindented_source() {
        assert_eq!(indent_unit("const value = 1;\n"), "\t");
        assert_eq!(indent_unit(""), "\t");
    }

    #[test]
    fn indent_unit_skips_block_comment_continuation_lines() {
        assert_eq!(indent_unit("/**\n * Doc comment.\n */\nfunction run() {\n    return go();\n}\n"), "    ");
    }

    #[test]
    fn indent_unit_reads_the_unit_relative_to_a_baseline_indented_block() {
        assert_eq!(indent_unit("\t\t\tconst r = builder().withA(1).withB(2).withC(3).build();\n"), "\t");
        assert_eq!(indent_unit("\t\t\tfunction run() {\n\t\t\t\treturn go();\n\t\t\t}\n"), "\t");
        assert_eq!(indent_unit("      const value = {\n        key: 1,\n      };\n"), "  ");
    }

    fn first_call<'a>(program: &'a Program<'a>) -> &'a CallExpression<'a> {
        let Some(Statement::ExpressionStatement(statement)) = program.body.first() else { panic!("expected an expression statement") };
        let Expression::CallExpression(call) = &statement.expression else { panic!("expected a call") };

        call
    }

    #[test]
    fn call_parens_locates_argument_parentheses() {
        let allocator = Allocator::default();
        let text = "wrap(value);\n";
        let program = parse(&allocator, text, SourceType::ts()).unwrap();
        let call = first_call(program);

        assert_eq!(call_parens(text, call, Chained::of(&call.callee).span().end), Some((offset(text, "("), offset(text, ")"))));
    }

    #[test]
    fn call_parens_scans_past_a_type_argument_list_holding_a_function_type() {
        let allocator = Allocator::default();
        let text = "wrap<Readonly<Record<Frame, () => void>>>(value);\n";
        let program = parse(&allocator, text, SourceType::ts()).unwrap();
        let call = first_call(program);
        let close = to_u32(text.rfind(')').unwrap());

        assert_eq!(call_parens(text, call, Chained::of(&call.callee).span().end), Some((offset(text, "(value"), close)));
    }

    #[test]
    fn chained_unwraps_an_optional_call() {
        let allocator = Allocator::default();
        let program = parse(&allocator, "a?.b();\n", SourceType::ts()).unwrap();
        let Some(Statement::ExpressionStatement(statement)) = program.body.first() else { panic!("expected an expression statement") };

        assert!(matches!(Chained::of(&statement.expression), Chained::Call(_)));
    }

    #[test]
    fn comment_between_requires_the_whole_comment_inside() {
        let allocator = Allocator::default();
        let text = "a(/* x */ 1);";
        let program = parse(&allocator, text, SourceType::ts()).unwrap();

        assert!(has_comment_between(&program.comments, 1, 11));
        assert!(!has_comment_between(&program.comments, 3, 11));
        assert!(!has_comment_between(&program.comments, 1, 5));
    }

    #[test]
    fn source_types_follow_the_extension() {
        assert!(source_type(Lang::Tsx, "a.tsx").unwrap().is_jsx());
        assert!(!source_type(Lang::Js, "a.js").unwrap().is_jsx());
        assert!(source_type(Lang::Mts, "a.mts").unwrap().is_module());
        assert!(source_type(Lang::Ts, "types/a.d.ts").unwrap().is_typescript_definition());
        assert!(source_type(Lang::Vue, "a.vue").is_none());
    }
}
