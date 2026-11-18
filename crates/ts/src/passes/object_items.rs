//! Put each property of an object literal with two or more properties on its
//! own line.
//!
//! The pass only breaks the line after the object's `{`. oxc_formatter, like
//! Prettier with `objectWrap: "preserve"`, keeps an object expanded when a
//! newline follows its `{`, so the formatter step that follows lays out one
//! property per line and the closing brace on its own line, and keeps that
//! layout on every later run.
//!
//! Objects inside a template literal's `${}` are left alone: splitting them
//! would break the template's text across lines.

use fmtkit_core::{Edit, EditSet};
use oxc_ast::AstKind;
use oxc_ast::ast::{ObjectExpression, Program};
use oxc_ast_visit::Visit;
use oxc_span::GetSpan;

/// One newline after the `{` of every object literal with two or more
/// properties whose first property shares a line with that brace, outside
/// template literals.
pub(crate) fn edits<'a>(text: &'a str, program: &'a Program<'a>) -> EditSet {
    let mut objects = Objects { text, templates: 0, edits: Vec::new() };

    objects.visit_program(program);

    objects.edits.into_iter().collect()
}

struct Objects<'t> {
    text: &'t str,
    /// How many template literals enclose the node being visited.
    templates: usize,
    edits: Vec<Edit>,
}

impl Objects<'_> {
    /// The offset just past `object`'s `{` when its properties need splitting.
    fn break_after_brace(&self, object: &ObjectExpression<'_>) -> Option<u32> {
        let first = object.properties.first()?;

        if object.properties.len() < 2 {
            return None;
        }

        let open = object.span.start;
        let gap = self.text.get(open as usize..first.span().start as usize)?;

        (gap.starts_with('{') && !gap.contains('\n')).then_some(open + 1)
    }
}

impl<'a> Visit<'a> for Objects<'_> {
    fn enter_node(&mut self, kind: AstKind<'a>) {
        match kind {
            AstKind::TemplateLiteral(_) => self.templates += 1,
            AstKind::ObjectExpression(object) if self.templates == 0 => {
                if let Some(at) = self.break_after_brace(object) {
                    self.edits.push(Edit::insert(at, "\n"));
                }
            }
            _ => {}
        }
    }

    fn leave_node(&mut self, kind: AstKind<'a>) {
        if matches!(kind, AstKind::TemplateLiteral(_)) {
            self.templates -= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::apply_once;
    use crate::passes::Pass;

    fn object_items(source: &str) -> String {
        apply_once(Pass::ObjectItems, "sample.tsx", source)
    }

    fn full(source: &str) -> String {
        crate::format_source("sample.tsx", fmtkit_core::Lang::Tsx, source, &fmtkit_config::TsFormat::default(), false).expect("the pipeline succeeds").output
    }

    #[test]
    fn breaks_the_line_after_the_brace_of_an_object_with_two_properties() {
        assert_eq!(object_items("const edge = { operation: 'a', surface: 'b' } as const;\n"), "const edge = {\n operation: 'a', surface: 'b' } as const;\n");
    }

    #[test]
    fn leaves_empty_and_single_property_objects_alone() {
        let source = "const a = {};\nconst b = { a: 1 };\nconst c = { ...rest };\n";

        assert_eq!(object_items(source), source);
    }

    #[test]
    fn leaves_objects_that_already_open_on_their_own_line_alone() {
        let source = "const a = {\n\tb: 1,\n\tc: 2,\n};\nconst d = { // why\n\te: 1, f: 2 };\n";

        assert_eq!(object_items(source), source);
    }

    #[test]
    fn leaves_objects_inside_template_literals_alone() {
        let source = "const a = `x ${JSON.stringify({ a: 1, b: 2 })} y`;\nconst b = tag`${{ c: 1, d: 2 }}`;\n";

        assert_eq!(object_items(source), source);
    }

    #[test]
    fn splits_objects_wherever_they_appear() {
        let source = [
            "class Ctx {",
            "\twaitUntil(promise: Promise<unknown>): void {",
            "\t\tconst edge = { operation: `${this.surface}_background`, surface: this.surface } as const;",
            "\t\tconst observed = Boundary.run(this.id.isPresent() ? { ...edge, id: this.id.getOrThrow() } : edge, async () => promise);",
            "\t\tconst el = <div style={{ color: 'red', margin: 0 }} />;",
            "\t\tconst nested = [{ a: 1, b: 2 }, { c: { d: 1, e: 2 }, f: 3 }];",
            "",
            "\t\treturn () => ({ el, nested, observed });",
            "\t}",
            "}",
            "",
        ]
        .join("\n");

        let expected = [
            "class Ctx {",
            "\twaitUntil(promise: Promise<unknown>): void {",
            "\t\tconst edge = {",
            "\t\t\toperation: `${this.surface}_background`,",
            "\t\t\tsurface: this.surface,",
            "\t\t} as const;",
            "\t\tconst observed = Boundary.run(",
            "\t\t\tthis.id.isPresent()",
            "\t\t\t\t? {",
            "\t\t\t\t\t\t...edge,",
            "\t\t\t\t\t\tid: this.id.getOrThrow(),",
            "\t\t\t\t\t}",
            "\t\t\t\t: edge,",
            "\t\t\tasync () => promise,",
            "\t\t);",
            "\t\tconst el = (",
            "\t\t\t<div",
            "\t\t\t\tstyle={{",
            "\t\t\t\t\tcolor: 'red',",
            "\t\t\t\t\tmargin: 0,",
            "\t\t\t\t}}",
            "\t\t\t/>",
            "\t\t);",
            "\t\tconst nested = [",
            "\t\t\t{",
            "\t\t\t\ta: 1,",
            "\t\t\t\tb: 2,",
            "\t\t\t},",
            "\t\t\t{",
            "\t\t\t\tc: {",
            "\t\t\t\t\td: 1,",
            "\t\t\t\t\te: 2,",
            "\t\t\t\t},",
            "\t\t\t\tf: 3,",
            "\t\t\t},",
            "\t\t];",
            "",
            "\t\treturn () => ({",
            "\t\t\tel,",
            "\t\t\tnested,",
            "\t\t\tobserved,",
            "\t\t});",
            "\t}",
            "}",
            "",
        ]
        .join("\n");

        crate::tests::assert_idempotent("sample.tsx", &source);

        assert_eq!(full(&source), expected);
    }
}
