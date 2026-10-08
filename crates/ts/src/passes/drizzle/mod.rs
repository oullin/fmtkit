//! Lay out the arguments of Drizzle ORM queries in modules importing `drizzle-orm`.

mod classifier;
mod imports;
mod vocabulary;
mod writer;

use fmtkit_core::EditSet;
use oxc_ast::AstKind;
use oxc_ast::ast::Program;
use oxc_ast_visit::Visit;

use self::classifier::Classifier;
use self::imports::DrizzleImports;
use self::writer::Writer;
use crate::syntax::indent_unit;

/// One argument rewrite per Drizzle method, relational query, or set
/// operation; nested candidates are covered by the outer rewrite.
pub(crate) fn edits<'a>(text: &'a str, program: &'a Program<'a>) -> EditSet {
    let imports = DrizzleImports::scan(program);

    if imports.is_empty() {
        return EditSet::new();
    }

    let classifier = Classifier { imports: &imports };
    let writer = Writer { text, unit: indent_unit(text), comments: &program.comments, classifier: &classifier };
    let mut calls = Calls { writer: &writer, edits: EditSet::new() };

    calls.visit_program(program);

    let mut edits = calls.edits;

    edits.retain_non_overlapping();

    edits
}

struct Calls<'w, 'a> {
    writer: &'w Writer<'w, 'a>,
    edits: EditSet,
}

impl<'a> Visit<'a> for Calls<'_, 'a> {
    fn enter_node(&mut self, kind: AstKind<'a>) {
        let AstKind::CallExpression(call) = kind else { return };
        let classifier = self.writer.classifier;
        let set_operation = classifier.is_set_operation(call);

        if !(set_operation || classifier.is_method_call(call) || classifier.is_relational_query(call)) {
            return;
        }

        let eligible = if set_operation { call.arguments.len() != 1 } else { classifier.formats_method_arguments(call) };

        if eligible && let Some(edit) = self.writer.call(call) {
            self.edits.push(edit);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::apply_once;
    use crate::passes::Pass;

    #[test]
    fn nests_with_four_spaces_when_the_source_is_space_indented() {
        let input = "import { and, eq } from 'drizzle-orm';\nfunction load() {\n    const rows = db.select().from(users).where(and(eq(users.id, id), eq(users.active, true)));\n}\n";
        let expected = [
            "import { and, eq } from 'drizzle-orm';",
            "function load() {",
            "    const rows = db.select().from(users).where(",
            "        and(",
            "            eq(users.id, id),",
            "            eq(users.active, true),",
            "        ),",
            "    );",
            "}",
            "",
        ]
        .join("\n");

        let output = apply_once(Pass::DrizzleQuery, "fixture.ts", input);

        assert_eq!(output, expected);
        assert!(!output.contains('\t'), "space-indented Drizzle formatting must not introduce tabs");
    }

    #[test]
    fn does_not_process_declaration_files() {
        let input = "import { and, eq } from 'drizzle-orm';\ndeclare const condition: ReturnType<typeof and>;\ndeclare const rows: typeof db.select().from(users).where(and(eq(users.id, id), eq(users.active, true)));\n";

        assert_eq!(apply_once(Pass::DrizzleQuery, "fixture.d.ts", input), input);
    }
}
