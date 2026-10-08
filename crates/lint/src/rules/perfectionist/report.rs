//! `reportAllErrors` and the fixes it attaches: moving nodes with their
//! comments, carrying trailing comments along, and fixing blank lines.

use std::sync::LazyLock;

use oxc_span::Span;
use regress::Regex;
use rustc_hash::{FxHashMap, FxHashSet};

use fmtkit_core::Edit;

use super::options::{Newlines, Options, PartitionComment};
use super::sort::{Item, circular, depends};
use super::source::{Directive, Src, directive};

/// A rule's message ids; perfectionist words every rule's messages the same.
#[derive(Debug, Clone, Copy)]
pub struct Messages {
    pub order: &'static str,
    pub group_order: &'static str,
    pub extra_spacing: &'static str,
    pub missed_spacing: &'static str,
    pub dependency_order: Option<&'static str>,
}

/// One report: where, the message id and its interpolated text, and the fix.
#[derive(Debug, Clone)]
pub struct Problem {
    pub span: Span,
    pub message: String,
    pub fix: Vec<Edit>,
}

pub struct Report<'r> {
    pub src: Src<'r>,
    pub options: &'r Options,
    pub items: &'r [Item],
    pub messages: Messages,
    /// The `partitionByComment` the fixes see.
    pub partition_comment: &'r PartitionComment,
}

fn single_line(text: &str) -> String {
    static SPACES: LazyLock<Regex> = LazyLock::new(|| Regex::with_flags(r"\s{2,}", "u").expect("a valid pattern"));

    super::source::js_trim(&SPACES.replace_all(text, " ")).to_owned()
}

impl Report<'_> {
    /// `reportAllErrors` over `nodes`, given perfectionist's two sorts.
    pub fn run(&self, nodes: &[usize], sorted: &[usize], sorted_excluding_disabled: &[usize]) -> Vec<Problem> {
        let items = self.items;
        let index: FxHashMap<usize, usize> = sorted.iter().enumerate().map(|(i, &node)| (node, i)).collect();
        let index_excluding: FxHashMap<usize, usize> = sorted_excluding_disabled.iter().enumerate().map(|(i, &node)| (node, i)).collect();
        let cycles = if self.messages.dependency_order.is_some() { circular(items, nodes) } else { FxHashSet::default() };
        let mut fix: Option<Vec<Edit>> = None;
        let mut problems = Vec::new();

        for (position, &right) in nodes.iter().enumerate() {
            let left = position.checked_sub(1).map(|i| nodes[i]);
            let right_group = self.options.group_index(&items[right].group);
            let right_index = index.get(&right).copied().unwrap_or(0);
            let unordered = self
                .messages
                .dependency_order
                .and_then(|_| nodes[..position].iter().copied().find(|&other| !cycles.contains(&other) && depends(items, right, other)));
            let mut ids = Vec::new();

            if let Some(left) = left {
                let left_index = index.get(&left).copied().unwrap_or(0);
                let left_group = self.options.group_index(&items[left].group);
                let right_excluding = index_excluding.get(&right).map_or(-1, |&i| i64::try_from(i).unwrap_or(i64::MAX));
                let left_disabled_out_of_place = items[left].disabled && i64::try_from(left_index).unwrap_or(i64::MAX) >= right_excluding;

                if unordered.is_some() || left_index > right_index || left_disabled_out_of_place {
                    ids.push(match (unordered, self.messages.dependency_order) {
                        (Some(_), Some(id)) => id,
                        _ if left_group == right_group => self.messages.order,
                        _ => self.messages.group_order,
                    });
                }

                ids.extend(self.newline_error(left, right, left_group, right_group));
            }

            for id in ids {
                let fix = fix.get_or_insert_with(|| self.fix(nodes, sorted_excluding_disabled)).clone();
                let message = self.message(id, left, right, unordered);

                problems.push(Problem { span: items[right].span, message, fix });
            }
        }

        problems
    }

    fn message(&self, id: &'static str, left: Option<usize>, right: usize, unordered: Option<usize>) -> String {
        let items = self.items;
        let left_name = single_line(left.map_or("", |left| items[left].name.as_str()));
        let right_name = single_line(&items[right].name);

        if Some(id) == self.messages.dependency_order {
            let dependent = unordered.map_or("", |node| items[node].name.as_str());

            format!("Expected dependency \"{right_name}\" to come before \"{dependent}\".")
        } else if id == self.messages.group_order {
            let left_group = left.map_or("undefined", |left| items[left].group.as_str());

            format!("Expected \"{right_name}\" ({}) to come before \"{left_name}\" ({left_group}).", items[right].group)
        } else if id == self.messages.extra_spacing {
            format!("Extra spacing between \"{left_name}\" and \"{right_name}\".")
        } else if id == self.messages.missed_spacing {
            format!("Missed spacing between \"{left_name}\" and \"{right_name}\".")
        } else {
            format!("Expected \"{right_name}\" to come before \"{left_name}\".")
        }
    }

    /// `getNewlinesBetweenErrors`.
    fn newline_error(&self, left: usize, right: usize, left_group: usize, right_group: usize) -> Option<&'static str> {
        let items = self.items;

        if left_group > right_group || items[left].partition != items[right].partition {
            return None;
        }

        let Newlines::Count(wanted) = self.options.newlines(left_group, right_group) else { return None };
        let found = self.src.blank_lines_between(items[left].span.end, items[right].span.start);

        match found.cmp(&wanted) {
            std::cmp::Ordering::Less => Some(self.messages.missed_spacing),
            std::cmp::Ordering::Greater => Some(self.messages.extra_spacing),
            std::cmp::Ordering::Equal => None,
        }
    }

    /// `createFixProvider`: every report of one sort shares one merged fix.
    fn fix(&self, nodes: &[usize], sorted: &[usize]) -> Vec<Edit> {
        let order = self.order_fixes(nodes, sorted);
        let comments = self.comment_after_fixes(nodes, sorted);
        let fixes = if comments.is_empty() {
            let newlines = self.newline_fixes(nodes, sorted);

            if newlines.is_empty() { order } else { [order, newlines].concat() }
        } else {
            [order, comments].concat()
        };

        self.merge(fixes)
    }

    /// `mergeFixes`: one edit spanning them all, or none when they overlap
    /// (ESLint rejects overlapping fixes in one report).
    fn merge(&self, mut fixes: Vec<Edit>) -> Vec<Edit> {
        if fixes.len() < 2 {
            return fixes;
        }

        fixes.sort_by_key(|fix| (fix.start, fix.end));

        let start = fixes[0].start;
        let end = fixes[fixes.len() - 1].end;
        let mut last = start;
        let mut text = String::new();

        for fix in &fixes {
            if fix.start < last {
                return Vec::new();
            }

            text.push_str(self.src.slice(last, fix.start));
            text.push_str(&fix.text);
            last = fix.end;
        }

        vec![Edit::new(start, end, text)]
    }

    /// `getNodeRange`: the node plus the comments directly above it.
    pub fn node_range(&self, span: Span, partition_comment: &PartitionComment) -> (u32, u32) {
        let src = self.src;
        let comments = src.relevant_comments_before(span.start);
        let mut start = span.start;

        for i in (0..comments.len()).rev() {
            let comment = comments[i];
            let value = src.comment_value(comment);
            let kind = directive(value).map(|(kind, _)| kind);

            if partition_comment.matches(comment, value) || matches!(kind, Some(Directive::Disable | Directive::Enable)) {
                break;
            }

            let anchor = if i + 1 == comments.len() { src.line(span.start) } else { src.line(comments[i + 1].span.start) };

            if src.line(comment.span.end) + 1 != anchor {
                break;
            }

            start = start.min(comment.span.start);
        }

        (start, span.end)
    }

    /// `makeOrderFixes`.
    fn order_fixes(&self, nodes: &[usize], sorted: &[usize]) -> Vec<Edit> {
        let src = self.src;
        let mut fixes = Vec::new();

        for (&node, &sorted_node) in nodes.iter().zip(sorted) {
            if node == sorted_node {
                continue;
            }

            let (node, sorted_node) = (&self.items[node], &self.items[sorted_node]);
            let (from, to) = self.node_range(sorted_node.span, self.partition_comment);
            let mut code = src.slice(from, to).to_owned();
            let sorted_text = src.text_of(sorted_node.span);
            let next = src.token_after(node.span.end);
            let safe_end = sorted_text.ends_with(';') || sorted_text.ends_with(',');
            let next_on_same_line = next.is_some_and(|next| src.line(next) == src.line(node.span.end));
            let next_safe = next.and_then(|next| src.char_at(next)).is_some_and(|c| c == ';' || c == ',');

            if sorted_node.semicolon && !safe_end && !next_safe && next_on_same_line {
                code.push(';');
            }

            let (start, end) = self.node_range(node.span, self.partition_comment);

            fixes.push(Edit::new(start, end, code));
        }

        fixes
    }

    /// `makeCommentAfterFixes`.
    fn comment_after_fixes(&self, nodes: &[usize], sorted: &[usize]) -> Vec<Edit> {
        let src = self.src;
        let mut fixes = Vec::new();

        for (&node, &sorted_node) in nodes.iter().zip(sorted) {
            if node == sorted_node {
                continue;
            }

            let (node, sorted_node) = (self.items[node].span, self.items[sorted_node].span);
            let Some(comment) = self.comment_after(sorted_node) else { continue };

            if src.line(node.start) == src.line(sorted_node.end) {
                continue;
            }

            let Some(removal_start) = src.token_before(comment.start) else { continue };
            let removed = src.slice(removal_start, comment.end);

            fixes.push(Edit::new(removal_start, comment.end, ""));

            let insert_after = self.insertion_point(node);
            let mut text = removed.to_owned();
            let is_line = src.comment_at(comment.start).is_some_and(|comment| !comment.block);

            if is_line && src.token_after(insert_after).is_some_and(|next| src.line(next) == src.line(insert_after)) {
                text.push('\n');
            }

            fixes.push(Edit::insert(insert_after, text));
        }

        fixes
    }

    /// The comment right after `span`'s `,`/`;`/`:`, on the line it ends on.
    fn comment_after(&self, span: Span) -> Option<Span> {
        let src = self.src;
        let mut pos = span.end;

        loop {
            let rest = src.text.get(pos as usize..)?;
            let skipped = rest.len() - rest.trim_start_matches(super::source::is_js_space).len();

            pos += super::source::offset(skipped);

            if let Some(comment) = src.comment_at(pos) {
                return (src.line(comment.span.end) == src.line(span.end)).then_some(comment.span);
            }

            match src.char_at(pos) {
                Some(',' | ';' | ':') => pos += 1,
                _ => return None,
            }
        }
    }

    /// `computeNodeToInsertAfter`: after a trailing `,`/`:` on the node's line.
    fn insertion_point(&self, span: Span) -> u32 {
        let src = self.src;

        match src.token_after(span.end) {
            Some(next) if matches!(src.char_at(next), Some(',' | ':')) && src.line(next + 1) == src.line(span.end) => next + 1,
            _ => span.end,
        }
    }

    /// `makeNewlinesBetweenFixes`.
    fn newline_fixes(&self, nodes: &[usize], sorted: &[usize]) -> Vec<Edit> {
        let src = self.src;
        let items = self.items;
        let mut fixes = Vec::new();

        for i in 0..sorted.len().saturating_sub(1) {
            let (current, next) = (&items[sorted[i]], &items[sorted[i + 1]]);

            if current.partition != next.partition {
                continue;
            }

            let (left_group, right_group) = (self.options.group_index(&current.group), self.options.group_index(&next.group));

            if left_group > right_group {
                continue;
            }

            let Newlines::Count(wanted) = self.options.newlines(left_group, right_group) else { continue };
            let (node, next_node) = (items[nodes[i]].span, items[nodes[i + 1]].span);
            let (_, from) = self.node_range(node, &PartitionComment::Off);
            let (to, _) = self.node_range(next_node, &PartitionComment::Off);

            if src.blank_lines_between(node.end, next_node.start) == wanted {
                continue;
            }

            let same_line = src.line(node.end) == src.line(next_node.start);
            let replacement = newline_replacement(src.slice(from, to.max(from)), wanted, same_line);

            fixes.push(Edit::new(from, to.max(from), replacement));
        }

        fixes
    }
}

/// `computeRangeReplacement`.
fn newline_replacement(between: &str, wanted: u32, same_line: bool) -> String {
    static BLANK: LazyLock<Regex> = LazyLock::new(|| Regex::with_flags(r"\n\s*\n", "u").expect("a valid pattern"));
    static RUNS: LazyLock<Regex> = LazyLock::new(|| Regex::with_flags(r"\n+", "u").expect("a valid pattern"));

    let mut text = RUNS.replace_all(&BLANK.replace_all(between, "\n"), "\n");

    if wanted == 0 {
        return text;
    }

    let add = |text: &str| match text.find('\n') {
        Some(at) => format!("{}\n{}", &text[..at], &text[at..]),
        None => format!("{text}\n"),
    };

    for _ in 0..wanted {
        text = add(&text);
    }

    if same_line { add(&text) } else { text }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_newlines_like_perfectionist() {
        assert_eq!(newline_replacement(",\n\n\n  ", 0, false), ",\n  ");
        assert_eq!(newline_replacement(",\n  ", 1, false), ",\n\n  ");
        assert_eq!(newline_replacement(", ", 1, true), ", \n\n");
        assert_eq!(single_line("a  b\n   c "), "a b c");
    }
}
