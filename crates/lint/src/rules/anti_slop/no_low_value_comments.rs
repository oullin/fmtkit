//! `anti-slop/no-low-value-comments`: no comment that only repeats the code,
//! divides a file into sections, labels test steps, or keeps dead code.
//! Adapted from t3code's `t3code/no-low-value-comments`.

use std::sync::LazyLock;

use oxc_ast::ast::Comment;
use oxc_span::Span;
use regress::Regex;

use super::super::{Context, Rule};
use super::shared::{comment_value, links_issue};

pub const NAME: &str = "anti-slop/no-low-value-comments";

fn regex(pattern: &str, flags: &str) -> Regex {
    Regex::with_flags(pattern, flags).expect("a valid pattern")
}

/// Comments that instruct a tool rather than a reader.
static DIRECTIVE: LazyLock<Regex> = LazyLock::new(|| {
    regex(
        r"^\s*(?:!|(?:oxlint|eslint|tslint|deno-lint)-|@ts-|@effect-diagnostics|prettier-ignore|biome-ignore|c8 |v8 |istanbul |#(?:end)?region|[@#]__(?:PURE|NO_SIDE_EFFECTS)__|@vite-ignore|webpack[A-Z]|@jsx|@license|@preserve|@vitest-environment|@jest-environment|\/?\s*<reference\s)",
        "u",
    )
});
static DIVIDER_LINE: LazyLock<Regex> = LazyLock::new(|| regex(r"^(?:[-=*#~_/+]{3,}.*|.*[-=*#~_]{3,})$", "u"));
static TEST_STEP: LazyLock<Regex> = LazyLock::new(|| regex(r"^(?:arrange|act|assert|given|when|then)\s*[:.]?$", "iu"));
static TASK_MARKER: LazyLock<Regex> = LazyLock::new(|| regex(r"\b(?:TODO|FIXME|XXX|HACK)\b", "u"));
static JSDOC_TYPE: LazyLock<Regex> = LazyLock::new(|| regex(r"^@(?:param|returns?|type|typedef)\s+\{", "u"));
static EMPTY_JSDOC_TAG: LazyLock<Regex> = LazyLock::new(|| regex(r"^@(?:param(?:\s+\[?[\w$.]+\]?)?|returns?)$", "u"));
static COMMENTED_CODE: LazyLock<Regex> = LazyLock::new(|| {
    regex(
        r#"^(?:(?:(?:const|let|var)\s+[\w${}[\], ]+=.*|import\s.+\sfrom\s+["'].+["']|(?:await\s+|return\s+)?[\w$.]+\(.*\)|[\w$.[\]]+\s*=\s*.+);|(?:if|for|while|switch)\s*\(.*\)\s*\{|\}(?:\s*else\s*\{)?)$"#,
        "u",
    )
});
static DECLARATION: LazyLock<Regex> = LazyLock::new(|| {
    regex(
        r"^\s*(?:export\s+)?(?:default\s+)?(?:declare\s+)?(?:async\s+)?(?:static\s+)?(?:readonly\s+)?(?:(?:const|let|var|function\*?|class|interface|type|enum)\s+)?([A-Za-z_$][\w$]*)\s*[?!]?\s*[:=(<{]",
        "u",
    )
});

/// Words that carry no meaning of their own when a comment is compared with a name.
const FILLER_WORDS: [&str; 30] = [
    "a", "an", "the", "of", "for", "to", "in", "on", "at", "by", "with", "from", "and", "or", "is", "are", "be", "this", "that", "it", "its", "as", "get",
    "set", "return", "create", "new", "current", "value", "given",
];

/// Statement keywords the declaration pattern would otherwise read as a name.
const KEYWORDS: [&str; 6] = ["catch", "for", "if", "return", "switch", "while"];

pub struct NoLowValueComments;

impl Rule for NoLowValueComments {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run_once(&self, ctx: &mut Context<'_, '_>) {
        let semantic = ctx.semantic;
        let source = semantic.source_text();
        let lines = Lines::new(source);
        let typescript = semantic.source_type().is_typescript();
        let groups = groups(source, &lines, semantic.comments());

        for group in groups {
            let text: Vec<&str> = group.iter().map(|comment| comment_value(source, comment)).collect();
            let Some(message) = problem(source, &lines, typescript, &group, &text) else {
                continue;
            };

            ctx.report(Span::new(group[0].span.start, group[group.len() - 1].span.end), message);
        }
    }
}

/// Line starts, for line numbers and columns by byte offset.
struct Lines(Vec<usize>);

impl Lines {
    fn new(source: &str) -> Self {
        Self(std::iter::once(0).chain(source.match_indices('\n').map(|(at, _)| at + 1)).collect())
    }

    /// The zero-based line holding `offset`.
    fn line(&self, offset: u32) -> usize {
        self.0.partition_point(|&start| start <= offset as usize) - 1
    }

    fn column(&self, offset: u32) -> usize {
        offset as usize - self.0[self.line(offset)]
    }

    /// The text of zero-based line `line`, without its line break.
    fn text<'s>(&self, source: &'s str, line: usize) -> Option<&'s str> {
        let start = *self.0.get(line)?;
        let end = self.0.get(line + 1).map_or(source.len(), |next| next - 1);

        Some(source[start..end].trim_end_matches('\r'))
    }
}

/// Comments as a reader sees them: consecutive `//` lines at one indent are one comment.
fn groups<'c>(source: &str, lines: &Lines, comments: &'c [Comment]) -> Vec<Vec<&'c Comment>> {
    let mut groups: Vec<Vec<&Comment>> = Vec::new();

    for comment in comments {
        let continues = groups.last().and_then(|group| group.last()).is_some_and(|previous| {
            comment.is_line()
                && previous.is_line()
                && lines.line(previous.span.end) + 1 == lines.line(comment.span.start)
                && lines.column(previous.span.start) == lines.column(comment.span.start)
                && source[previous.span.end as usize..comment.span.start as usize].trim().is_empty()
        });

        match groups.last_mut() {
            Some(group) if continues => group.push(comment),
            _ => groups.push(vec![comment]),
        }
    }

    groups
}

/// The comment's lines without their `*` gutter, blank ones dropped.
fn content_lines<'s>(text: &[&'s str]) -> Vec<&'s str> {
    text.iter()
        .flat_map(|value| value.split('\n'))
        .map(|line| {
            let line = line.trim_start();
            let line = line.strip_prefix('*').unwrap_or(line);

            line.strip_prefix([' ', '\t']).unwrap_or(line).trim()
        })
        .filter(|line| !line.is_empty())
        .collect()
}

fn matches(regex: &Regex, text: &str) -> bool {
    regex.find(text).is_some()
}

/// What is wrong with the comment group, if anything.
fn problem(source: &str, lines: &Lines, typescript: bool, group: &[&Comment], text: &[&str]) -> Option<&'static str> {
    let content = content_lines(text);

    if content.is_empty() || text.iter().any(|value| matches(&DIRECTIVE, value)) {
        return None;
    }

    let is_line = group[0].is_line();

    if content.iter().any(|line| matches(&DIVIDER_LINE, line)) {
        return Some("Drop the section divider; split the file or let the code's structure show the grouping.");
    }

    if is_line && content.len() == 1 && matches(&TEST_STEP, content[0]) {
        return Some("Drop the test step label.");
    }

    let joined = text.join("\n");

    if matches(&TASK_MARKER, &joined) && !links_issue(&joined) {
        return Some("Link the issue that tracks this, or drop the marker.");
    }

    if typescript && content.iter().any(|line| matches(&JSDOC_TYPE, line)) {
        return Some("Drop the JSDoc type; TypeScript already has it.");
    }

    if content.iter().any(|line| matches(&EMPTY_JSDOC_TAG, line)) {
        return Some("Drop the JSDoc tag that has no description.");
    }

    if is_line && content.iter().all(|line| matches(&COMMENTED_CODE, line)) {
        return Some("Delete commented-out code; git keeps the history.");
    }

    if restates_next_name(source, lines, group, &content) {
        return Some("Drop the comment; it only repeats the name below it. Say why, or say nothing.");
    }

    None
}

/// Whether an own-line comment says nothing but the name declared on the next line.
fn restates_next_name(source: &str, lines: &Lines, group: &[&Comment], content: &[&str]) -> bool {
    let first = group[0].span.start;
    let own_line = source[lines.0[lines.line(first)]..first as usize].trim().is_empty();

    if !own_line || content.iter().any(|line| line.starts_with('@')) {
        return false;
    }

    let Some(next) = lines.text(source, lines.line(group[group.len() - 1].span.end) + 1) else {
        return false;
    };

    let Some(name) = DECLARATION.find(next).and_then(|found| found.group(1)).map(|range| &next[range]) else {
        return false;
    };

    if KEYWORDS.contains(&name) {
        return false;
    }

    let said = words(&content.join(" "));
    let named = words(&split_camel_case(name));

    (1..=6).contains(&said.len()) && said.iter().all(|word| named.contains(word))
}

/// The meaningful words of `text`, lowercased and with a plural `s` dropped.
fn words(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_ascii_lowercase() && !c.is_ascii_digit())
        .filter(|word| !word.is_empty() && !FILLER_WORDS.contains(word))
        .map(stem)
        .filter(|word| !FILLER_WORDS.contains(&word.as_str()))
        .collect()
}

fn stem(word: &str) -> String {
    match word.strip_suffix('s') {
        Some(rest) if word.len() > 3 && !rest.ends_with('s') => rest.to_owned(),
        _ => word.to_owned(),
    }
}

/// `parseHTMLDocument` as `parse HTML Document`.
fn split_camel_case(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    let mut out = String::with_capacity(name.len() + 4);

    for (at, &c) in chars.iter().enumerate() {
        if at > 0 && c.is_ascii_uppercase() {
            let previous = chars[at - 1];
            let next_is_lower = chars.get(at + 1).is_some_and(char::is_ascii_lowercase);

            if previous.is_ascii_lowercase() || previous.is_ascii_digit() || (previous.is_ascii_uppercase() && next_is_lower) {
                out.push(' ');
            }
        }

        out.push(c);
    }

    out
}
