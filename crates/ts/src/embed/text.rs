//! The text channel: HTML through markup_fmt and Markdown through
//! oxc_formatter_markdown, where oxfmt calls Prettier (`prettier_string.rs`
//! for the text, `prettier_doc.rs` for handing it to the JS document).
//!
//! The formatted text becomes IR line by line. Prettier's document would let
//! the JS printer break the embedded code at its real column; text cannot, so
//! HTML is laid out one indent narrower than the configured width, which is
//! where a template body usually sits.

use std::borrow::Cow;
use std::ops::Range;

use fmtkit_config::TsFormat;
use markup_fmt::config::{FormatOptions, LanguageOptions, LayoutOptions, LineBreak, Quotes};
use markup_fmt::{Hints, Language};
use oxc_allocator::{Allocator, ArenaVec};
use oxc_formatter::JsFormatOptions;
use oxc_formatter_core::{FormatElement, FormatSession, IndentWidth, LineEnding, LineMode, LineWidth, TextWidth};
use oxc_formatter_css::{CssFormatOptions, CssVariant};
use oxc_formatter_markdown::{MarkdownFormatOptions, ProseWrap};
use oxc_markdown_parser::Segment;
use oxc_markdown_parser::ast::{Block, CodeBlock, CodeBlockKind};
use oxc_span::SourceType;

/// The options of both text formatters and of the scripts and styles inside
/// HTML, derived once per file.
pub(super) struct TextOptions {
    pub(super) indent_width: IndentWidth,
    markup: FormatOptions,
    markdown: MarkdownFormatOptions,
    js: JsFormatOptions,
    css: CssFormatOptions,
}

impl TextOptions {
    /// Prettier's defaults on top of `[ts.format]`, as `crates/hosts`
    /// configures the same formatters for whole documents: double-quoted
    /// attributes, indented `<script>` and `<style>` bodies, and
    /// `proseWrap: "preserve"`.
    pub(super) fn new(options: &TsFormat, js: &JsFormatOptions, css: CssFormatOptions) -> Self {
        let width = usize::from(js.line_width.value()).saturating_sub(usize::from(js.indent_width.value()));

        let markup = FormatOptions {
            layout: LayoutOptions {
                print_width: width,
                use_tabs: options.use_tabs,
                indent_width: usize::from(js.indent_width.value()),
                line_break: LineBreak::Lf,
            },
            language: LanguageOptions { quotes: Quotes::Double, html_script_indent: Some(true), html_style_indent: Some(true), ..LanguageOptions::default() },
        };

        let markdown = MarkdownFormatOptions {
            indent_style: js.indent_style,
            indent_width: js.indent_width,
            line_width: js.line_width,
            line_ending: LineEnding::Lf,
            prose_wrap: ProseWrap::Preserve,
            single_quote: options.single_quote.into(),
        };

        Self { indent_width: js.indent_width, markup, markdown, js: js.clone(), css }
    }
}

/// Format an HTML template body. `None` keeps it as written.
pub(super) fn html(code: &str, options: &TextOptions) -> Option<String> {
    let formatted = markup_fmt::format_text(code, Language::Html, &options.markup, |code, hints| Ok(embedded(code, &hints, options))).ok()?;

    Some(trim(&formatted).to_owned())
}

/// markup_fmt's callback: `<style>` through oxc_formatter_css and `<script>`
/// through oxc_formatter, as Prettier formats them. Attributes and anything
/// else stay as written.
fn embedded<'a>(code: &'a str, hints: &Hints<'_>, options: &TextOptions) -> Cow<'a, str> {
    if hints.attr {
        return Cow::Borrowed(code);
    }

    let allocator = Allocator::default();
    let width = LineWidth::try_from(u16::try_from(hints.print_width).unwrap_or(u16::MAX).clamp(LineWidth::MIN, LineWidth::MAX)).unwrap_or_default();

    let formatted = match hints.ext {
        "css" | "scss" | "less" => {
            let variant = match hints.ext {
                "scss" => CssVariant::Scss,
                "less" => CssVariant::Less,
                _ => CssVariant::Css,
            };

            oxc_formatter_css::format(&allocator, code, CssFormatOptions { variant, line_width: width, ..options.css })
                .ok()
                .and_then(|formatted| formatted.print().ok())
        }
        "js" | "mjs" | "jsx" | "ts" | "mts" | "tsx" => SourceType::from_extension(hints.ext).ok().and_then(|source_type| {
            let js = JsFormatOptions { line_width: width, ..options.js.clone() };

            oxc_formatter::format(&allocator, code, source_type, js).ok().and_then(|formatted| formatted.print().ok())
        }),
        _ => None,
    };

    formatted.map_or(Cow::Borrowed(code), |printed| Cow::Owned(printed.into_code()))
}

/// Format a Markdown template body (already unescaped and dedented by
/// oxc_formatter). `None` keeps it as written.
pub(super) fn markdown(code: &str, options: &TextOptions) -> Option<String> {
    let allocator = Allocator::default();
    let printed = oxc_formatter_markdown::format(&allocator, code, options.markdown).ok()?.print().ok()?.into_code();

    Some(tilde_fences(trim(&printed)))
}

/// Prettier formats Markdown in a template with `__inJsTemplate`, which
/// fences every code block with tildes so its backticks need no escaping.
/// oxc_formatter_markdown has no such option; rewrite its backtick fences.
fn tilde_fences(printed: &str) -> String {
    let allocator = Allocator::default();
    let mut runs: Vec<(Range<usize>, usize)> = Vec::new();

    if let Ok(parsed) = oxc_formatter_markdown::parse_for_format(&allocator, printed)
        && parsed.source.len() == printed.len()
    {
        let mut fences = Vec::new();

        collect_fences(&parsed.root.children, &mut fences);

        for fence in fences {
            if let CodeBlockKind::Fenced { fence: b'`', .. } = fence.kind
                && let Some([open, close]) = fence_runs(printed, fence)
            {
                let content = Segment::join(parsed.source, &fence.lines);
                let tildes = content.split(|c| c != '~').map(str::len).max().unwrap_or(0).saturating_add(1).max(3);

                runs.push((open, tildes));
                runs.push((close, tildes));
            }
        }
    }

    let mut out = String::with_capacity(printed.len());
    let mut cursor = 0;

    for (run, tildes) in runs {
        out.push_str(&printed[cursor..run.start]);
        out.push_str(&"~".repeat(tildes));
        cursor = run.end;
    }

    out.push_str(&printed[cursor..]);

    out
}

/// The byte ranges of a printed fence's opening and closing backtick runs.
fn fence_runs(printed: &str, fence: &CodeBlock<'_>) -> Option<[Range<usize>; 2]> {
    let start = fence.span.start as usize;
    let block = printed.get(start..fence.span.end as usize)?.trim_end_matches('\n');
    let open = block.find('`')?;
    let open_end = open + block[open..].bytes().take_while(|&byte| byte == b'`').count();
    let closing_line = block.rfind('\n')? + 1;
    let close = closing_line + block[closing_line..].find('`')?;

    (closing_line > open_end && block[close..].bytes().all(|byte| byte == b'`')).then(|| [start + open..start + open_end, start + close..start + block.len()])
}

/// Every fenced code block, in document order, at any container depth.
fn collect_fences<'a>(blocks: &'a [Block<'a>], out: &mut Vec<&'a CodeBlock<'a>>) {
    for block in blocks {
        match block {
            Block::CodeBlock(code) => out.push(code),
            Block::Blockquote(quote) => collect_fences(&quote.children, out),
            Block::List(list) => {
                for item in &list.children {
                    collect_fences(&item.children, out);
                }
            }
            Block::FootnoteDefinition(definition) => collect_fences(&definition.children, out),
            Block::ContainerDirective(directive) => collect_fences(&directive.children, out),
            Block::MdxJsx(element) => collect_fences(&element.children, out),
            _ => {}
        }
    }
}

/// Embedded code never carries surrounding blank lines; oxc_formatter lays
/// out the template around it.
fn trim(text: &str) -> &str {
    text.trim_start_matches('\n').trim_end()
}

/// `text` as IR: a `Text` per line joined by hard lines, so the JS printer
/// indents every line to the template's level. A blank line is an empty line,
/// which prints without trailing indentation.
pub(super) fn into_ir<'a>(session: &FormatSession<'a>, text: &str, indent_width: IndentWidth) -> ArenaVec<'a, FormatElement<'a>> {
    let allocator = session.allocator();
    let mut ir = ArenaVec::new_in(&allocator);

    for (index, line) in text.split('\n').enumerate() {
        if index > 0 {
            ir.push(FormatElement::Line(if line.is_empty() { LineMode::Empty } else { LineMode::Hard }));
        }

        if !line.is_empty() {
            let line = allocator.alloc_str(line);

            ir.push(FormatElement::Text { text: line, width: TextWidth::from_text(line, indent_width) });
        }
    }

    ir
}

/// Elements that never have content or a closing tag.
const VOID: [&str; 14] = ["area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source", "track", "wbr"];

/// Elements whose content is raw text, never tags.
const RAW: [&str; 4] = ["script", "style", "textarea", "title"];

/// Whether `html` has more than one root node once whitespace-only text is
/// dropped: Prettier's `htmlHasMultipleRootElements`, which decides whether a
/// template hugging its backticks indents its content. A scan of tags rather
/// than a parse, since markup_fmt keeps its tree private.
pub(super) fn has_multiple_roots(html: &str) -> bool {
    let html = html.to_ascii_lowercase();
    let mut rest = html.as_str();
    let mut roots = 0;
    let mut depth = 0_usize;
    let mut in_text = false;

    while let Some(first) = rest.chars().next() {
        let tag = rest.strip_prefix('<').filter(|after| after.starts_with(|c: char| c.is_ascii_alphabetic() || c == '/' || c == '!'));

        let Some(after) = tag else {
            // Text runs to the next `<`; a stray `<` stays in the same run.
            let end = rest[first.len_utf8()..].find('<').map_or(rest.len(), |at| at + first.len_utf8());

            if depth == 0 && !in_text && !rest[..end].trim().is_empty() {
                roots += 1;
                in_text = true;
            }

            rest = &rest[end..];

            continue;
        };

        in_text = false;

        if let Some(comment) = after.strip_prefix("!--") {
            rest = comment.find("-->").map_or("", |end| &comment[end + 3..]);
            roots += usize::from(depth == 0);
        } else if let Some(closing) = after.strip_prefix('/') {
            rest = closing.find('>').map_or("", |end| &closing[end + 1..]);
            depth = depth.saturating_sub(1);
        } else {
            let name_end = after.find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '-' | ':' | '!'))).unwrap_or(after.len());
            let name = &after[..name_end];
            let tag_end = tag_end(&after[name_end..]).map_or(after.len(), |end| name_end + end);
            let self_closing = after[..tag_end].ends_with("/>");

            roots += usize::from(depth == 0);
            rest = &after[tag_end..];

            if name.starts_with('!') || self_closing || VOID.contains(&name) {
                // No content.
            } else if RAW.contains(&name) {
                let closing = format!("</{name}");

                rest = rest.find(&closing).map_or("", |at| rest[at..].find('>').map_or("", |end| &rest[at + end + 1..]));
            } else {
                depth += 1;
            }
        }

        if roots > 1 {
            return true;
        }
    }

    roots > 1
}

/// The length of a start tag's remainder up to and including its `>`,
/// skipping quoted attribute values.
fn tag_end(rest: &str) -> Option<usize> {
    let mut quote = None;

    for (index, c) in rest.char_indices() {
        match (quote, c) {
            (None, '"' | '\'') => quote = Some(c),
            (Some(open), _) if c == open => quote = None,
            (None, '>') => return Some(index + 1),
            _ => {}
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::{has_multiple_roots, tilde_fences};

    #[test]
    fn counts_root_nodes_like_prettier() {
        assert!(!has_multiple_roots("<div><h1>a</h1><p>b</p></div>"));
        assert!(!has_multiple_roots("\n  <ul>\n    <li>a</li>\n  </ul>\n"));
        assert!(!has_multiple_roots("<input type=\"a>b\">"));
        assert!(!has_multiple_roots("<style>p > a {}</style>"));
        assert!(!has_multiple_roots("text with a < sign"));
        assert!(has_multiple_roots("<h1>a</h1><p>b</p>"));
        assert!(has_multiple_roots("<br><br>"));
        assert!(has_multiple_roots("<p>a</p> tail"));
        assert!(has_multiple_roots("<!-- note --><p>a</p>"));
    }

    #[test]
    fn fences_use_tildes_longer_than_any_tilde_run() {
        assert_eq!(tilde_fences("# a\n\n```js\nx\n```"), "# a\n\n~~~js\nx\n~~~");
        assert_eq!(tilde_fences("- a\n\n  ````\n  ~~~~\n  ```\n  ````"), "- a\n\n  ~~~~~\n  ~~~~\n  ```\n  ~~~~~");
        assert_eq!(tilde_fences("text `code`"), "text `code`");
    }
}
