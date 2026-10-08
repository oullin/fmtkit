//! The `oxc_formatter` step, with options mapped from `[ts.format]` the way
//! oxfmt maps `.oxfmtrc.json`.
//!
//! The formatter runs on a session carrying the [embedded-language
//! dispatcher](crate::embed), as oxfmt 0.71.0 did, so tagged CSS, GraphQL,
//! HTML, and Markdown templates are formatted too.

use std::borrow::Cow;

use fmtkit_config::{ArrowParens, TrailingComma, TsFormat};
use oxc_allocator::Allocator;
use oxc_ast::ast::{Program, TemplateElement};
use oxc_ast_visit::Visit;
use oxc_formatter::{ArrowParentheses, JsFormatOptions, QuoteStyle, Semicolons, TrailingCommas};
use oxc_formatter_core::{FormatSession, IndentStyle, IndentWidth, InputKind, LineWidth, SessionServices};
use oxc_span::{SourceType, Span};

/// The formatter options for `options`. Fields `[ts.format]` does not cover
/// keep oxfmt's defaults; an out-of-range width keeps the default width.
pub(crate) fn js_options(options: &TsFormat) -> JsFormatOptions {
    let mut js = JsFormatOptions::default();

    js.indent_style = if options.use_tabs { IndentStyle::Tab } else { IndentStyle::Space };
    js.indent_width = IndentWidth::try_from(options.tab_width).unwrap_or(js.indent_width);
    js.line_width = LineWidth::try_from(options.print_width).unwrap_or(js.line_width);
    js.quote_style = if options.single_quote { QuoteStyle::Single } else { QuoteStyle::Double };
    js.semicolons = if options.semi { Semicolons::Always } else { Semicolons::AsNeeded };

    js.trailing_commas = match options.trailing_comma {
        TrailingComma::All => TrailingCommas::All,
        TrailingComma::Es5 => TrailingCommas::Es5,
        TrailingComma::None => TrailingCommas::None,
    };

    js.arrow_parentheses = match options.arrow_parens {
        ArrowParens::Always => ArrowParentheses::Always,
        ArrowParens::Avoid => ArrowParentheses::AsNeeded,
    };

    js
}

/// Print `text` with `oxc_formatter` on a session carrying `services`.
/// Whitespace-only input becomes empty, as oxfmt leaves it. The error is the
/// formatter's message.
pub(crate) fn format(
    allocator: &Allocator,
    text: &str,
    source_type: SourceType,
    options: JsFormatOptions,
    services: &SessionServices,
) -> Result<String, String> {
    if text.trim().is_empty() {
        return Ok(String::new());
    }

    let session = FormatSession::with_services(allocator, InputKind::PhysicalFile, services.clone());
    let formatted = oxc_formatter::format_with_session(&session, text, source_type, options).map_err(|diagnostic| diagnostic.message.to_string())?;

    formatted.print().map(oxc_formatter_core::Printed::into_code).map_err(|error| error.to_string())
}

/// `text` with every `\r\n` and lone `\r` inside a template element of
/// `program` turned into `\n`; borrowed when there is nothing to change.
///
/// oxc_formatter prints the raw text of a template without a cooked value (an
/// invalid escape such as `\1` in a tagged template) through a text builder
/// whose `debug_assert` rejects `\r` (see `docs/known-issues.md`). The rewrite
/// keeps the program: ECMAScript reads `\r\n` and `\r` in a template as `\n`
/// in both its cooked and its raw value, and oxc_formatter prints every other
/// template that way already. Nothing outside a template element changes, so
/// line breaks in layout, comments, and JSX keep whatever they are.
pub(crate) fn template_line_breaks<'t>(text: &'t str, program: &Program<'_>) -> Cow<'t, str> {
    if !text.contains('\r') {
        return Cow::Borrowed(text);
    }

    let mut elements = TemplateElements(Vec::new());

    elements.visit_program(program);

    let mut out = String::with_capacity(text.len());
    let mut copied = 0;

    for span in elements.0 {
        let (start, end) = (span.start as usize, span.end as usize);

        if !text[start..end].contains('\r') {
            continue;
        }

        out.push_str(&text[copied..start]);
        out.push_str(&text[start..end].replace("\r\n", "\n").replace('\r', "\n"));
        copied = end;
    }

    if copied == 0 {
        return Cow::Borrowed(text);
    }

    out.push_str(&text[copied..]);

    Cow::Owned(out)
}

/// The spans of every template element, in source order.
struct TemplateElements(Vec<Span>);

impl<'a> Visit<'a> for TemplateElements {
    fn visit_template_element(&mut self, it: &TemplateElement<'a>) {
        self.0.push(it.span);
    }
}

#[cfg(test)]
mod tests {
    use fmtkit_config::TsFormat;
    use oxc_allocator::Allocator;
    use oxc_span::SourceType;

    use super::{format, js_options, template_line_breaks};
    use crate::embed;

    fn run(text: &str, options: &TsFormat) -> String {
        let js = js_options(options);
        let services = embed::services(options, &js);

        format(&Allocator::default(), text, SourceType::ts(), js, &services).expect("formats")
    }

    #[test]
    fn prints_with_the_default_fmtkit_style() {
        assert_eq!(run("const a = {b: \"c\"}\nfunction f(x) { return x }", &TsFormat::default()), "const a = { b: 'c' };\nfunction f(x) {\n\treturn x;\n}\n");
    }

    #[test]
    fn maps_every_option() {
        let options = TsFormat {
            use_tabs: false,
            tab_width: 2,
            print_width: 20,
            single_quote: false,
            semi: false,
            trailing_comma: fmtkit_config::TrailingComma::None,
            arrow_parens: fmtkit_config::ArrowParens::Avoid,
        };

        assert_eq!(run("const f = (x) => [aaaaaaaa, bbbbbbbb, 'c'];", &options), "const f = x => [\n  aaaaaaaa,\n  bbbbbbbb,\n  \"c\"\n]\n");
    }

    #[test]
    fn formats_embedded_language_templates() {
        let source = "const style = css`a{color:red}`;\nconst query = gql`query{a}`;\n";

        assert_eq!(run(source, &TsFormat::default()), "const style = css`\n\ta {\n\t\tcolor: red;\n\t}\n`;\nconst query = gql`\n\tquery {\n\t\ta\n\t}\n`;\n");
    }

    #[test]
    fn turns_carriage_returns_into_line_feeds_inside_template_elements_only() {
        let rewrite = |text: &str| {
            let allocator = Allocator::default();
            let program = crate::syntax::parse(&allocator, text, SourceType::tsx()).expect("parses");

            template_line_breaks(text, program).into_owned()
        };

        assert_eq!(rewrite("a`\r\\1`"), "a`\n\\1`");
        assert_eq!(rewrite("a`x\r\n${b}\r`;\r/*\r*/c"), "a`x\n${b}\n`;\r/*\r*/c");
        assert_eq!(rewrite("type T = `a\rb`;"), "type T = `a\nb`;");
        assert_eq!(rewrite("const v = <a b=\"x\ry\">\r</a>;\r`a`"), "const v = <a b=\"x\ry\">\r</a>;\r`a`");
    }

    /// Fuzz finding `oxc-debug-assert-lone-cr-in-template`.
    #[test]
    fn formats_a_template_without_a_cooked_value_across_a_carriage_return() {
        for source in ["a`\r\\1`", "a`\r\n\\1`"] {
            let allocator = Allocator::default();
            let program = crate::syntax::parse(&allocator, source, SourceType::ts()).expect("parses");

            assert_eq!(run(&template_line_breaks(source, program), &TsFormat::default()), "a`\n\\1`;\n");
        }
    }

    #[test]
    fn empties_whitespace_only_input() {
        assert_eq!(run(" \n\t\n", &TsFormat::default()), "");
    }
}
