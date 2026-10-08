//! Insert the blank lines the statement-spacing policy requires.

use fmtkit_core::{Edit, EditSet};
use oxc_ast::ast::Program;
use rustc_hash::FxHashSet;

use super::lists::for_each_list;
use super::spacing::needs_blank_line;
use crate::syntax::to_u32;

/// One zero-width `\n` insert at the start of each following sibling's line.
/// Positions are deduplicated, so siblings sharing a line add one newline.
pub(crate) fn edits<'a>(text: &'a str, program: &'a Program<'a>) -> EditSet {
    let mut seen = FxHashSet::default();
    let mut edits = EditSet::new();

    for_each_list(program, |items| {
        for pair in items.windows(2) {
            let (previous, next) = (pair[0], pair[1]);

            if !needs_blank_line(previous, next) {
                continue;
            }

            let previous_end = previous.span().end as usize;
            let next_start = next.span().start as usize;

            if next_start <= previous_end || text.as_bytes()[previous_end..next_start].iter().filter(|b| **b == b'\n').count() >= 2 {
                continue;
            }

            if let Some(newline) = text[..next_start].rfind('\n') {
                let position = to_u32(newline + 1);

                if seen.insert(position) {
                    edits.push(Edit::insert(position, "\n"));
                }
            }
        }
    });

    edits
}

#[cfg(test)]
mod tests {
    use fmtkit_core::{Edit, EditSet};

    use super::super::tests::{apply_once, compute};
    use crate::passes::Pass;

    /// The v1 reference insert: dedupe positions, insert one newline at each.
    fn reference_insert(content: &str, positions: &[u32]) -> String {
        let mut sorted: Vec<u32> = positions.to_vec();

        sorted.sort_unstable();
        sorted.dedup();

        let mut out = content.to_owned();

        for position in sorted.into_iter().rev() {
            out.insert(position as usize, '\n');
        }

        out
    }

    fn zero_width_inserts(positions: &[u32]) -> EditSet {
        let mut edits: EditSet = positions.iter().map(|position| Edit::insert(*position, "\n")).collect();

        edits.normalize("");

        edits
    }

    #[test]
    fn edit_set_matches_the_reference_insert_for_distinct_positions() {
        let source = "alpha\nbeta\ngamma\n";
        let positions = [0, 6, 11];

        assert_eq!(zero_width_inserts(&positions).apply(source).unwrap(), reference_insert(source, &positions));
    }

    #[test]
    fn edit_set_matches_the_reference_insert_for_adjacent_positions() {
        let positions = [3, 4];

        assert_eq!(zero_width_inserts(&positions).apply("abcdef").unwrap(), reference_insert("abcdef", &positions));
    }

    #[test]
    fn edit_set_matches_the_reference_insert_at_offset_zero_and_end_of_file() {
        let positions = [0, 3];

        assert_eq!(zero_width_inserts(&positions).apply("abc").unwrap(), reference_insert("abc", &positions));
    }

    #[test]
    fn deduplicated_inserts_match_the_reference_insert_for_repeated_positions() {
        let source = "let value = 1; doWork(); let next = 2;\n";
        let positions = [15, 15, 24];

        assert_eq!(zero_width_inserts(&positions).apply(source).unwrap(), reference_insert(source, &positions));
    }

    const IMPORT_THEN_FUNCTION: &str = "import { foo } from \"node:foo\";\nexport function bar() {\n\treturn foo();\n}\n";

    #[test]
    fn inserts_a_blank_line_between_an_import_and_a_following_function() {
        assert_eq!(
            apply_once(Pass::BlankLine, "fixture.ts", IMPORT_THEN_FUNCTION),
            "import { foo } from \"node:foo\";\n\nexport function bar() {\n\treturn foo();\n}\n"
        );
    }

    #[test]
    fn proposes_only_zero_width_newline_inserts() {
        let edits = compute(Pass::BlankLine, "fixture.ts", IMPORT_THEN_FUNCTION);

        assert!(!edits.is_empty());

        for edit in edits.iter() {
            assert_eq!(edit.start, edit.end);
            assert_eq!(edit.text, "\n");
        }
    }

    #[test]
    fn leaves_an_already_spaced_document_unchanged() {
        let source = "import { foo } from \"node:foo\";\n\nexport function bar() {\n\treturn foo();\n}\n";

        assert!(compute(Pass::BlankLine, "fixture.ts", source).is_empty());
    }

    #[test]
    fn separates_the_first_binding_call_from_preceding_code_and_keeps_later_bindings_together() {
        let source = [
            "function register(application: Application) {",
            "\tthis.application = application;",
            "\tapplication.bindings.singleton(Service, () => new Service());",
            "\tapplication.bindings.instance(Config, config);",
            "\tapplication.bindings.singletonIf(Cache, () => new Cache());",
            "}",
            "",
        ]
        .join("\n");

        let expected = source.replace("\tthis.application = application;\n", "\tthis.application = application;\n\n");
        let output = apply_once(Pass::BlankLine, "fixture.ts", &source);

        assert_eq!(output, expected);
        assert!(compute(Pass::BlankLine, "fixture.ts", &output).is_empty());
    }

    #[test]
    fn recognises_direct_bindings_calls_without_separating_a_binding_block() {
        let source = "function register() {\n\tprepare();\n\tbindings.singleton(Service, makeService);\n\tbindings.instance(Config, config);\n}";

        assert_eq!(apply_once(Pass::BlankLine, "fixture.js", source), source.replace("\tprepare();\n", "\tprepare();\n\n"));
    }

    #[test]
    fn does_not_separate_calls_through_an_unrelated_member_named_singleton() {
        let source = "function register() {\n\tprepare();\n\tservices.singleton(Service, makeService);\n}";

        assert!(compute(Pass::BlankLine, "fixture.ts", source).is_empty());
    }

    #[test]
    fn returns_no_edits_for_source_with_syntax_errors() {
        assert!(compute(Pass::BlankLine, "fixture.ts", "function broken( {\n").is_empty());
    }

    #[test]
    fn keeps_consecutive_empty_case_labels_tight_and_still_spaces_the_ones_with_bodies() {
        let source = [
            "export function classify(value: string): number {",
            "\tswitch (value) {",
            "\t\tcase 'a':",
            "\t\tcase 'b':",
            "\t\t\treturn 1;",
            "\t\tdefault:",
            "\t\tcase 'c':",
            "\t\t\treturn 2;",
            "\t}",
            "}",
            "",
        ]
        .join("\n");

        let expected = [
            "export function classify(value: string): number {",
            "\tswitch (value) {",
            "\t\tcase 'a':",
            "\t\tcase 'b':",
            "\t\t\treturn 1;",
            "",
            "\t\tdefault:",
            "\t\tcase 'c':",
            "\t\t\treturn 2;",
            "\t}",
            "}",
            "",
        ]
        .join("\n");

        assert_eq!(apply_once(Pass::BlankLine, "fixture.ts", &source), expected);
    }
}
