//! Stylesheets embedded in hosts, through oxc_formatter_css.

use fmtkit_config::{TrailingComma, TsFormat};
use oxc_allocator::Allocator;
use oxc_formatter_core::{IndentStyle, IndentWidth, LineEnding, LineWidth};
use oxc_formatter_css::{CssFormatOptions, CssVariant, TrailingCommas};

/// Format a stylesheet at `width` columns. `None` when it does not parse.
pub(crate) fn format(code: &str, variant: CssVariant, options: &TsFormat, width: usize) -> Option<String> {
    let allocator = Allocator::default();
    let formatted = oxc_formatter_css::format(&allocator, code, css_options(variant, options, width)).ok()?;

    formatted.print().ok().map(oxc_formatter_core::Printed::into_code)
}

/// Format the declarations of a `style="…"` attribute onto one line, as
/// Prettier does (`color: red; margin: 0`). `None` keeps the attribute as
/// written: it does not parse, or holds more than plain declarations.
pub(crate) fn format_declarations(code: &str, options: &TsFormat) -> Option<String> {
    let wrapped = format!("a {{\n{code}\n}}");
    let formatted = format(&wrapped, CssVariant::Css, options, usize::MAX)?;
    let body = formatted.trim_end().strip_prefix("a {")?.strip_suffix('}')?;
    let mut declarations = Vec::new();

    for line in body.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let declaration = line.strip_suffix(';')?;

        if declaration.contains(['{', '}', ';']) || declaration.starts_with("/*") || declaration.starts_with("//") {
            return None;
        }

        declarations.push(declaration);
    }

    Some(declarations.join("; "))
}

fn css_options(variant: CssVariant, options: &TsFormat, width: usize) -> CssFormatOptions {
    CssFormatOptions {
        indent_style: indent_style(options),
        indent_width: indent_width(options),
        line_width: line_width(width),
        line_ending: LineEnding::Lf,
        variant,
        single_quote: options.single_quote.into(),
        trailing_commas: if options.trailing_comma == TrailingComma::None { TrailingCommas::Never } else { TrailingCommas::Always },
        sort_tailwindcss: false,
    }
}

pub(crate) fn indent_style(options: &TsFormat) -> IndentStyle {
    if options.use_tabs { IndentStyle::Tab } else { IndentStyle::Space }
}

pub(crate) fn indent_width(options: &TsFormat) -> IndentWidth {
    IndentWidth::try_from(options.tab_width.min(IndentWidth::MAX)).unwrap_or_default()
}

/// `width` clamped to what the oxc printers accept.
pub(crate) fn line_width(width: usize) -> LineWidth {
    let width = u16::try_from(width).unwrap_or(u16::MAX).clamp(LineWidth::MIN, LineWidth::MAX);

    LineWidth::try_from(width).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_a_stylesheet_with_the_configured_indent() {
        let out = format("a{color:red}", CssVariant::Css, &TsFormat::default(), 80);

        assert_eq!(out.as_deref(), Some("a {\n\tcolor: red;\n}\n"));
    }

    #[test]
    fn scss_and_less_parse_in_their_dialects() {
        let scss = format("$x: 1px;\n.a{ .b{ margin:$x } }", CssVariant::Scss, &TsFormat::default(), 80);
        let less = format("@x: 1px;\n.a{ .b{ margin:@x } }", CssVariant::Less, &TsFormat::default(), 80);

        assert_eq!(scss.as_deref(), Some("$x: 1px;\n.a {\n\t.b {\n\t\tmargin: $x;\n\t}\n}\n"));
        assert_eq!(less.as_deref(), Some("@x: 1px;\n.a {\n\t.b {\n\t\tmargin: @x;\n\t}\n}\n"));
    }

    #[test]
    fn invalid_css_is_rejected() {
        assert_eq!(format("a { color: red", CssVariant::Css, &TsFormat::default(), 80), None);
    }

    #[test]
    fn style_attribute_declarations_join_on_one_line() {
        let options = TsFormat::default();

        assert_eq!(format_declarations("color:red;margin:0", &options).as_deref(), Some("color: red; margin: 0"));
        assert_eq!(format_declarations(" color : red ", &options).as_deref(), Some("color: red"));
        assert_eq!(format_declarations("", &options).as_deref(), Some(""));
        assert_eq!(format_declarations("color: red; /* note */", &options), None);
        assert_eq!(format_declarations("color: {{ x }}", &options), None);
    }

    #[test]
    fn widths_clamp_to_the_printer_range() {
        assert_eq!(line_width(0).value(), LineWidth::MIN);
        assert_eq!(line_width(usize::MAX).value(), LineWidth::MAX);
        assert_eq!(line_width(200).value(), 200);
    }
}
