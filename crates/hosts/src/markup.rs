//! Vue single-file components and HTML through markup_fmt. Its external
//! formatter callback routes scripts to the TS pipeline and styles to
//! oxc_formatter_css; everything else stays as written.

use std::borrow::Cow;

use fmtkit_config::TsFormat;
use fmtkit_core::{Lang, LineIndex};
use fmtkit_ts::TsError;
use markup_fmt::config::{FormatOptions, LanguageOptions, LayoutOptions, LineBreak, Quotes};
use markup_fmt::{FormatError, Hints, Language};

use crate::embed::Embedded;
use crate::{HostError, ScriptFormatter, css, raw, same_code, syntax_error};

/// Format a Vue or HTML document.
pub(crate) fn format(lang: Lang, text: &str, options: &TsFormat, scripts: ScriptFormatter<'_>, applied: &mut Vec<&'static str>) -> Result<String, HostError> {
    let language = if lang == Lang::Vue { Language::Vue } else { Language::Html };
    let masked = raw::mask(lang, text);
    let input = masked.as_ref().map_or(text, |masked| masked.text.as_str());
    let original = |offset: usize| masked.as_ref().map_or(offset, |masked| masked.original_offset(offset));
    let mut router = Router::new(text, input, &original, options, scripts);

    let result = markup_fmt::format_text(input, language, &markup_options(options), |code, hints| router.route(code, &hints));

    match result {
        Ok(output) => {
            applied.extend(router.applied);

            // A placeholder only goes missing if markup_fmt dropped a block;
            // leave the document as written rather than lose it.
            Ok(match &masked {
                Some(masked) => masked.restore(&output).unwrap_or_else(|| text.to_owned()),
                None => output,
            })
        }
        Err(FormatError::Syntax(error)) => Err(syntax_error(text, original(error.pos), error.kind.to_string())),
        Err(FormatError::External(errors)) => Err(router.failure.unwrap_or_else(|| HostError::Syntax {
            line: 1,
            column: 1,
            message: errors.first().map_or_else(|| "embedded code failed".to_owned(), ToString::to_string),
        })),
    }
}

/// Prettier's markup defaults on top of the shared layout: double-quoted
/// attributes, indented `<script>` and `<style>` bodies in HTML but not in Vue
/// (`vueIndentScriptAndStyle: false`), and tags, attribute styles and
/// component names left as written.
fn markup_options(options: &TsFormat) -> FormatOptions {
    FormatOptions {
        layout: LayoutOptions {
            print_width: usize::from(options.print_width),
            use_tabs: options.use_tabs,
            indent_width: usize::from(options.tab_width),
            line_break: LineBreak::Lf,
        },
        language: LanguageOptions {
            quotes: Quotes::Double,
            html_script_indent: Some(true),
            html_style_indent: Some(true),
            vue_script_indent: Some(false),
            vue_style_indent: Some(false),
            ..LanguageOptions::default()
        },
    }
}

/// The state behind markup_fmt's external formatter callback.
struct Router<'s> {
    /// The document as written, for error lines.
    text: &'s str,
    /// The text markup_fmt formats, with raw blocks masked.
    input: &'s str,
    /// Maps an offset in `input` back to `text`.
    original: &'s dyn Fn(usize) -> usize,
    options: &'s TsFormat,
    /// Options for expressions inside double-quoted attributes, where single
    /// quotes avoid `&quot;` escapes whatever `single_quote` says.
    attribute_options: TsFormat,
    scripts: ScriptFormatter<'s>,
    applied: Vec<&'static str>,
    failure: Option<HostError>,
}

impl<'s> Router<'s> {
    fn new(text: &'s str, input: &'s str, original: &'s dyn Fn(usize) -> usize, options: &'s TsFormat, scripts: ScriptFormatter<'s>) -> Self {
        let attribute_options = TsFormat { single_quote: true, ..options.clone() };

        Self { text, input, original, options, attribute_options, scripts, applied: Vec::new(), failure: None }
    }

    fn route<'a>(&mut self, code: &'a str, hints: &Hints<'_>) -> Result<Cow<'a, str>, anyhow::Error> {
        match Embedded::from_tag(hints.ext) {
            Some(Embedded::Script(lang)) => self.script(code, lang, hints.attr),
            Some(Embedded::Style(variant)) => Ok(self.style(code, variant, hints)),
            None => Ok(Cow::Borrowed(code)),
        }
    }

    /// A whole `<script>` body goes through the TS pipeline and fails the
    /// document when it cannot be formatted. A template expression, binding or
    /// attribute that fails is left as written.
    fn script<'a>(&mut self, code: &'a str, lang: Lang, attribute: bool) -> Result<Cow<'a, str>, anyhow::Error> {
        let block = self.script_block(code);
        let options = if attribute { &self.attribute_options } else { self.options };

        match (self.scripts)(lang, code, options) {
            Ok(output) if output == code => Ok(Cow::Borrowed(code)),
            Ok(output) => {
                if block.is_some() && !same_code(&output, code) {
                    self.mark("embedded");
                }

                Ok(Cow::Owned(output))
            }
            Err(error) => match block {
                Some(offset) => {
                    let failure = self.embedded_error(lang, offset, &error);
                    let message = failure.to_string();

                    self.failure.get_or_insert(failure);

                    Err(anyhow::Error::msg(message))
                }
                None => Ok(Cow::Borrowed(code)),
            },
        }
    }

    fn style<'a>(&mut self, code: &'a str, variant: oxc_formatter_css::CssVariant, hints: &Hints<'_>) -> Cow<'a, str> {
        let formatted = if hints.attr { css::format_declarations(code, self.options) } else { css::format(code, variant, self.options, hints.print_width) };

        match formatted {
            Some(output) => {
                if !same_code(&output, code) {
                    self.mark("css");
                }

                Cow::Owned(output)
            }
            None => Cow::Borrowed(code),
        }
    }

    /// The offset of `code` in the masked input when it is the whole body of a
    /// `<script>` element rather than an expression markup_fmt built.
    fn script_block(&self, code: &str) -> Option<usize> {
        let offset = code.as_ptr().addr().checked_sub(self.input.as_ptr().addr())?;
        let end = offset.checked_add(code.len())?;

        self.input.get(offset..end)?;

        let closing = self.input.get(end..)?.as_bytes();

        closing.get(..8).is_some_and(|tag| tag.eq_ignore_ascii_case(b"</script")).then_some(offset)
    }

    fn embedded_error(&self, lang: Lang, offset: usize, error: &TsError) -> HostError {
        let start = LineIndex::new(self.text).line(u32::try_from((self.original)(offset)).unwrap_or(u32::MAX));

        let (line, message) = match error {
            TsError::Syntax { line, message, .. } => (start + line.saturating_sub(1), message.clone()),
            TsError::Invariant { .. } => (start, error.to_string()),
        };

        HostError::Embedded { lang, line, message }
    }

    fn mark(&mut self, step: &'static str) {
        if !self.applied.contains(&step) {
            self.applied.push(step);
        }
    }
}
