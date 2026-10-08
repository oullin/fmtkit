//! Wrap unbraced statement bodies in a block.

use fmtkit_core::{Edit, EditSet};
use oxc_ast::AstKind;
use oxc_ast::ast::{Program, Statement};
use oxc_ast_visit::Visit;
use oxc_span::GetSpan;

use crate::syntax::{indent_unit, line_indent};

/// Wrap each unbraced loop, `with`, and `if` body; an `else if` link stays.
/// Nested candidates are dropped here and picked up on the next run.
pub(crate) fn edits<'a>(text: &'a str, program: &'a Program<'a>) -> EditSet {
    let mut collector = Bodies { text, unit: indent_unit(text), edits: EditSet::new() };

    collector.visit_program(program);

    let mut edits = collector.edits;

    edits.retain_non_overlapping();

    edits
}

struct Bodies<'t> {
    text: &'t str,
    unit: &'t str,
    edits: EditSet,
}

impl Bodies<'_> {
    fn wrap(&mut self, owner_start: u32, body: &Statement<'_>) {
        if matches!(body, Statement::BlockStatement(_)) {
            return;
        }

        let span = body.span();
        let indent = line_indent(self.text, owner_start);
        let source = &self.text[span.start as usize..span.end as usize];

        self.edits.push(Edit::new(span.start, span.end, format!("{{\n{indent}{}{source}\n{indent}}}", self.unit)));
    }
}

impl<'a> Visit<'a> for Bodies<'_> {
    fn enter_node(&mut self, kind: AstKind<'a>) {
        let start = kind.span().start;

        match kind {
            AstKind::DoWhileStatement(it) => self.wrap(start, &it.body),
            AstKind::ForInStatement(it) => self.wrap(start, &it.body),
            AstKind::ForOfStatement(it) => self.wrap(start, &it.body),
            AstKind::ForStatement(it) => self.wrap(start, &it.body),
            AstKind::WhileStatement(it) => self.wrap(start, &it.body),
            AstKind::WithStatement(it) => self.wrap(start, &it.body),
            AstKind::IfStatement(it) => {
                self.wrap(start, &it.consequent);

                if let Some(alternate) = &it.alternate
                    && !matches!(alternate, Statement::IfStatement(_))
                {
                    self.wrap(start, alternate);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{compute, wrap_fully};
    use crate::passes::Pass;

    #[test]
    fn wraps_an_unbraced_if_body_in_a_block() {
        assert_eq!(wrap_fully("if (ready) run();\n"), "if (ready) {\n\trun();\n}\n");
    }

    #[test]
    fn wraps_nested_unbraced_bodies_across_iterations() {
        let result = wrap_fully("for (const item of items) if (item) use(item);\n");

        assert!(result.starts_with("for (const item of items) {\n\tif (item) {\n"), "{result}");
        assert!(result.contains("use(item);"));
    }

    #[test]
    fn leaves_else_if_chains_unwrapped_at_the_chain_link() {
        let source = "if (a) {\n\tone();\n} else if (b) {\n\ttwo();\n}\n";

        assert_eq!(wrap_fully(source), source);
    }

    #[test]
    fn wraps_the_loop_body_while_keeping_its_comment() {
        let result = wrap_fully("while (busy) tick(); // spin\n");

        assert!(result.starts_with("while (busy) {\n\ttick();\n}"), "{result}");
        assert!(result.contains("// spin"));
    }

    #[test]
    fn wraps_with_four_spaces_when_the_source_is_space_indented() {
        let result = wrap_fully("function run() {\n    if (ready) go();\n}\n");

        assert_eq!(result, "function run() {\n    if (ready) {\n        go();\n    }\n}\n");
        assert!(!result.contains('\t'), "space-indented body wrap must not introduce tabs");
    }

    #[test]
    fn leaves_already_braced_bodies_alone() {
        assert!(compute(Pass::BodyWrap, "sample.ts", "if (ready) {\n\trun();\n}\n").is_empty());
    }

    #[test]
    fn returns_no_edits_for_source_with_syntax_errors() {
        assert!(compute(Pass::BodyWrap, "sample.ts", "if (broken run();\n").is_empty());
    }
}
