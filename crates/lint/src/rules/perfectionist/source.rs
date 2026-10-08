//! The parts of ESLint's `SourceCode` perfectionist leans on: lines, comments,
//! the tokens around a node, and `eslint-disable` directives.

use oxc_ast::CommentKind;
use oxc_semantic::Semantic;
use oxc_span::Span;
use rustc_hash::{FxHashMap, FxHashSet};

/// One comment: its full span (delimiters included) and its value's span.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Comment {
    pub span: Span,
    pub value: Span,
    pub block: bool,
}

/// Per-file line and comment index, shared by every perfectionist rule.
#[derive(Debug, Default)]
pub struct Index {
    /// Start offset of each line; ESLint's line terminators.
    starts: Vec<u32>,
    /// End offset of each line, before its terminator.
    ends: Vec<u32>,
    comments: Vec<Comment>,
    by_start: FxHashMap<u32, usize>,
    by_end: FxHashMap<u32, usize>,
}

impl Index {
    pub fn new(semantic: &Semantic<'_>) -> Self {
        let text = semantic.source_text();
        let mut starts = vec![0];
        let mut ends = Vec::new();
        let mut chars = text.char_indices().peekable();

        while let Some((at, c)) = chars.next() {
            let at = offset(at);

            match c {
                '\r' if chars.peek().is_some_and(|&(_, next)| next == '\n') => {
                    chars.next();
                    ends.push(at);
                    starts.push(at + 2);
                }
                '\r' | '\n' | '\u{2028}' | '\u{2029}' => {
                    ends.push(at);
                    starts.push(at + offset(c.len_utf8()));
                }
                _ => {}
            }
        }

        ends.push(offset(text.len()));

        let comments: Vec<Comment> = semantic
            .comments()
            .iter()
            .map(|comment| {
                let span = comment.span;
                let (block, open, close) = match comment.kind {
                    CommentKind::SingleLineBlock | CommentKind::MultiLineBlock => (true, 2, 2),
                    CommentKind::Line => (false, 2, 0),
                    CommentKind::HtmlOpen => (false, 4, 0),
                    CommentKind::HtmlClose => (false, 3, 0),
                };
                let value = Span::new((span.start + open).min(span.end), span.end.saturating_sub(close).max(span.start));

                Comment { span, value, block }
            })
            .collect();
        let by_start = comments.iter().enumerate().map(|(i, c)| (c.span.start, i)).collect();
        let by_end = comments.iter().enumerate().map(|(i, c)| (c.span.end, i)).collect();

        Self { starts, ends, comments, by_start, by_end }
    }

    /// The 1-based line holding `offset`.
    pub fn line(&self, offset: u32) -> u32 {
        let index = self.starts.partition_point(|&start| start <= offset);

        u32::try_from(index).unwrap_or(u32::MAX)
    }

    pub fn comments(&self) -> &[Comment] {
        &self.comments
    }
}

/// A file's text plus its [`Index`].
#[derive(Clone, Copy)]
pub struct Src<'s> {
    pub text: &'s str,
    pub index: &'s Index,
}

impl<'s> Src<'s> {
    pub fn slice(&self, start: u32, end: u32) -> &'s str {
        self.text.get(start as usize..end as usize).unwrap_or_default()
    }

    pub fn text_of(&self, span: Span) -> &'s str {
        self.slice(span.start, span.end)
    }

    pub fn line(&self, offset: u32) -> u32 {
        self.index.line(offset)
    }

    pub fn comment_value(&self, comment: &Comment) -> &'s str {
        self.text_of(comment.value)
    }

    /// ESLint's `getLinesBetween`: blank lines strictly between the line `left`
    /// ends on and the line `right` starts on.
    pub fn blank_lines_between(&self, left_end: u32, right_start: u32) -> u32 {
        let from = self.line(left_end) + 1;
        let to = self.line(right_start);
        let mut count = 0;

        for line in from..to {
            let index = (line - 1) as usize;
            let (Some(&start), Some(&end)) = (self.index.starts.get(index), self.index.ends.get(index)) else {
                continue;
            };

            if self.slice(start, end).chars().all(is_js_space) {
                count += 1;
            }
        }

        count
    }

    /// Where the token before `pos` ends, skipping whitespace and comments.
    pub fn token_before(&self, pos: u32) -> Option<u32> {
        let mut pos = pos;

        loop {
            pos = self.skip_space_back(pos);

            match self.index.by_end.get(&pos) {
                Some(&i) => pos = self.index.comments[i].span.start,
                None => return (pos > 0).then_some(pos),
            }
        }
    }

    /// Where the token after `pos` starts, skipping whitespace and comments.
    pub fn token_after(&self, pos: u32) -> Option<u32> {
        let mut pos = pos;

        loop {
            pos = self.skip_space(pos);

            match self.index.by_start.get(&pos) {
                Some(&i) => pos = self.index.comments[i].span.end,
                None => return ((pos as usize) < self.text.len()).then_some(pos),
            }
        }
    }

    /// The comment starting at `pos`, if any.
    pub fn comment_at(&self, pos: u32) -> Option<&'s Comment> {
        self.index.by_start.get(&pos).map(|&i| &self.index.comments[i])
    }

    pub fn char_at(&self, pos: u32) -> Option<char> {
        self.text.get(pos as usize..).and_then(|rest| rest.chars().next())
    }

    /// ESLint's `getCommentsBefore`: the comments directly before `pos`.
    pub fn comments_before(&self, pos: u32) -> Vec<&'s Comment> {
        let mut found = Vec::new();
        let mut pos = pos;

        loop {
            pos = self.skip_space_back(pos);

            let Some(&i) = self.index.by_end.get(&pos) else { break };
            let comment = &self.index.comments[i];

            found.push(comment);
            pos = comment.span.start;
        }

        found.reverse();
        found
    }

    /// Perfectionist's `getCommentsBefore`: [`Self::comments_before`] minus
    /// comments trailing the previous token's line.
    pub fn relevant_comments_before(&self, pos: u32) -> Vec<&'s Comment> {
        self.comments_before(pos)
            .into_iter()
            .filter(|comment| self.token_before(comment.span.start).is_none_or(|end| self.line(end) != self.line(comment.span.start)))
            .collect()
    }

    fn skip_space_back(&self, pos: u32) -> u32 {
        let head = self.text.get(..pos as usize).unwrap_or_default();
        let kept = head.trim_end_matches(is_js_space);

        offset(kept.len())
    }

    fn skip_space(&self, pos: u32) -> u32 {
        let tail = self.text.get(pos as usize..).unwrap_or_default();
        let kept = tail.trim_start_matches(is_js_space);

        pos + offset(tail.len() - kept.len())
    }
}

/// JavaScript's `WhiteSpace` and `LineTerminator` (`\s`, `String#trim`).
pub fn is_js_space(c: char) -> bool {
    c == '\u{FEFF}' || (c.is_whitespace() && c != '\u{85}')
}

pub fn js_trim(text: &str) -> &str {
    text.trim_matches(is_js_space)
}

pub fn offset(len: usize) -> u32 {
    u32::try_from(len).unwrap_or(u32::MAX)
}

/// An `eslint-disable*` / `eslint-enable` directive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Directive {
    Disable,
    Enable,
    DisableLine,
    DisableNextLine,
}

/// Perfectionist's `getEslintDisabledRules`: the directive in a comment's
/// value and the rules it names (`None` meaning all of them).
pub fn directive(value: &str) -> Option<(Directive, Option<Vec<&str>>)> {
    const DIRECTIVES: [(&str, Directive); 4] = [
        ("eslint-disable", Directive::Disable),
        ("eslint-enable", Directive::Enable),
        ("eslint-disable-line", Directive::DisableLine),
        ("eslint-disable-next-line", Directive::DisableNextLine),
    ];

    let part = js_trim(&value[..justification(value).unwrap_or(value.len())]);

    for (name, directive) in DIRECTIVES {
        if part == name {
            return Some((directive, None));
        }

        let Some(rest) = part.strip_prefix(name).and_then(|rest| rest.strip_prefix(' ')) else { continue };

        if rest.is_empty() {
            continue;
        }

        return Some((directive, Some(rest.split(',').map(js_trim).filter(|rule| !rule.is_empty()).collect())));
    }

    None
}

/// Where `/\s-{2,}\s/u` first matches.
fn justification(value: &str) -> Option<usize> {
    let chars: Vec<(usize, char)> = value.char_indices().collect();

    for (i, &(at, c)) in chars.iter().enumerate() {
        if !is_js_space(c) {
            continue;
        }

        let dashes = chars[i + 1..].iter().take_while(|&&(_, c)| c == '-').count();

        if dashes >= 2 && chars.get(i + 1 + dashes).is_some_and(|&(_, c)| is_js_space(c)) {
            return Some(at);
        }
    }

    None
}

/// Perfectionist's `getEslintDisabledLines` for `rule`.
pub fn disabled_lines(src: Src<'_>, rule: &str) -> FxHashSet<u32> {
    let mut lines = FxHashSet::default();
    let mut disabled_since: Option<u32> = None;

    for comment in src.index.comments() {
        let Some((kind, rules)) = directive(src.comment_value(comment)) else { continue };

        if rules.is_some_and(|rules| !rules.contains(&rule)) {
            continue;
        }

        match kind {
            Directive::DisableNextLine => {
                lines.insert(src.line(comment.span.end) + 1);
            }
            Directive::DisableLine => {
                lines.insert(src.line(comment.span.start));
            }
            Directive::Disable => {
                disabled_since.get_or_insert(src.line(comment.span.start));
            }
            Directive::Enable => {
                if let Some(since) = disabled_since.take().filter(|&since| since != 0) {
                    lines.extend(since + 1..=src.line(comment.span.start));
                }
            }
        }
    }

    lines
}
