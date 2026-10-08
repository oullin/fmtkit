//! End-to-end tests ported from the v1 sidecar: the pipelines, the
//! `blank-lines`, `fluent-chains`, `format-all`, and `validate-syntax` CLIs,
//! and the parser. File-system, git, and argument-parsing cases have no
//! counterpart in an in-memory pipeline and are left to the engine and CLI.

use std::fmt::Write;
use std::path::Path;

use fmtkit_config::TsFormat;
use fmtkit_core::{Edit, EditSet, Lang};
use proptest::prelude::*;

use crate::pipeline::{FLUENT, FULL, SEGMENT, Stage, run, run_round};
use crate::{Formatted, TsError, format_embedded, format_source, validate};

fn lines(lines: &[&str]) -> String {
    lines.join("\n")
}

fn lang(rel: &str) -> Lang {
    Lang::from_path(Path::new(rel)).unwrap_or(Lang::Ts)
}

fn schedule(stages: &[Stage], rel: &str, source: &str) -> Result<Formatted, TsError> {
    run(rel, crate::syntax::source_type(lang(rel), rel).expect("a script"), source, &TsFormat::default(), false, stages)
}

/// One round of `stages`, where a stage budget is the last word.
fn round(stages: &[Stage], rel: &str, source: &str) -> String {
    run_round(rel, crate::syntax::source_type(lang(rel), rel).expect("a script"), source, stages).expect("the round succeeds").output
}

fn segment(source: &str, rel: &str) -> String {
    schedule(&SEGMENT, rel, source).expect("the segment pipeline succeeds").output
}

fn fluent(source: &str) -> String {
    schedule(&FLUENT, "fixture.ts", source).expect("the fluent pipeline succeeds").output
}

fn full(source: &str) -> String {
    format_source("fixture.ts", Lang::Ts, source, &TsFormat::default(), false).expect("the pipeline succeeds").output
}

/// The whole pipeline over `source` succeeds when the source parses, and a
/// second run changes nothing and reports nothing. Returns whether it parsed.
pub(crate) fn assert_idempotent(rel: &str, source: &str) -> bool {
    let lang = lang(rel);
    let once = match format_source(rel, lang, source, &TsFormat::default(), false) {
        Ok(once) => once,
        Err(TsError::Syntax { .. }) => return false,
        Err(error) => panic!("{rel}: {error}\n--- input ---\n{source}"),
    };

    let twice =
        format_source(rel, lang, &once.output, &TsFormat::default(), false).unwrap_or_else(|error| panic!("{rel}: {error}\n--- once ---\n{}", once.output));

    assert_eq!(twice.output, once.output, "{rel}: a second run changed the output of\n{source}");
    assert!(twice.applied.is_empty(), "{rel}: {:?}", twice.applied);

    true
}

mod pass_pipeline {
    use super::*;

    fn nested_ifs(depth: usize) -> String {
        let mut source = String::new();

        for level in 0..depth {
            write!(source, "if (c{level}) ").expect("writes to a string");
        }

        source.push_str("run();\n");

        source
    }

    #[test]
    fn a_once_budget_caps_a_step_at_a_single_application() {
        // Expanded calls lift one nesting level per run; once leaves the inner one.
        let input = lines(&["const app = createApp({", "\troot: defineComponent({ a: [1] }),", "});", ""]);
        let output = round(&FLUENT, "fixture.ts", &input);

        assert!(output.starts_with("const app = createApp(\n\t{\n\t\troot: defineComponent({ a: [1] }),"), "{output}");
    }

    #[test]
    fn an_until_stable_budget_re_runs_a_pass_until_it_proposes_nothing() {
        let output = segment(&nested_ifs(3), "sample.ts");

        assert_eq!(output, "if (c0) {\n\tif (c1) {\n\t\tif (c2) {\n\t\t\trun();\n\t\t}\n\t}\n}\n");
    }

    #[test]
    fn an_until_stable_budget_stops_at_its_limit() {
        let output = round(&SEGMENT, "sample.ts", &nested_ifs(6));

        assert_eq!(output.matches('{').count(), 5, "{output}");
        assert!(output.contains("if (c5) run();"), "{output}");
    }

    #[test]
    fn a_settled_document_is_left_untouched_and_reports_nothing() {
        let formatted = schedule(&SEGMENT, "sample.ts", "const a = 1;\n").expect("succeeds");

        assert_eq!(formatted.output, "const a = 1;\n");
        assert_eq!(formatted.applied, Vec::<&str>::new());
    }

    #[test]
    fn an_empty_schedule_returns_its_input_unchanged() {
        let formatted = schedule(&[], "sample.ts", "const a = 1;\n").expect("succeeds");

        assert_eq!(formatted.output, "const a = 1;\n");
    }

    #[test]
    fn a_schedule_runs_every_step_in_order_past_no_op_steps() {
        let formatted = schedule(&SEGMENT, "sample.ts", "class A {\n\trun() {}\n\ta = 1;\n}\nif (a) b();\n").expect("succeeds");

        assert_eq!(formatted.output, "class A {\n\ta = 1;\n\n\trun() {}\n}\nif (a) {\n\tb();\n}\n");
        assert_eq!(formatted.applied, ["body-wrap", "class-reorder", "blank-lines"]);
    }

    #[test]
    fn steps_report_the_v1_names() {
        let formatted =
            format_source("fixture.ts", Lang::Ts, "const rows = make( ).use(a).get(b)\nif (a) b()\n", &TsFormat::default(), false).expect("succeeds");

        assert_eq!(formatted.applied, ["body-wrap", "blank-lines", "oxfmt", "fluent-chains"]);
        assert_eq!(schedule(&FULL, "fixture.ts", "const rows = make( ).use(a).get(b)\nif (a) b()\n"), Ok(formatted));
    }
}

mod segment_pipeline {
    use super::*;

    struct Case {
        name: &'static str,
        input: &'static [&'static str],
        expected: &'static [&'static str],
    }

    const CASES: &[Case] = &[
        Case {
            name: "import followed by export function gets a blank line",
            input: &["import { foo } from \"node:foo\";", "export function bar() {", "\treturn foo();", "}", ""],
            expected: &["import { foo } from \"node:foo\";", "", "export function bar() {", "\treturn foo();", "}", ""],
        },
        Case {
            name: "multiple imports get a blank line only after the last one",
            input: &[
                "import { a } from \"node:a\";",
                "import { b } from \"node:b\";",
                "import { c } from \"node:c\";",
                "export function run() {",
                "\treturn a(b(c()));",
                "}",
                "",
            ],
            expected: &[
                "import { a } from \"node:a\";",
                "import { b } from \"node:b\";",
                "import { c } from \"node:c\";",
                "",
                "export function run() {",
                "\treturn a(b(c()));",
                "}",
                "",
            ],
        },
        Case {
            name: "consecutive imports stay tight",
            input: &["import { a } from \"node:a\";", "import { b } from \"node:b\";", "", "export function run() {", "\treturn a(b());", "}", ""],
            expected: &["import { a } from \"node:a\";", "import { b } from \"node:b\";", "", "export function run() {", "\treturn a(b());", "}", ""],
        },
        Case {
            name: "TS enum followed by function gets blank lines on both sides",
            input: &["import { x } from \"node:x\";", "enum Colour {", "\tRed,", "\tBlue,", "}", "function paint() {", "\treturn Colour.Red;", "}", ""],
            expected: &[
                "import { x } from \"node:x\";",
                "",
                "enum Colour {",
                "\tRed,",
                "\tBlue,",
                "}",
                "",
                "function paint() {",
                "\treturn Colour.Red;",
                "}",
                "",
            ],
        },
        Case {
            name: "TS namespace (module) followed by function gets blank lines on both sides",
            input: &["namespace Utils {", "\texport const value = 1;", "}", "function consume() {", "\treturn Utils.value;", "}", ""],
            expected: &["namespace Utils {", "\texport const value = 1;", "}", "", "function consume() {", "\treturn Utils.value;", "}", ""],
        },
        Case {
            name: "inline statement bodies are wrapped",
            input: &[
                "function run() {",
                "\tif (a) b(); else if (c) d(); else e();",
                "\tfor (const item of items) consume(item);",
                "\tconst fn = (value: number) => value + 1;",
                "}",
                "",
            ],
            expected: &[
                "function run() {",
                "\tif (a) {",
                "\t\tb();",
                "\t} else if (c) {",
                "\t\td();",
                "\t} else {",
                "\t\te();",
                "\t}",
                "\tfor (const item of items) {",
                "\t\tconsume(item);",
                "\t}",
                "",
                "\tconst fn = (value: number) => value + 1;",
                "}",
                "",
            ],
        },
        Case {
            name: "nested if statement bodies are wrapped except else-if chains",
            input: &["function run() {", "\tif (a) if (b) c();", "\tfor (const item of items) if (item.ready) consume(item);", "}", ""],
            expected: &[
                "function run() {",
                "\tif (a) {",
                "\t\tif (b) {",
                "\t\t\tc();",
                "\t\t}",
                "\t}",
                "\tfor (const item of items) {",
                "\t\tif (item.ready) {",
                "\t\t\tconsume(item);",
                "\t\t}",
                "\t}",
                "}",
                "",
            ],
        },
        Case {
            name: "await statements are isolated from adjacent code",
            input: &["async function run() {", "\tconst before = 1;", "\tawait work();", "\tconst after = 2;", "}", ""],
            expected: &["async function run() {", "\tconst before = 1;", "", "\tawait work();", "", "\tconst after = 2;", "}", ""],
        },
        Case {
            name: "await inside nested functions does not isolate parent statements",
            input: &["function run() {", "\tconst onClick = async () => await work();", "\tconst after = 1;", "}", ""],
            expected: &["function run() {", "\tconst onClick = async () => await work();", "\tconst after = 1;", "}", ""],
        },
        Case {
            name: "Vue primitive const declarations get a blank line above",
            input: &["function setupState() {", "\tconst before = 1;", "\tconst value = computed(() => 1);", "\tconst after = 2;", "}", ""],
            expected: &["function setupState() {", "\tconst before = 1;", "", "\tconst value = computed(() => 1);", "\tconst after = 2;", "}", ""],
        },
        Case {
            name: "expression followed by let gets a blank line",
            input: &["function run() {", "\tdoWork();", "\tlet value = 1;", "\tlet next = 2;", "}", ""],
            expected: &["function run() {", "\tdoWork();", "", "\tlet value = 1;", "\tlet next = 2;", "}", ""],
        },
        Case {
            name: "let followed by expression gets a blank line",
            input: &["function run() {", "\tlet value = 1;", "\tlet next = 2;", "\tdoWork(value + next);", "}", ""],
            expected: &["function run() {", "\tlet value = 1;", "\tlet next = 2;", "", "\tdoWork(value + next);", "}", ""],
        },
        Case {
            name: "const followed by let gets a blank line",
            input: &["function run() {", "\tconst before = 1;", "\tlet value = before;", "}", ""],
            expected: &["function run() {", "\tconst before = 1;", "", "\tlet value = before;", "}", ""],
        },
        Case {
            name: "consecutive lets stay tight",
            input: &["function run() {", "\tlet value = 1;", "\tlet next = 2;", "\treturn value + next;", "}", ""],
            expected: &["function run() {", "\tlet value = 1;", "\tlet next = 2;", "", "\treturn value + next;", "}", ""],
        },
        Case {
            name: "binding calls start a spaced block",
            input: &[
                "function register(application: Application) {",
                "\tthis.application = application;",
                "\tapplication.bindings.singleton(Service, makeService);",
                "\tapplication.bindings.instance(Config, config);",
                "}",
                "",
            ],
            expected: &[
                "function register(application: Application) {",
                "\tthis.application = application;",
                "",
                "\tapplication.bindings.singleton(Service, makeService);",
                "\tapplication.bindings.instance(Config, config);",
                "}",
                "",
            ],
        },
        Case {
            name: "multiline imports and consts move last in their groups",
            input: &[
                "import { z } from \"z\";",
                "import {",
                "\ta,",
                "} from \"a\";",
                "import { y } from \"y\";",
                "const b = 1;",
                "const a = {",
                "\tx: 1,",
                "};",
                "const c = 2;",
                "",
            ],
            expected: &[
                "import { z } from \"z\";",
                "import { y } from \"y\";",
                "",
                "import {",
                "\ta,",
                "} from \"a\";",
                "",
                "const b = 1;",
                "const c = 2;",
                "",
                "const a = {",
                "\tx: 1,",
                "};",
                "",
            ],
        },
        Case {
            name: "multiline consts with nested side effects keep their order",
            input: &["const config = {", "\tvalue: makeValue(),", "};", "const next = 1;", ""],
            expected: &["const config = {", "\tvalue: makeValue(),", "};", "", "const next = 1;", ""],
        },
        Case {
            name: "multiline destructuring consts keep their order",
            input: &["const { value } = {", "\tvalue: 1,", "};", "const next = value;", ""],
            expected: &["const { value } = {", "\tvalue: 1,", "};", "", "const next = value;", ""],
        },
    ];

    #[test]
    fn blank_line_rules() {
        for case in CASES {
            let input = lines(case.input);

            assert_eq!(segment(&input, "fixture.ts"), lines(case.expected), "{}", case.name);
            assert!(assert_idempotent("fixture.ts", &input));
        }
    }

    #[test]
    fn is_idempotent_running_twice_produces_no_further_changes() {
        let input = lines(&["import { a } from \"node:a\";", "import { b } from \"node:b\";", "export function run() {", "\treturn a(b());", "}", ""]);
        let once = segment(&input, "fixture.ts");

        assert_eq!(segment(&once, "fixture.ts"), once);
    }
}

mod blank_lines_cli {
    use super::*;

    #[test]
    fn adds_expected_blank_lines_in_vue_script_blocks() {
        let script = lines(&[
            "",
            "const hoveredItem = computed(() => {",
            "    const id = hoveredId.value;",
            "    if (!id) return null;",
            "    if (id in systemLabels) return { id, label: systemLabels[id], shortcut: undefined };",
            "    return props.items.find((i) => i.id === id) ?? null;",
            "});",
            "",
        ]);
        let output = segment(&script, "AgentDock.vue.ts");

        assert!(output.contains("const hoveredItem = computed(() => {\n    const id = hoveredId.value;\n\n    if (!id) {"), "{output}");
        assert!(output.contains("return null;\n    }\n\n    if (id in systemLabels) {"), "{output}");
        assert!(output.contains("shortcut: undefined };\n    }\n\n    return props.items"), "{output}");
        assert!(assert_idempotent("AgentDock.vue.ts", &script));
    }

    #[test]
    fn keeps_adjacent_computed_declarations_syntactically_valid() {
        let input = lines(&[
            "import { computed, ref } from \"vue\";",
            "",
            "export function useAppController() {",
            "\tconst searchQuery = ref(\"\");",
            "\tconst debouncedSearch = ref(\"\");",
            "\tconst normalizedSearch = computed(() => searchQuery.value.trim().toLowerCase());",
            "\tconst normalizedDebouncedSearch = computed(() => debouncedSearch.value.trim().toLowerCase());",
            "",
            "\treturn { normalizedSearch, normalizedDebouncedSearch };",
            "}",
            "",
        ]);
        let output = segment(&segment(&input, "useAppController.ts"), "useAppController.ts");

        assert!(!output.contains(");););"));
        assert!(output.contains("const normalizedDebouncedSearch = computed(() => debouncedSearch.value.trim().toLowerCase());"), "{output}");
        assert!(assert_idempotent("useAppController.ts", &input));
    }

    #[test]
    fn spaces_tracked_files_and_ignores_declaration_files() {
        let tracked = lines(&["function run() {", "\tconst value = 1;", "\tif (value) return value;", "\treturn 0;", "}", ""]);
        let types = lines(&["declare const value: string;", "declare function run(): string;", ""]);

        assert!(segment(&tracked, "tracked.ts").contains("const value = 1;\n\n\tif (value) {\n\t\treturn value;\n\t}\n\n\treturn 0;"));
        assert_eq!(segment(&types, "types.d.ts"), types);
        assert_eq!(format_source("types.d.ts", Lang::Ts, &types, &TsFormat::default(), false).expect("parses").output, types);
    }

    #[test]
    fn spaces_a_return_after_a_declaration() {
        let kept = lines(&["function run() {", "\tconst value = 1;", "\treturn value;", "}", ""]);

        assert!(segment(&kept, "kept.ts").contains("const value = 1;\n\n\treturn value;"));
    }

    #[test]
    fn adds_blank_lines_above_loops_after_simple_statements_only() {
        let input = lines(&[
            "function run(items: string[], map: Record<string, string>) {",
            "\tlet count = 0;",
            "\tcount++;",
            "\tfor (; count < 1; count++) {",
            "\t\tcount++;",
            "\t}",
            "\tcount++;",
            "\tfor (const item of items) {",
            "\t\tconsole.log(item);",
            "\t}",
            "\tcount++;",
            "\tfor (const key in map) {",
            "\t\tconsole.log(key);",
            "\t}",
            "\tcount++;",
            "\twhile (count < 3) {",
            "\t\tcount++;",
            "\t}",
            "\tcount++;",
            "\tdo {",
            "\t\tcount++;",
            "\t} while (count < 4);",
            "\tfunction local() {",
            "\t\treturn count;",
            "\t}",
            "\tfor (; count < local(); count++) {",
            "\t\tcount++;",
            "\t}",
            "\tif (count > 10) {",
            "\t\tcount--;",
            "\t}",
            "\twhile (count > 0) {",
            "\t\tcount--;",
            "\t}",
            "\ttype LoopCount = number;",
            "\twhile (count < Number.MAX_SAFE_INTEGER) {",
            "\t\tconst typedCount: LoopCount = count;",
            "\t\tcount = typedCount + 1;",
            "\t}",
            "}",
            "",
        ]);
        let output = segment(&segment(&input, "loops.ts"), "loops.ts");

        for expected in [
            "count++;\n\n\tfor (; count < 1; count++) {",
            "count++;\n\n\tfor (const item of items) {",
            "count++;\n\n\tfor (const key in map) {",
            "count++;\n\n\twhile (count < 3) {",
            "count++;\n\n\tdo {",
            "function local() {\n\t\treturn count;\n\t}\n\tfor (; count < local(); count++) {",
            "if (count > 10) {\n\t\tcount--;\n\t}\n\twhile (count > 0) {",
            "type LoopCount = number;\n\n\twhile (count < Number.MAX_SAFE_INTEGER) {",
        ] {
            assert!(output.contains(expected), "missing {expected:?} in\n{output}");
        }

        assert!(assert_idempotent("loops.ts", &input));
    }

    #[test]
    fn uses_the_modular_formatter_for_body_wrapping_and_declaration_ordering() {
        let input = lines(&["import {", "\ta,", "} from \"a\";", "import { b } from \"b\";", "function run() {", "\tif (ready) done();", "}", ""]);
        let output = segment(&input, "tracked.ts");

        assert!(output.contains("import { b } from \"b\";\n\nimport {\n\ta,\n} from \"a\";"), "{output}");
        assert!(output.contains("if (ready) {\n\t\tdone();\n\t}"), "{output}");
        assert!(assert_idempotent("tracked.ts", &input));
    }
}

mod fluent_chains_cli {
    use super::*;

    #[test]
    fn splits_router_builder_chains_onto_independent_lines() {
        let input = lines(&[
            "export const meRoutes = createRouter<{ Bindings: WorkerEnv; Variables: IdentityVariables }>().use('*', bindEnv).use(identityMiddleware).get('/', getMe).get('/sessions', getSessions);",
            "",
        ]);
        let expected = lines(&[
            "export const meRoutes = createRouter<{ Bindings: WorkerEnv; Variables: IdentityVariables }>()",
            "\t.use('*', bindEnv)",
            "\t.use(identityMiddleware)",
            "\t.get('/', getMe)",
            "\t.get('/sessions', getSessions);",
            "",
        ]);

        assert_eq!(fluent(&input), expected);
        assert_eq!(full(&input), expected);
    }

    #[test]
    fn is_idempotent_for_already_split_chains() {
        let input = lines(&["const routes = createRouter()", "\t.use('*', bindEnv)", "\t.get('/', getMe);", ""]);

        assert_eq!(fluent(&fluent(&input)), input);
        assert_eq!(full(&input), input);
    }

    #[test]
    fn uses_the_file_indentation_style_for_split_chains() {
        let input = lines(&["function routes() {", "  return createRouter().use('*', bindEnv).get('/', getMe);", "}", ""]);
        let expected = lines(&["function routes() {", "  return createRouter()", "    .use('*', bindEnv)", "    .get('/', getMe);", "}", ""]);

        assert_eq!(fluent(&input), expected);
        assert!(assert_idempotent("fixture.ts", &input));
    }

    #[test]
    fn splits_chains_with_four_spaces_when_the_source_is_space_indented() {
        let input = lines(&["function routes() {", "    return createRouter().use('*', bindEnv).get('/', getMe);", "}", ""]);
        let expected = lines(&["function routes() {", "    return createRouter()", "        .use('*', bindEnv)", "        .get('/', getMe);", "}", ""]);
        let output = fluent(&input);

        assert_eq!(output, expected);
        assert!(!output.contains('\t'), "space-indented chain splitting must not introduce tabs");
    }

    #[test]
    fn indents_split_chains_one_unit_past_a_baseline_indented_block() {
        let input = lines(&["\t\t\tconst result = builder().withA(1).withB(2).withC(3).build();", ""]);
        let expected = lines(&["\t\t\tconst result = builder()", "\t\t\t\t.withA(1)", "\t\t\t\t.withB(2)", "\t\t\t\t.withC(3)", "\t\t\t\t.build();", ""]);
        let output = fluent(&input);

        assert_eq!(output, expected);
        assert!(!output.contains("\t\t\t\t\t"), "continuations must be base plus one unit, never doubled");
    }

    #[test]
    fn leaves_short_value_transform_chains_unchanged() {
        let input = lines(&["const normalized = value.trim().toLowerCase();", ""]);

        assert_eq!(fluent(&input), input);
        assert_eq!(full(&input), input);
    }

    #[test]
    fn preserves_optional_chain_operators() {
        let input = lines(&["const result = makeClient()?.use(auth).get('/');", ""]);
        let expected = lines(&["const result = makeClient()", "\t?.use(auth)", "\t.get('/');", ""]);

        assert_eq!(fluent(&input), expected);
        assert_eq!(full(&input), expected);
    }

    #[test]
    fn skips_chains_with_comments_between_links() {
        let input = lines(&["const routes = createRouter()", "\t// attach middleware first", "\t.use('*', bindEnv).get('/', getMe);", ""]);

        assert_eq!(fluent(&input), input);
        assert!(assert_idempotent("fixture.ts", &input));
    }

    #[test]
    fn reaches_a_fixed_point_over_an_expanded_multiline_template_literal() {
        let interior = ["        <div>", "            <span>hello</span>", "        </div>"];
        let input = lines(&["const Harness = defineComponent({", "    template: `", interior[0], interior[1], interior[2], "    `,", "});", ""]);
        let once = fluent(&input);
        let twice = fluent(&once);

        assert_eq!(twice, once);
        assert_eq!(fluent(&twice), once);
        assert!(once.contains(&interior.join("\n")), "the literal interior must keep its original bytes");
        assert!(full(&input).contains(&interior.join("\n")), "the formatter must keep the literal interior too");
        assert!(assert_idempotent("fixture.ts", &input));
    }

    /// The script bodies of v1's Vue, HTML, and Markdown host cases; the host
    /// split itself belongs to `fmtkit-hosts`.
    #[test]
    fn formats_embedded_router_chains() {
        let vue = "\nexport const meRoutes = createRouter<{ Bindings: WorkerEnv; Variables: IdentityVariables }>().use('*', bindEnv).use(identityMiddleware).get('/', getMe).get('/sessions', getSessions);\n";
        let output = fluent(&fluent(vue));

        assert!(
            output.contains("createRouter<{ Bindings: WorkerEnv; Variables: IdentityVariables }>()\n\t.use('*', bindEnv)\n\t.use(identityMiddleware)"),
            "{output}"
        );
        assert!(output.contains(".get('/', getMe)\n\t.get('/sessions', getSessions);"), "{output}");

        let script = "\nexport const meRoutes = createRouter().use('*', bindEnv).use(identityMiddleware).get('/', getMe);\n";
        let split = "createRouter()\n\t.use('*', bindEnv)\n\t.use(identityMiddleware)\n\t.get('/', getMe);";

        assert!(fluent(&fluent(script)).contains(split));
        assert!(format_embedded(Lang::Ts, script, &TsFormat::default()).expect("formats").contains(split));
        assert!(format_embedded(Lang::Js, script, &TsFormat::default()).expect("formats").contains(split));

        let short = "\nexport const meRoutes = createRouter().use('*', bindEnv).get('/', getMe);\n";

        assert!(fluent(short).contains("createRouter()\n\t.use('*', bindEnv)\n\t.get('/', getMe);"));
    }

    #[test]
    fn formats_embedded_drizzle_queries() {
        let script = "\nimport { and, eq, gt } from 'drizzle-orm';\nconst rows = await db.select().from(sessions).where(and(eq(sessions.userId, userId), gt(sessions.expiresAt, now)));\n";
        let expected = ".where(\n\t\tand(\n\t\t\teq(sessions.userId, userId),\n\t\t\tgt(sessions.expiresAt, now),\n\t\t),\n\t);";

        assert!(fluent(&fluent(script)).contains(expected));
        assert!(format_embedded(Lang::Ts, script, &TsFormat::default()).expect("formats").contains(expected));
    }

    #[test]
    fn formats_embedded_expanded_call_arguments() {
        let script = "\nfunction createAuth() {\n\treturn betterAuth(buildAuthConfig(env, db, mailer, app, options)) as unknown as SasuAuth;\n}\n";
        let expected = "return betterAuth(\n\t\tbuildAuthConfig(env, db, mailer, app, options),\n\t) as unknown as SasuAuth;";

        assert!(fluent(&fluent(script)).contains(expected));
        assert!(format_embedded(Lang::Ts, script, &TsFormat::default()).expect("formats").contains(expected));
    }
}

mod format_all_cli {
    use super::*;

    #[test]
    fn formats_a_file_end_to_end_in_schedule_order() {
        let formatted =
            format_source("app.ts", Lang::Ts, "function run() {\n\tconst x = 1;\n\tif (x) return x;\n\treturn 0;\n}\n", &TsFormat::default(), false)
                .expect("formats");

        assert!(formatted.output.contains("if (x) {\n\t\treturn x;\n\t}"), "{}", formatted.output);
        assert_eq!(formatted.applied, ["body-wrap", "blank-lines"]);
    }

    #[test]
    fn reaches_a_fixed_point_in_a_single_run() {
        // A single-line statement followed by a call that expanded-calls turns
        // multiline: the blank line separating them can only be inserted after
        // the expansion has happened.
        let input = "function queueMessage(id: string, body: object) {\n\treturn { id, body };\n}\n\nexport function run() {\n\tconst registry = new Map<string, string>();\n\tconst invalid = queueMessage('1', { unexpected: true });\n\treturn [registry, invalid];\n}\n";
        let once = full(input);

        assert!(once.contains("queueMessage(\n\t\t'1',"), "expected the call to be expanded: {once}");
        assert_eq!(full(&once), once, "a second run must not change an already formatted file");
        assert!(assert_idempotent("app.ts", input));
    }

    #[test]
    fn formats_embedded_router_chains_through_the_whole_schedule() {
        let chain = "export const meRoutes = createRouter().use('*', bindEnv).use(identityMiddleware).get('/', getMe);\n";
        let split = "createRouter()\n\t.use('*', bindEnv)\n\t.use(identityMiddleware)\n\t.get('/', getMe);";

        assert!(format_embedded(Lang::Ts, chain, &TsFormat::default()).expect("formats").contains(split));
    }

    #[test]
    fn reports_syntax_errors_in_embedded_and_standalone_scripts() {
        assert!(matches!(format_embedded(Lang::Js, "const broken = {;\n", &TsFormat::default()), Err(TsError::Syntax { line: 1, .. })));
        assert!(matches!(format_source("broken.ts", Lang::Ts, "const broken = {;\n", &TsFormat::default(), false), Err(TsError::Syntax { line: 1, .. })));
    }
}

mod validate_syntax_cli {
    use super::*;

    #[test]
    fn accepts_valid_typescript_and_script_blocks() {
        assert_eq!(validate("valid.ts", Lang::Ts, "const value = computed(() => source.value.trim().toLowerCase());\n"), Ok(()));
        assert_eq!(validate("Valid.vue.ts", Lang::Ts, "\nconst value = 1;\n"), Ok(()));
    }

    #[test]
    fn accepts_tsx_and_jsx_script_blocks() {
        assert_eq!(validate("Component.vue.tsx", Lang::Tsx, "\nconst view = <section>Ready</section>;\n"), Ok(()));
        assert_eq!(validate("Legacy.vue.jsx", Lang::Jsx, "\nconst view = <section>Ready</section>;\n"), Ok(()));
    }

    #[test]
    fn accepts_standalone_tsx_mts_and_cts_files() {
        assert_eq!(validate("legacy.cts", Lang::Cts, "export const value: number = 1;\n"), Ok(()));
        assert_eq!(validate("loader.mts", Lang::Mts, "export const load = async (): Promise<number> => 1;\n"), Ok(()));
        assert_eq!(validate("Screen.tsx", Lang::Tsx, "export const Screen = (): JSX.Element => <section>Ready</section>;\n"), Ok(()));
    }

    #[test]
    fn reports_tsx_syntax_errors_on_original_file_lines() {
        assert!(matches!(validate("Broken.tsx", Lang::Tsx, "export const Screen = () => <section>Ready</div>;\n"), Err(TsError::Syntax { line: 1, .. })));
    }

    #[test]
    fn reports_host_script_errors_on_original_file_lines() {
        // v1 blanked the host text before the script so lines stay in place.
        let virtual_content = "\n\n\n\nconst broken = ;\n";

        assert!(matches!(validate("Broken.vue.ts", Lang::Ts, virtual_content), Err(TsError::Syntax { line: 5, column: 16, .. })));
    }

    #[test]
    fn fails_with_a_clear_diagnostic_for_corrupted_formatter_output() {
        let result =
            validate("useAppController.ts", Lang::Ts, "const normalizedDebouncedSearch = computed(() => debouncedSearch.value.trim().toLowerCase()););\n");

        assert!(matches!(&result, Err(TsError::Syntax { message, .. }) if message.contains("Unexpected token")), "{result:?}");
    }

    #[test]
    fn does_not_treat_host_languages_as_scripts() {
        assert!(matches!(validate("notes.md", Lang::Markdown, "# Notes\n"), Err(TsError::Invariant { step: "parse", .. })));
    }

    #[test]
    fn parses_valid_source_with_comments_and_rejects_syntax_errors() {
        let allocator = oxc_allocator::Allocator::default();
        let program = crate::syntax::parse(&allocator, "const one = 1; // note\n", oxc_span::SourceType::ts()).expect("parses");

        assert_eq!(program.comments.len(), 1);
        assert!(crate::syntax::parse(&allocator, "const broken = {;\n", oxc_span::SourceType::ts()).is_err());
    }

    #[test]
    fn scores_through_the_validation_parse() {
        assert!(matches!(crate::score("broken.ts", Lang::Ts, "const broken = {;\n"), Err(TsError::Syntax { line: 1, .. })));
        assert_eq!(crate::score("ok.ts", Lang::Ts, "function f(a) { if (a) { return 1; } return 2; }\n").map(|scores| scores.len()), Ok(1));
    }
}

mod edits {
    use super::*;

    #[test]
    fn retaining_non_overlapping_edits_drops_overlaps_and_sorts_by_start() {
        let mut edits: EditSet = [Edit::new(10, 20, "b"), Edit::new(0, 5, "a"), Edit::new(15, 25, "c")].into_iter().collect();

        edits.retain_non_overlapping();

        assert_eq!(edits.iter().map(|edit| edit.start).collect::<Vec<_>>(), [0, 10]);
    }

    fn edit_case() -> impl Strategy<Value = (String, Vec<Edit>)> {
        "[a-z ]{1,60}".prop_flat_map(|source| {
            let length = u32::try_from(source.len()).expect("short");
            let edit = (0..length, 1..=length, "[a-z]{0,12}").prop_map(move |(start, width, text)| Edit::new(start, (start + width).min(length), text));

            (Just(source), proptest::collection::vec(edit, 0..30))
        })
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 100, failure_persistence: None, ..ProptestConfig::default() })]

        #[test]
        fn retained_edits_are_a_sorted_non_overlapping_subset((_source, edits) in edit_case()) {
            let mut set: EditSet = edits.iter().cloned().collect();

            set.retain_non_overlapping();

            let kept: Vec<&Edit> = set.iter().collect();

            for (index, edit) in kept.iter().enumerate() {
                prop_assert!(edits.contains(edit));
                prop_assert!(index == 0 || kept[index - 1].start <= edit.start);
                prop_assert!(kept[index + 1..].iter().all(|next| !edit.overlaps(next)));
            }
        }

        #[test]
        fn applying_retained_edits_matches_applying_them_right_to_left((source, edits) in edit_case()) {
            let mut set: EditSet = edits.into_iter().collect();

            set.retain_non_overlapping();

            let mut individually = source.clone();

            for edit in set.iter().collect::<Vec<_>>().into_iter().rev() {
                individually.replace_range(edit.start as usize..edit.end as usize, &edit.text);
            }

            prop_assert_eq!(set.apply(&source).expect("non-overlapping"), individually);
        }
    }
}

mod idempotency {
    use super::*;

    const NAMES: [&str; 5] = ["alpha", "beta", "gamma", "delta", "epsilon"];

    /// v1's segment property generator: consts, conditional functions,
    /// unordered classes, imports, templates, and comments.
    fn segment_statement() -> impl Strategy<Value = String> {
        let name = proptest::sample::select(NAMES.as_slice());
        let value = -10i32..=10;

        prop_oneof![
            (name.clone(), value.clone()).prop_map(|(name, value)| format!("const {name} = {value};")),
            (name.clone(), value.clone()).prop_map(|(name, value)| format!("const {name} = {{\n\tvalue: {value},\n}};")),
            (name.clone(), value.clone())
                .prop_map(|(name, fallback)| format!("function read{name}(value: number) {{\n\tif (value > 0) return value;\n\treturn {fallback};\n}}")),
            (name.clone(), value).prop_map(|(name, value)| format!(
                "class {name}Model {{\n\tread() {{\n\t\treturn this.value;\n\t}}\n\tvalue = {value};\n\tconstructor() {{}}\n}}"
            )),
            proptest::sample::select(["alpha-package", "beta-package", "gamma-package"].as_slice()).prop_map(|module| format!("import '{module}';")),
            name.prop_map(|name| format!("const {name}Label = `value-${{1}}`;")),
            proptest::sample::select(["// formatter note", "/* formatter block note */"].as_slice()).prop_map(str::to_owned),
        ]
    }

    /// Statements that exercise every pass: nested unbraced bodies, chains,
    /// Drizzle queries, expandable calls, awaits, lets, JSX-free TS types.
    fn program_statement() -> impl Strategy<Value = String> {
        let name = proptest::sample::select(NAMES.as_slice());

        prop_oneof![
            segment_statement(),
            name.clone().prop_map(|name| format!("export const {name}Routes = createRouter().use('*', bind).get('/', {name}).get('/x', other);")),
            name.clone().prop_map(|name| format!("const {name}Rows = await db.select().from(users).where(and(eq(users.id, {name}), eq(users.active, true)));")),
            name.clone().prop_map(|name| format!("const {name}Value = resolveConfig(prefix, buildOptions({name}), {{ strict: true }});")),
            name.clone().prop_map(|name| format!("for (const item of {name}) if (item) for (;;) while (item) use(item);")),
            name.clone().prop_map(|name| format!("let {name}Count = 0;\n{name}Count++;\nawait flush({name}Count);")),
            name.clone().prop_map(|name| format!("type {name}Shape = {{ a: string; 'b-c': number }} | null;")),
            name.clone().prop_map(|name| format!(
                "class {name}Outer {{\n\trun() {{\n\t\treturn class {{\n\t\t\tgo() {{}}\n\t\t\tb = 2;\n\t\t}};\n\t}}\n\ta = (1);\n}}"
            )),
            name.prop_map(|name| format!(
                "function {name}Nested() {{\n\tconst inner = {{\n\t\ta: 1,\n\t}};\n\tconst x = 1;\n\tif (x) return [inner, x]; else return null;\n}}"
            )),
        ]
    }

    fn source(statement: impl Strategy<Value = String>) -> impl Strategy<Value = String> {
        proptest::collection::vec(statement, 1..=8).prop_map(|statements| {
            let mut source = statements.join("\n");

            source.push('\n');

            source
        })
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 100, failure_persistence: None, ..ProptestConfig::default() })]

        #[test]
        fn the_segment_pipeline_is_idempotent_for_composed_sources(source in source(segment_statement())) {
            let once = segment(&source, "property.ts");

            prop_assert_eq!(segment(&once, "property.ts"), once);
        }

        #[test]
        fn the_whole_pipeline_is_idempotent_for_composed_sources(source in source(program_statement())) {
            let nested = format!("async function main() {{\n{}}}\n", source.replace("import '", "load('").replace("';", "');"));

            prop_assert!(assert_idempotent("property.ts", &source));
            prop_assert!(assert_idempotent("property.ts", &nested));
        }
    }
}

mod fixed_point {
    use std::cell::Cell;

    use super::*;
    use crate::pipeline::{EXTRA_ROUNDS, IDEMPOTENCY, fixed_point};

    /// A round that appends `suffix` to its input while the input is shorter
    /// than `until`, reporting `step` when it changes anything.
    fn growing<'c>(calls: &'c Cell<usize>, until: usize, steps: &'c [&'static str]) -> impl FnMut(&str) -> Result<Formatted, TsError> + 'c {
        move |text: &str| {
            let call = calls.get();

            calls.set(call + 1);

            if text.len() >= until {
                return Ok(Formatted { output: text.to_owned(), ..Formatted::default() });
            }

            Ok(Formatted { output: format!("{text}x"), applied: vec![steps[call % steps.len()]], complexity: Vec::new() })
        }
    }

    #[test]
    fn a_round_that_changes_nothing_is_not_repeated() {
        let calls = Cell::new(0);
        let formatted = fixed_point("done", growing(&calls, 0, &["a"])).expect("converges");

        assert_eq!(formatted.output, "done");
        assert_eq!(formatted.applied, Vec::<&str>::new());
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn rounds_repeat_until_one_changes_nothing_and_report_every_step_once_in_order() {
        let calls = Cell::new(0);
        let formatted = fixed_point("", growing(&calls, 3, &["b", "a", "b"])).expect("converges");

        assert_eq!(formatted.output, "xxx");
        assert_eq!(formatted.applied, ["b", "a"]);
        assert_eq!(calls.get(), 4);
    }

    #[test]
    fn a_text_that_keeps_changing_is_an_idempotency_error() {
        let calls = Cell::new(0);
        let error = fixed_point("", growing(&calls, usize::MAX, &["a"])).expect_err("never converges");

        assert!(matches!(error, TsError::Invariant { step: IDEMPOTENCY, .. }), "{error:?}");
        assert_eq!(calls.get(), EXTRA_ROUNDS + 1);
    }

    #[test]
    fn a_graphql_call_that_needs_a_second_round_is_formatted_in_one_call() {
        let source = "export const x = graphql(`{a}`)";
        let source_type = crate::syntax::source_type(Lang::Ts, "a.ts").expect("a script");
        let first = run_round("a.ts", source_type, source, &FULL).expect("the round succeeds");
        let fixed = lines(&["export const x = graphql(`", "\t{", "\t\ta", "\t}", "`);", ""]);

        assert_ne!(first.output, fixed, "oxfmt reaches the fixed point in one round now; this test no longer covers the loop");

        let formatted = format_source("a.ts", Lang::Ts, source, &TsFormat::default(), false).expect("succeeds");

        assert_eq!(formatted.output, fixed);
        assert_eq!(formatted.applied, ["oxfmt"]);
        assert!(assert_idempotent("a.ts", source));
    }

    #[test]
    fn scores_the_fixed_point() {
        let source = "export const x = graphql(`{a}`)\nfunction f(a) { if (a) return 1; return 2 }\n";
        let formatted = format_source("a.ts", Lang::Ts, source, &TsFormat::default(), true).expect("succeeds");
        let scored = crate::score("a.ts", Lang::Ts, &formatted.output).expect("parses");

        assert_eq!(formatted.complexity, scored);
        assert!(!scored.is_empty(), "{scored:?}");
    }

    /// Fuzz finding `not-idempotent-comment-in-comparison`: oxfmt moves the
    /// comment in the first round and joins the comparison in the second.
    #[test]
    fn a_line_comment_inside_a_comparison_settles() {
        let formatted = format_source("fuzz.ts", Lang::Ts, "a<a//\n<a>", &TsFormat::default(), false).expect("succeeds");

        assert_eq!(formatted.output, "a < a<a>; //\n");
        assert!(assert_idempotent("fuzz.ts", "a<a//\n<a>"));
    }
}

/// The minimized inputs of the `ts_pipeline` fuzz target, by finding name.
mod fuzz_findings {
    use super::*;

    fn format(rel: &str, source: &str) -> Result<Formatted, TsError> {
        format_source(rel, lang(rel), source, &TsFormat::default(), false)
    }

    #[test]
    fn blank_lines_lone_cr() {
        assert_eq!(format("fuzz.ts", "const a=`\n`\ra").expect("succeeds").output, "const a = `\n`;\n\na;\n");
        assert!(assert_idempotent("fuzz.ts", "const a=`\n`\ra"));
        assert!(assert_idempotent("fuzz.ts", "const a=`\n`; a"));
        assert!(assert_idempotent("fuzz.ts", "const a=`\n`\u{2028}a"));
    }

    #[test]
    fn oxc_debug_assert_lone_cr_in_template() {
        assert_eq!(format("fuzz.ts", "a`\r\\1`").expect("succeeds").output, "a`\n\\1`;\n");
        assert!(assert_idempotent("fuzz.ts", "a`\r\n\\1`"));
    }

    #[test]
    fn oxfmt_drops_logical_parens() {
        assert_eq!(format("fuzz.js", "{e&&(b&&c)}").expect("succeeds").output, "{\n\te && b && c;\n}\n");
        assert!(assert_idempotent("fuzz.ts", "function f() { return a && (b && c) || (d ?? (e ?? f)); }"));
    }

    #[test]
    fn not_idempotent_comment_in_comparison() {
        assert!(assert_idempotent("fuzz.ts", "a<a//\n<a>"));
    }

    /// oxfmt bugs that the invariant check refuses; `docs/known-issues.md`
    /// lists them. A failure here means oxfmt fixed one: update the document.
    #[test]
    fn upstream_oxfmt_bugs_stay_refused() {
        for (rel, source) in
            [("fuzz.ts", "a//\n<a//"), ("fuzz.ts", "a<a>a%a%a"), ("fuzz.tsx", "await 1e3.a"), ("fuzz.ts", "await//\nawait b"), ("fuzz.ts", "n<v>t({e:`\n`})")]
        {
            let result = format(rel, source);

            assert!(matches!(result, Err(TsError::Invariant { step: "oxfmt", .. })), "{rel} {source:?}: {result:?}");
        }
    }
}
