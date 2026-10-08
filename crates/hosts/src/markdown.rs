//! Markdown through oxc_formatter_markdown, which prints fenced code
//! verbatim. Script and style fences are formatted afterwards on the printed
//! document, spliced back under their container prefixes (list indent,
//! `> `), and the document is printed once more so fence lengths settle.

use fmtkit_config::TsFormat;
use oxc_allocator::Allocator;
use oxc_formatter_core::LineEnding;
use oxc_formatter_markdown::{MarkdownFormatOptions, ProseWrap};
use oxc_markdown_parser::Segment;
use oxc_markdown_parser::ast::{Block, CodeBlock, CodeBlockKind};

use crate::embed::Embedded;
use crate::{HostError, ScriptFormatter, css, syntax_error};

/// Format a Markdown document.
pub(crate) fn format(text: &str, options: &TsFormat, scripts: ScriptFormatter<'_>, applied: &mut Vec<&'static str>) -> Result<String, HostError> {
    let printed = print(text, options)?;

    if printed != text {
        applied.push("markdown");
    }

    let Some((spliced, steps)) = format_fences(&printed, options, scripts) else {
        return Ok(printed);
    };

    // The fences were printed by this formatter, so a failure here can only
    // come from the spliced code; keep the document without it.
    let Ok(reprinted) = print(&spliced, options) else {
        return Ok(printed);
    };

    applied.extend(steps);

    if reprinted != spliced && !applied.contains(&"markdown") {
        applied.push("markdown");
    }

    Ok(reprinted)
}

fn print(text: &str, options: &TsFormat) -> Result<String, HostError> {
    let allocator = Allocator::default();

    let formatted = oxc_formatter_markdown::format(&allocator, text, markdown_options(options)).map_err(|diagnostic| {
        let offset = diagnostic.labels.first().map_or(0, |label| usize::try_from(label.offset()).unwrap_or(usize::MAX));

        syntax_error(text, offset, diagnostic.message.to_string())
    })?;

    formatted.print().map(oxc_formatter_core::Printed::into_code).map_err(|error| syntax_error(text, 0, error.to_string()))
}

/// Prettier's Markdown defaults (`proseWrap: "preserve"`) on the shared layout.
fn markdown_options(options: &TsFormat) -> MarkdownFormatOptions {
    MarkdownFormatOptions {
        indent_style: css::indent_style(options),
        indent_width: css::indent_width(options),
        line_width: css::line_width(usize::from(options.print_width)),
        line_ending: LineEnding::Lf,
        prose_wrap: ProseWrap::Preserve,
        single_quote: options.single_quote.into(),
    }
}

/// One fence to replace: the byte range from its opening run to the end of its
/// closing run, and the text that takes its place.
struct Splice {
    start: usize,
    end: usize,
    body: String,
}

/// Format every script and style fence of a printed document. `None` when no
/// fence changed; otherwise the new text and the steps that changed it.
fn format_fences(printed: &str, options: &TsFormat, scripts: ScriptFormatter<'_>) -> Option<(String, Vec<&'static str>)> {
    let allocator = Allocator::default();
    let parsed = oxc_formatter_markdown::parse_for_format(&allocator, printed).ok()?;
    let source = parsed.source;

    // The parser sees the document without its byte order mark.
    let base = printed.len().checked_sub(source.len())?;
    let mut fences = Vec::new();

    collect_fences(&parsed.root.children, &mut fences);

    let mut splices = Vec::new();
    let mut steps = Vec::new();

    for fence in fences {
        let CodeBlockKind::Fenced { lang: Some(lang), .. } = fence.kind else {
            continue;
        };

        let Some(embedded) = Embedded::from_tag(lang.slice(source)) else {
            continue;
        };

        let content = Segment::join(source, &fence.lines);

        if content.trim().is_empty() {
            continue;
        }

        let formatted = match embedded {
            Embedded::Script(lang) => scripts(lang, &content, options).ok(),
            Embedded::Style(variant) => css::format(&content, variant, options, usize::from(options.print_width)),
        };

        let Some(formatted) = formatted else {
            continue;
        };

        let formatted = formatted.trim_end_matches('\n');

        if formatted == content || formatted.trim().is_empty() {
            continue;
        }

        let Some(splice) = splice(source, fence, formatted) else {
            continue;
        };

        let step = if matches!(embedded, Embedded::Script(_)) { "embedded" } else { "css" };

        if !steps.contains(&step) {
            steps.push(step);
        }

        splices.push(splice);
    }

    if splices.is_empty() {
        return None;
    }

    // Copy from the printed text, not `source`, whose front matter is blanked.
    let (bom, body) = printed.split_at(base);
    let mut out = String::with_capacity(printed.len());
    let mut cursor = 0;

    out.push_str(bom);

    for splice in &splices {
        out.push_str(&body[cursor..splice.start]);
        out.push_str(&splice.body);
        cursor = splice.end;
    }

    out.push_str(&body[cursor..]);

    Some((out, steps))
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

/// The printed fence rebuilt around `formatted`: content lines under the
/// container prefix its closing fence line carries, and both fence runs
/// lengthened when the code holds a run as long. `None` when the printed
/// fence does not have the expected shape.
fn splice(source: &str, fence: &CodeBlock<'_>, formatted: &str) -> Option<Splice> {
    let span_start = fence.span.start as usize;
    let span = source.get(span_start..fence.span.end as usize)?;
    let open_line = &span[..span.find('\n')?];
    let block = span.trim_end_matches('\n');
    let close = span_start + block.rfind('\n')? + 1;
    let closing_line = source.get(close..span_start + block.len())?;
    let marker = closing_line.find(['`', '~'])?;
    let (prefix, run) = closing_line.split_at(marker);
    let fence_char = run.chars().next()?;
    let open = open_line.find(fence_char)?;
    let info = open_line[open..].trim_start_matches(fence_char);

    if span_start + open_line.len() + 1 > close || run.len() < 3 || !run.chars().all(|c| c == fence_char) {
        return None;
    }

    let longest = formatted.split(|c| c != fence_char).map(str::len).max().unwrap_or(0);
    let run = fence_char.to_string().repeat(run.len().max(longest + 1));
    let mut body = String::with_capacity(formatted.len() + prefix.len() * 8 + run.len() * 2 + info.len() + 2);

    body.push_str(&run);
    body.push_str(info);
    body.push('\n');

    for line in formatted.split('\n') {
        if line.is_empty() {
            body.push_str(prefix.trim_end());
        } else {
            body.push_str(prefix);
            body.push_str(line);
        }

        body.push('\n');
    }

    body.push_str(prefix);
    body.push_str(&run);

    Some(Splice { start: span_start + open, end: span_start + block.len(), body })
}
