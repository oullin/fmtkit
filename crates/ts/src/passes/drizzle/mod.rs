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
    use super::super::tests::{apply_once, fluent};
    use crate::passes::Pass;

    fn lines(lines: &[&str]) -> String {
        lines.join("\n")
    }

    #[test]
    fn formats_nested_where_predicates_after_fluent_chain_splitting() {
        let input = lines(&[
            "import { and, desc, eq, gt } from 'drizzle-orm';",
            "const rows = await db.select().from(sessions).where(and(eq(sessions.userId, userId), gt(sessions.expiresAt, now))).orderBy(desc(sessions.createdAt));",
            "",
        ]);
        let expected = lines(&[
            "import { and, desc, eq, gt } from 'drizzle-orm';",
            "const rows = await db.select()",
            "\t.from(sessions)",
            "\t.where(",
            "\t\tand(",
            "\t\t\teq(sessions.userId, userId),",
            "\t\t\tgt(sessions.expiresAt, now),",
            "\t\t),",
            "\t)",
            "\t.orderBy(desc(sessions.createdAt));",
            "",
        ]);

        assert_eq!(fluent("fixture.ts", &input), expected);
    }

    #[test]
    fn formats_join_predicates_with_drizzle_helpers() {
        let input = lines(&[
            "import { and, eq, isNull } from 'drizzle-orm';",
            "const rows = await db.select().from(events).leftJoin(users, and(eq(events.userId, users.id), isNull(users.deletedAt)));",
            "",
        ]);
        let expected = lines(&[
            "import { and, eq, isNull } from 'drizzle-orm';",
            "const rows = await db.select()",
            "\t.from(events)",
            "\t.leftJoin(",
            "\t\tusers,",
            "\t\tand(",
            "\t\t\teq(events.userId, users.id),",
            "\t\t\tisNull(users.deletedAt),",
            "\t\t),",
            "\t);",
            "",
        ]);

        assert_eq!(fluent("fixture.ts", &input), expected);
    }

    #[test]
    fn formats_mutation_objects_and_nested_conflict_predicates() {
        let input = lines(&[
            "import { and, eq } from 'drizzle-orm';",
            "await db.insert(users).values({ id: user.id, email: user.email }).onConflictDoUpdate({ target: users.id, set: { email: user.email, updatedAt: now }, where: and(eq(users.id, user.id), eq(users.active, true)) });",
            "",
        ]);
        let expected = lines(&[
            "import { and, eq } from 'drizzle-orm';",
            "await db.insert(users)",
            "\t.values(",
            "\t\t{",
            "\t\t\tid: user.id,",
            "\t\t\temail: user.email,",
            "\t\t},",
            "\t)",
            "\t.onConflictDoUpdate(",
            "\t\t{",
            "\t\t\ttarget: users.id,",
            "\t\t\tset: {",
            "\t\t\t\temail: user.email,",
            "\t\t\t\tupdatedAt: now,",
            "\t\t\t},",
            "\t\t\twhere: and(",
            "\t\t\t\teq(users.id, user.id),",
            "\t\t\t\teq(users.active, true),",
            "\t\t\t),",
            "\t\t},",
            "\t);",
            "",
        ]);

        assert_eq!(fluent("fixture.ts", &input), expected);
    }

    #[test]
    fn formats_relational_query_builder_option_objects() {
        let input = lines(&[
            "import { eq } from 'drizzle-orm';",
            "const users = await db.query.users.findMany({ with: { posts: { with: { comments: true } } }, where: { OR: [{ id: 1 }, { id: 2 }] } });",
            "",
        ]);
        let expected = lines(&[
            "import { eq } from 'drizzle-orm';",
            "const users = await db.query.users.findMany(",
            "\t{",
            "\t\twith: {",
            "\t\t\tposts: {",
            "\t\t\t\twith: { comments: true },",
            "\t\t\t},",
            "\t\t},",
            "\t\twhere: {",
            "\t\t\tOR: [",
            "\t\t\t\t{ id: 1 },",
            "\t\t\t\t{ id: 2 },",
            "\t\t\t],",
            "\t\t},",
            "\t},",
            ");",
            "",
        ]);

        assert_eq!(fluent("fixture.ts", &input), expected);
    }

    #[test]
    fn formats_set_operation_operands() {
        let input =
            lines(&["import { union } from 'drizzle-orm';", "const rows = await union(db.select().from(users), db.select().from(admins)).limit(10);", ""]);
        let expected = lines(&[
            "import { union } from 'drizzle-orm';",
            "const rows = await union(",
            "\tdb.select().from(users),",
            "\tdb.select().from(admins),",
            ").limit(10);",
            "",
        ]);

        assert_eq!(fluent("fixture.ts", &input), expected);
    }

    #[test]
    fn supports_aliased_drizzle_imports() {
        let input = lines(&[
            "import { and as all, eq } from 'drizzle-orm';",
            "const rows = await tx.select().from(users).where(all(eq(users.id, id), eq(users.active, true)));",
            "",
        ]);
        let expected = lines(&[
            "import { and as all, eq } from 'drizzle-orm';",
            "const rows = await tx.select()",
            "\t.from(users)",
            "\t.where(",
            "\t\tall(",
            "\t\t\teq(users.id, id),",
            "\t\t\teq(users.active, true),",
            "\t\t),",
            "\t);",
            "",
        ]);

        assert_eq!(fluent("fixture.ts", &input), expected);
    }

    #[test]
    fn leaves_non_drizzle_helpers_unchanged() {
        let input = lines(&["const rows = await db.select().from(users).where(and(eq(users.id, id), eq(users.active, true)));", ""]);
        let expected = lines(&["const rows = await db.select()", "\t.from(users)", "\t.where(and(eq(users.id, id), eq(users.active, true)));", ""]);

        assert_eq!(fluent("fixture.ts", &input), expected);
    }

    #[test]
    fn does_not_treat_non_db_select_chains_as_drizzle_receivers() {
        let input = lines(&[
            "import { and, eq } from 'drizzle-orm';",
            "const rows = await builder.select().from(users).where(and(eq(users.id, id), eq(users.active, true)));",
            "",
        ]);
        let expected = lines(&[
            "import { and, eq } from 'drizzle-orm';",
            "const rows = await builder.select()",
            "\t.from(users)",
            "\t.where(and(eq(users.id, id), eq(users.active, true)));",
            "",
        ]);

        assert_eq!(fluent("fixture.ts", &input), expected);
    }

    #[test]
    fn skips_commented_drizzle_spans() {
        let input = lines(&[
            "import { and, eq } from 'drizzle-orm';",
            "const rows = await db.select().from(users).where(and(eq(users.id, id), /* keep inline */ eq(users.active, true)));",
            "",
        ]);
        let expected = lines(&[
            "import { and, eq } from 'drizzle-orm';",
            "const rows = await db.select()",
            "\t.from(users)",
            "\t.where(and(eq(users.id, id), /* keep inline */ eq(users.active, true)));",
            "",
        ]);

        assert_eq!(fluent("fixture.ts", &input), expected);
    }

    #[test]
    fn is_idempotent_for_formatted_drizzle_queries() {
        let input = lines(&[
            "import { and, eq } from 'drizzle-orm';",
            "const rows = await db.select()",
            "\t.from(users)",
            "\t.where(",
            "\t\tand(",
            "\t\t\teq(users.id, id),",
            "\t\t\teq(users.active, true),",
            "\t\t),",
            "\t);",
            "",
        ]);

        assert_eq!(fluent("fixture.ts", &fluent("fixture.ts", &input)), input);
    }

    #[test]
    fn nests_with_four_spaces_when_the_source_is_space_indented() {
        let input = lines(&[
            "import { and, eq } from 'drizzle-orm';",
            "function load() {",
            "    const rows = db.select().from(users).where(and(eq(users.id, id), eq(users.active, true)));",
            "}",
            "",
        ]);
        let expected = lines(&[
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
        ]);

        let output = apply_once(Pass::DrizzleQuery, "fixture.ts", &input);

        assert_eq!(output, expected);
        assert!(!output.contains('\t'), "space-indented Drizzle formatting must not introduce tabs");
    }

    #[test]
    fn does_not_process_declaration_files() {
        let input = lines(&[
            "import { and, eq } from 'drizzle-orm';",
            "declare const condition: ReturnType<typeof and>;",
            "declare const rows: typeof db.select().from(users).where(and(eq(users.id, id), eq(users.active, true)));",
            "",
        ]);

        assert_eq!(apply_once(Pass::DrizzleQuery, "fixture.d.ts", &input), input);
    }
}
