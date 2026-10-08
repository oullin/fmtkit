//! Order class members as properties, constructors, then methods.

use fmtkit_core::{Edit, EditSet};
use oxc_ast::AstKind;
use oxc_ast::ast::{ClassBody, Program};
use oxc_ast_visit::Visit;
use oxc_span::GetSpan;

use super::spacing::classify;
use crate::syntax::contains_comment_opener;

/// One whole-body rewrite per out-of-order class. A class nested inside a
/// rewritten one is left for the next run, so the set never overlaps.
pub(crate) fn edits<'a>(text: &'a str, program: &'a Program<'a>) -> EditSet {
    let mut collector = Bodies { text, edits: EditSet::new() };

    collector.visit_program(program);

    let mut edits = collector.edits;

    edits.retain_non_overlapping();

    edits
}

struct Bodies<'t> {
    text: &'t str,
    edits: EditSet,
}

impl<'a> Visit<'a> for Bodies<'_> {
    fn enter_node(&mut self, kind: AstKind<'a>) {
        if let AstKind::ClassBody(body) = kind
            && let Some(edit) = reorder(self.text, body)
        {
            self.edits.push(edit);
        }
    }
}

fn reorder(text: &str, body: &ClassBody<'_>) -> Option<Edit> {
    let members = &body.body;

    if members.len() < 2 {
        return None;
    }

    let mut desired: Vec<usize> = (0..members.len()).collect();

    desired.sort_by_key(|index| classify(&members[*index]));

    if desired.iter().enumerate().all(|(position, index)| position == *index) {
        return None;
    }

    let slice = |start: u32, end: u32| &text[start as usize..end as usize];
    let inner_start = body.span.start + 1;
    let inner_end = body.span.end - 1;
    let first_start = members[0].span().start;
    let last_end = members[members.len() - 1].span().end;

    let gaps_hold_comment = contains_comment_opener(slice(inner_start, first_start))
        || members.windows(2).any(|pair| contains_comment_opener(slice(pair[0].span().end, pair[1].span().start)))
        || contains_comment_opener(slice(last_end, inner_end));

    if gaps_hold_comment {
        return None;
    }

    let prefix = slice(inner_start, first_start);
    let newline = prefix.rfind('\n')?;
    let indent = &prefix[newline + 1..];

    if !indent.bytes().all(|b| b == b' ' || b == b'\t') {
        return None;
    }

    let slices: Vec<&str> = desired.iter().map(|index| slice(members[*index].span().start, members[*index].span().end)).collect();

    if slices.windows(2).any(|pair| continues_onto(pair[0], pair[1])) {
        return None;
    }

    let separator = format!("\n{indent}");
    let mut replacement = String::with_capacity((inner_end - inner_start) as usize + indent.len() * slices.len());

    replacement.push_str(&separator);
    replacement.push_str(&slices.join(&separator));
    replacement.push_str(slice(last_end, inner_end));

    Some(Edit::new(inner_start, inner_end, replacement))
}

/// Whether a member without a terminating `;` or `}` would run into the next
/// one: `x = a` followed by `[key]() {}` reads as `a[key]()`.
fn continues_onto(previous: &str, next: &str) -> bool {
    let terminated = previous.ends_with(';') || previous.ends_with('}');
    let hazard = next.bytes().next().is_some_and(|b| matches!(b, b'[' | b'(' | b'*' | b'<' | b'`' | b'+' | b'-' | b'/'));

    !terminated && hazard
}

#[cfg(test)]
mod tests {
    use super::super::tests::{apply_once, compute, until_stable};
    use crate::passes::Pass;

    #[test]
    fn class_members_are_reordered_as_properties_constructors_then_methods() {
        let input = "class Example {\n\trun() {}\n\tvalue = 1;\n\tconstructor() {}\n}\n";

        assert_eq!(compute(Pass::ClassReorder, "fixture.ts", input).len(), 1);
        assert_eq!(apply_once(Pass::ClassReorder, "fixture.ts", input), "class Example {\n\tvalue = 1;\n\tconstructor() {}\n\trun() {}\n}\n");
    }

    #[test]
    fn class_reorder_skips_members_with_comments_between_them() {
        let input = "class Example {\n\trun() {}\n\t// Preserve this member grouping.\n\tvalue = 1;\n}\n";

        assert!(compute(Pass::ClassReorder, "fixture.ts", input).is_empty());
    }

    #[test]
    fn class_reorder_skips_already_ordered_and_single_member_classes() {
        assert!(compute(Pass::ClassReorder, "ordered.ts", "class Ordered {\n\tvalue = 1;\n\tconstructor() {}\n\trun() {}\n}\n").is_empty());
        assert!(compute(Pass::ClassReorder, "single.ts", "class Single {\n\trun() {}\n}\n").is_empty());
    }

    /// v1 applied the outer and inner rewrites against stale offsets and
    /// corrupted the class; the outer one now lands first, the inner one next.
    #[test]
    fn nested_classes_reorder_without_overlapping_edits() {
        let input = "class Outer {\n\trun() {\n\t\treturn class Inner {\n\t\t\tgo() {}\n\t\t\tb = 2;\n\t\t};\n\t}\n\ta = 1;\n}\n";
        let expected = "class Outer {\n\ta = 1;\n\trun() {\n\t\treturn class Inner {\n\t\t\tb = 2;\n\t\t\tgo() {}\n\t\t};\n\t}\n}\n";

        assert_eq!(compute(Pass::ClassReorder, "fixture.ts", input).len(), 1);
        assert_eq!(until_stable(Pass::ClassReorder, "fixture.ts", input), expected);
    }

    #[test]
    fn declines_a_reorder_that_would_join_an_unterminated_property_to_the_next_member() {
        let input = "class Example {\n\t[key]() {}\n\tvalue = 1\n}\n";

        assert!(compute(Pass::ClassReorder, "fixture.ts", input).is_empty());
    }
}
