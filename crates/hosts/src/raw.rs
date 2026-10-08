//! Blocks markup_fmt must not touch. It parses a Vue custom block or a
//! `<template lang="pug">` as markup and collapses its whitespace, and it
//! re-indents the body of a `<script>` or `<style>` it could not format,
//! which breaks indentation-sensitive languages such as CoffeeScript or
//! Stylus. Their bodies are swapped for a placeholder before formatting and
//! put back afterwards, verbatim apart from surrounding blank lines.

use fmtkit_core::Lang;

use crate::embed::Embedded;

const TOKEN: &str = "__fmtkit_raw_block_";

/// A document with its raw bodies replaced by placeholders.
pub(crate) struct Masked {
    pub(crate) text: String,
    bodies: Vec<String>,
    /// Per placeholder: its offset in `text`, its length, and the length of
    /// the body it replaced.
    swaps: Vec<(usize, usize, usize)>,
}

/// Mask the raw bodies of `text`. `None` when there are none, or when the
/// document already contains the placeholder text.
pub(crate) fn mask(lang: Lang, text: &str) -> Option<Masked> {
    if text.contains(TOKEN) {
        return None;
    }

    let elements = if lang == Lang::Vue { top_level_blocks(text) } else { raw_text_elements(text) };
    let mut masked = String::with_capacity(text.len());
    let mut bodies = Vec::new();
    let mut swaps = Vec::new();
    let mut cursor = 0;

    for element in elements.into_iter().filter(|element| element.is_raw(lang)) {
        let Some(body) = normalize(&text[element.body.clone()]) else {
            continue;
        };

        let token = placeholder(bodies.len());

        masked.push_str(&text[cursor..element.body.start]);
        swaps.push((masked.len(), token.len(), element.body.len()));
        masked.push_str(&token);
        cursor = element.body.end;
        bodies.push(body.to_owned());
    }

    if bodies.is_empty() {
        return None;
    }

    masked.push_str(&text[cursor..]);

    Some(Masked { text: masked, bodies, swaps })
}

impl Masked {
    /// The offset in the original document of `offset` in the masked text.
    pub(crate) fn original_offset(&self, offset: usize) -> usize {
        let mut shifted = offset;

        for &(start, token, body) in &self.swaps {
            if offset < start {
                break;
            }

            shifted = if offset < start + token { shifted - (offset - start) } else { shifted - token + body };
        }

        shifted
    }

    /// Put the bodies back into the formatted text. `None` when a placeholder
    /// went missing.
    pub(crate) fn restore(&self, formatted: &str) -> Option<String> {
        let mut out = formatted.to_owned();

        for (index, body) in self.bodies.iter().enumerate() {
            let token = placeholder(index);
            let at = out.find(&token)?;
            let open = out[..at].rfind('>')? + 1;
            let close = at + token.len() + out[at + token.len()..].find('<')?;
            let line = out[..close].rfind('\n').map_or(0, |i| i + 1);
            let end = if line > open && out[line..close].trim().is_empty() { line } else { close };

            out.replace_range(open..end, &format!("\n{body}\n"));
        }

        Some(out)
    }
}

fn placeholder(index: usize) -> String {
    format!("{TOKEN}{index}__")
}

/// The body without its leading blank lines and trailing whitespace, or
/// `None` when it is blank. The first line keeps its indentation.
fn normalize(body: &str) -> Option<&str> {
    let first = body.find(|c: char| !c.is_whitespace())?;
    let start = body[..first].rfind('\n').map_or(0, |i| i + 1);

    Some(body[start..].trim_end())
}

/// An element whose body is kept as raw text.
#[derive(Debug)]
struct Element<'s> {
    name: &'s str,
    open_tag: &'s str,
    body: std::ops::Range<usize>,
}

impl Element<'_> {
    fn is_raw(&self, lang: Lang) -> bool {
        let tag_lang = attribute(self.open_tag, "lang");

        if self.name.eq_ignore_ascii_case("script") {
            return tag_lang.is_some_and(|value| !matches!(Embedded::from_tag(&value), Some(Embedded::Script(_))));
        }

        if self.name.eq_ignore_ascii_case("style") {
            return tag_lang.is_some_and(|value| !matches!(Embedded::from_tag(&value), Some(Embedded::Style(_))));
        }

        if self.name.eq_ignore_ascii_case("template") {
            return lang == Lang::Vue && tag_lang.is_some_and(|value| value != "html");
        }

        // Any other top-level block of a Vue component is a custom block.
        lang == Lang::Vue
    }
}

/// The top-level blocks of a Vue component. `<template>` nests; every other
/// block ends at its first closing tag, as in the Vue SFC parser.
fn top_level_blocks(text: &str) -> Vec<Element<'_>> {
    let mut elements = Vec::new();
    let mut cursor = 0;

    while let Some(offset) = text[cursor..].find('<') {
        let start = cursor + offset;
        let rest = &text[start..];

        if rest.starts_with("<!--") {
            cursor = rest.find("-->").map_or(text.len(), |end| start + end + 3);

            continue;
        }

        let Some((name, open_end)) = open_tag(text, start) else {
            cursor = start + 1;

            continue;
        };

        let open_tag = &text[start..open_end];

        if open_tag.ends_with("/>") {
            cursor = open_end;

            continue;
        }

        let close = if name.eq_ignore_ascii_case("template") { closing_template(text, open_end) } else { find_closing(text, open_end, name) };

        let Some(close) = close else {
            break;
        };

        elements.push(Element { name, open_tag, body: open_end..close });
        cursor = close;
    }

    elements
}

/// Every `<script>` and `<style>` element, anywhere in an HTML document.
fn raw_text_elements(text: &str) -> Vec<Element<'_>> {
    let mut elements = Vec::new();
    let mut cursor = 0;

    while let Some(offset) = text[cursor..].find('<') {
        let start = cursor + offset;

        let Some((name, open_end)) = open_tag(text, start).filter(|(name, _)| name.eq_ignore_ascii_case("script") || name.eq_ignore_ascii_case("style")) else {
            cursor = start + 1;

            continue;
        };

        let Some(close) = find_closing(text, open_end, name) else {
            break;
        };

        elements.push(Element { name, open_tag: &text[start..open_end], body: open_end..close });
        cursor = close;
    }

    elements
}

/// The tag name of an opening tag at `start` and the offset just past its
/// `>`, skipping `>` inside quoted attribute values.
fn open_tag(text: &str, start: usize) -> Option<(&str, usize)> {
    let bytes = text.as_bytes();
    let name_start = start + 1;
    let name_len = text[name_start..].find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ':' || c == '.'))?;

    if name_len == 0 || !bytes[name_start].is_ascii_alphabetic() {
        return None;
    }

    let mut quote = None;

    for (index, &byte) in bytes.iter().enumerate().skip(name_start + name_len) {
        match (quote, byte) {
            (Some(q), _) if q == byte => quote = None,
            (None, b'"' | b'\'') => quote = Some(byte),
            (None, b'>') => return Some((&text[name_start..name_start + name_len], index + 1)),
            (None, b'<') => return None,
            _ => {}
        }
    }

    None
}

/// The offset of the first `</name` at or after `from`, case-insensitively.
fn find_closing(text: &str, from: usize, name: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut cursor = from;

    while let Some(offset) = text[cursor..].find("</") {
        let at = cursor + offset;
        let candidate = bytes.get(at + 2..at + 2 + name.len())?;

        if candidate.eq_ignore_ascii_case(name.as_bytes()) && bytes.get(at + 2 + name.len()).is_none_or(|b| !b.is_ascii_alphanumeric() && *b != b'-') {
            return Some(at);
        }

        cursor = at + 2;
    }

    None
}

/// The `</template>` that closes the template opened before `from`.
fn closing_template(text: &str, from: usize) -> Option<usize> {
    let mut depth = 1usize;
    let mut cursor = from;

    loop {
        let close = find_closing(text, cursor, "template")?;
        let nested = text[cursor..close]
            .match_indices('<')
            .filter_map(|(i, _)| open_tag(text, cursor + i))
            .filter(|(name, end)| name.eq_ignore_ascii_case("template") && !text[..*end].ends_with("/>"))
            .count();

        depth += nested;
        depth -= 1;

        if depth == 0 {
            return Some(close);
        }

        cursor = close + 2;
    }
}

/// An attribute value of an opening tag, lower-cased; `None` when absent or
/// valueless. Quoted and bare values, names matched case-insensitively.
pub(crate) fn attribute(open_tag: &str, name: &str) -> Option<String> {
    let inner = open_tag.strip_prefix('<')?.trim_end_matches('>').trim_end_matches('/');
    let mut rest = inner.trim_start_matches(|c: char| !c.is_whitespace());

    loop {
        rest = rest.trim_start();

        if rest.is_empty() {
            return None;
        }

        let name_end = rest.find(|c: char| c.is_whitespace() || c == '=').unwrap_or(rest.len());
        let attribute_name = &rest[..name_end];

        rest = rest[name_end..].trim_start();

        let value = if let Some(after) = rest.strip_prefix('=') {
            let after = after.trim_start();

            let (value, remaining) = if let Some(quote @ ('"' | '\'')) = after.chars().next() {
                let end = after[1..].find(quote).map_or(after.len(), |i| i + 1);

                (&after[1..end], after.get(end + 1..).unwrap_or(""))
            } else {
                after.split_at(after.find(char::is_whitespace).unwrap_or(after.len()))
            };

            rest = remaining;

            Some(value)
        } else {
            None
        };

        if attribute_name.eq_ignore_ascii_case(name) {
            return value.map(str::to_ascii_lowercase);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attribute_reads_quoted_and_bare_values_case_insensitively() {
        assert_eq!(attribute("<script LANG=\"TS\">", "lang").as_deref(), Some("ts"));
        assert_eq!(attribute("<script lang='tsx'>", "lang").as_deref(), Some("tsx"));
        assert_eq!(attribute("<script lang=jsx>", "lang").as_deref(), Some("jsx"));
        assert_eq!(attribute("<script setup>", "lang"), None);
        assert_eq!(attribute("<script setup lang = \"ts\" >", "lang").as_deref(), Some("ts"));
        assert_eq!(attribute("<script data-lang=\"x\" lang=\"ts\">", "lang").as_deref(), Some("ts"));
        assert_eq!(attribute("<style scoped lang=\"stylus\"/>", "lang").as_deref(), Some("stylus"));
    }

    #[test]
    fn vue_top_level_blocks_nest_templates_and_skip_comments() {
        let text = "<!-- <i18n> -->\n<template>\n<template v-if=\"a\"><p/></template>\n<br/>\n</template>\n<script setup>\nconst a = '</template>';\n</script>\n<i18n lang=\"yaml\">\na: 1\n</i18n>\n";
        let blocks = top_level_blocks(text);
        let names: Vec<&str> = blocks.iter().map(|b| b.name).collect();

        assert_eq!(names, ["template", "script", "i18n"]);
        assert_eq!(&text[blocks[0].body.clone()], "\n<template v-if=\"a\"><p/></template>\n<br/>\n");
        assert_eq!(&text[blocks[2].body.clone()], "\na: 1\n");
    }

    #[test]
    fn only_unformatted_languages_are_raw() {
        let raw = |lang, tag: &str| Element { name: &tag[1..tag.find([' ', '>']).unwrap()], open_tag: tag, body: 0..0 }.is_raw(lang);

        assert!(raw(Lang::Vue, "<script lang=\"coffee\">"));
        assert!(!raw(Lang::Vue, "<script lang=\"ts\">"));
        assert!(!raw(Lang::Vue, "<script setup>"));
        assert!(raw(Lang::Vue, "<style lang=\"stylus\">"));
        assert!(raw(Lang::Vue, "<style lang=\"sass\">"));
        assert!(!raw(Lang::Vue, "<style lang=\"scss\">"));
        assert!(!raw(Lang::Vue, "<style scoped>"));
        assert!(raw(Lang::Vue, "<template lang=\"pug\">"));
        assert!(!raw(Lang::Vue, "<template>"));
        assert!(raw(Lang::Vue, "<docs>"));
        assert!(!raw(Lang::Html, "<template lang=\"pug\">"));
        assert!(raw(Lang::Html, "<style lang=\"stylus\">"));
    }

    #[test]
    fn mask_and_restore_round_trip() {
        let text = "<template>\n  <p>x</p>\n</template>\n\n<i18n lang=\"yaml\">\n\n  en:\n    a:   1\n\n</i18n>\n";
        let masked = mask(Lang::Vue, text).unwrap();

        assert_eq!(masked.text, "<template>\n  <p>x</p>\n</template>\n\n<i18n lang=\"yaml\">__fmtkit_raw_block_0__</i18n>\n");
        assert_eq!(masked.restore("<i18n lang=\"yaml\">\n\t__fmtkit_raw_block_0__\n</i18n>\n").unwrap(), "<i18n lang=\"yaml\">\n  en:\n    a:   1\n</i18n>\n");
        assert_eq!(masked.restore("<i18n lang=\"yaml\">__fmtkit_raw_block_0__</i18n>\n").unwrap(), "<i18n lang=\"yaml\">\n  en:\n    a:   1\n</i18n>\n");
        assert!(masked.restore("<i18n></i18n>").is_none());

        let token = "<i18n lang=\"yaml\">".len() + text.find("<i18n").unwrap();

        assert_eq!(masked.original_offset(3), 3);
        assert_eq!(masked.original_offset(token + 4), token);
        assert_eq!(masked.original_offset(masked.text.len() - 1), text.len() - 1);
    }

    #[test]
    fn nothing_to_mask() {
        assert!(mask(Lang::Vue, "<template><p/></template>\n<script>\na\n</script>\n").is_none());
        assert!(mask(Lang::Vue, "<docs>   </docs>\n").is_none());
        assert!(mask(Lang::Vue, "<docs>__fmtkit_raw_block_0__</docs>\n").is_none());
    }
}
