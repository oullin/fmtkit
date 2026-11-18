//! `anti-slop/max-doc-comment-lines`: fmtkit's own rule, so its tables are fmtkit's own.

use super::{Case, check};

/// The default options: `max` 2, test files checked.
#[test]
fn defaults() {
    check(
        "anti-slop/max-doc-comment-lines",
        "[]",
        &[
            Case {
                name: "the motivating example: the class doc's length and the constructor's doc",
                path: "case.ts",
                code: "/**\n * The invocation's `ExecutionContext`, with a `waitUntil` whose rejection is observed: work handed to it, such as a failure\n * raised behind the response, reaches a failure boundary under the invocation's correlation, which writes its one line.\n * Every other member is the runtime's own.\n */\nexport class ObservedExecutionContext implements ExecutionContext {\n\t/** Builds the wrapper around the runtime's context. */\n\tprivate constructor(/* ... */) {}\n\n\t/** Called inside the invocation's correlation scope, whose UUID the background boundary keeps. */\n\tstatic of(context: ExecutionContext): ExecutionContext {\n\t\treturn context;\n\t}\n}\n",
                expected: &[
                    (
                        0,
                        297,
                        "This doc comment runs 3 lines. Keep a doc comment to what the code can't say, in at most 2 lines; the code, its tests and the commit message carry the rest.",
                    ),
                    (367, 422, "Drop the doc comment on this constructor. Say what callers need on the class, or on the factory they call."),
                ],
            },
            Case {
                name: "two content lines between lone delimiters",
                path: "case.ts",
                code: "/**\n * Retries once: the first call warms the pool.\n * The second is the one that counts.\n */\nexport function load(): void {}\n",
                expected: &[],
            },
            Case {
                name: "delimiters that hold text count",
                path: "case.ts",
                code: "/** Retries once.\n * The first call warms the pool.\n * The second counts. */\nexport function load(): void {}\n",
                expected: &[(
                    0,
                    76,
                    "This doc comment runs 3 lines. Keep a doc comment to what the code can't say, in at most 2 lines; the code, its tests and the commit message carry the rest.",
                )],
            },
            Case {
                name: "blank gutter lines do not count",
                path: "case.ts",
                code: "/**\n * Retries once.\n *\n * The second call counts.\n */\nexport function load(): void {}\n",
                expected: &[],
            },
            Case {
                name: "a run of line comments above a statement",
                path: "case.ts",
                code: "function run(): void {\n\t// The pool is cold on the first call,\n\t// so the first result is thrown away\n\t// and the call is made again.\n\twarm();\n}\n",
                expected: &[(
                    24,
                    133,
                    "This doc comment runs 3 lines. Keep a doc comment to what the code can't say, in at most 2 lines; the code, its tests and the commit message carry the rest.",
                )],
            },
            Case {
                name: "two line comments pass",
                path: "case.ts",
                code: "// The pool is cold on the first call,\n// so the first result is thrown away.\nwarm();\n",
                expected: &[],
            },
            Case {
                name: "a blank line before the code ends the doc",
                path: "case.ts",
                code: "// One.\n// Two.\n// Three.\n\nexport const a = 1;\n",
                expected: &[],
            },
            Case {
                name: "a blank line inside the run splits it",
                path: "case.ts",
                code: "// One.\n// Two.\n\n// Three.\nexport const a = 1;\n",
                expected: &[],
            },
            Case {
                name: "a trailing comment is not a doc",
                path: "case.ts",
                code: "export const a = 1; // One.\n// Two.\n// Three.\nexport const b = 2;\n",
                expected: &[],
            },
            Case {
                name: "an object property",
                path: "case.ts",
                code: "export const options = {\n\t// One.\n\t// Two.\n\t// Three.\n\tretries: 1,\n};\n",
                expected: &[(
                    26,
                    53,
                    "This doc comment runs 3 lines. Keep a doc comment to what the code can't say, in at most 2 lines; the code, its tests and the commit message carry the rest.",
                )],
            },
            Case {
                name: "an interface member",
                path: "case.ts",
                code: "export interface Options {\n\t/**\n\t * One.\n\t * Two.\n\t * Three.\n\t */\n\tretries: number;\n}\n",
                expected: &[(
                    28,
                    65,
                    "This doc comment runs 3 lines. Keep a doc comment to what the code can't say, in at most 2 lines; the code, its tests and the commit message carry the rest.",
                )],
            },
            Case {
                name: "an enum member",
                path: "case.ts",
                code: "export enum Mode {\n\t/** One.\n\t * Two.\n\t * Three. */\n\tFast,\n}\n",
                expected: &[(
                    20,
                    51,
                    "This doc comment runs 3 lines. Keep a doc comment to what the code can't say, in at most 2 lines; the code, its tests and the commit message carry the rest.",
                )],
            },
            Case {
                name: "a public class member",
                path: "case.ts",
                code: "export class Pool {\n\t/**\n\t * One.\n\t * Two.\n\t * Three.\n\t */\n\tsize = 1;\n\n\t/** Short. */\n\tdrain(): void {}\n}\n",
                expected: &[(
                    21,
                    58,
                    "This doc comment runs 3 lines. Keep a doc comment to what the code can't say, in at most 2 lines; the code, its tests and the commit message carry the rest.",
                )],
            },
            Case {
                name: "a private member, any length",
                path: "case.ts",
                code: "export class Pool {\n\t/** Short. */\n\tprivate size = 1;\n}\n",
                expected: &[(21, 34, "Drop the doc comment on this private member. Only its class calls it, so let its name and body say what it does.")],
            },
            Case {
                name: "a private method",
                path: "case.ts",
                code: "export class Pool {\n\t// Short.\n\tprivate drain(): void {}\n}\n",
                expected: &[(21, 30, "Drop the doc comment on this private member. Only its class calls it, so let its name and body say what it does.")],
            },
            Case {
                name: "a #private member",
                path: "case.ts",
                code: "export class Pool {\n\t/** Short. */\n\t#size = 1;\n\n\t/** Short too. */\n\t#drain(): void {}\n}\n",
                expected: &[
                    (21, 34, "Drop the doc comment on this private member. Only its class calls it, so let its name and body say what it does."),
                    (49, 66, "Drop the doc comment on this private member. Only its class calls it, so let its name and body say what it does."),
                ],
            },
            Case {
                name: "a #private member in JavaScript",
                path: "case.js",
                code: "export class Pool {\n\t/** Short. */\n\t#size = 1;\n}\n",
                expected: &[(21, 34, "Drop the doc comment on this private member. Only its class calls it, so let its name and body say what it does.")],
            },
            Case {
                name: "a private parameter property",
                path: "case.ts",
                code: "export class Pool {\n\tconstructor(\n\t\t/** Short. */\n\t\tprivate readonly size: number,\n\t\t/** Public. */\n\t\treadonly name: string,\n\t) {}\n}\n",
                expected: &[(36, 49, "Drop the doc comment on this private member. Only its class calls it, so let its name and body say what it does.")],
            },
            Case {
                name: "a decorated private member",
                path: "case.ts",
                code: "export class Pool {\n\t/** Short. */\n\t@observed()\n\tprivate size = 1;\n}\n",
                expected: &[(21, 34, "Drop the doc comment on this private member. Only its class calls it, so let its name and body say what it does.")],
            },
            Case {
                name: "a public constructor",
                path: "case.ts",
                code: "export class Pool {\n\t/** Short. */\n\tconstructor() {}\n}\n",
                expected: &[(21, 34, "Drop the doc comment on this constructor. Say what callers need on the class, or on the factory they call.")],
            },
            Case {
                name: "protected and public members pass",
                path: "case.ts",
                code: "export class Pool {\n\t/** Short. */\n\tprotected size = 1;\n\n\t/** Short. */\n\tpublic drain(): void {}\n}\n",
                expected: &[],
            },
            Case { name: "an undocumented private member", path: "case.ts", code: "export class Pool {\n\tprivate size = 1; // trailing\n}\n", expected: &[] },
            Case {
                name: "directives are not counted",
                path: "case.ts",
                code: "// One.\n// Two.\n// oxlint-disable-next-line no-console -- the CLI writes to the terminal\nconsole.log(1);\n/** One.\n * Two. */\n// eslint-disable-next-line no-console -- the CLI writes to the terminal\nconsole.log(2);\n",
                expected: &[],
            },
            Case {
                name: "a directive does not hide the doc above it",
                path: "case.ts",
                code: "// One.\n// Two.\n// Three.\n// oxlint-disable-next-line no-console -- the CLI writes to the terminal\nconsole.log(1);\n",
                expected: &[(
                    0,
                    25,
                    "This doc comment runs 3 lines. Keep a doc comment to what the code can't say, in at most 2 lines; the code, its tests and the commit message carry the rest.",
                )],
            },
            Case {
                name: "tool comments alone pass, even on private members",
                path: "case.ts",
                code: "/// <reference types=\"node\" />\n// @ts-expect-error the module has no types\nimport legacy from \"legacy\";\nexport class Pool {\n\t// prettier-ignore\n\t// oxfmt-ignore\n\t// fallow-ignore-next-line unused-class-member\n\t/* istanbul ignore next */\n\t/* c8 ignore next */\n\tprivate size = legacy;\n}\n",
                expected: &[],
            },
            Case {
                name: "a SAFETY comment is required, whatever its length",
                path: "case.ts",
                code: "// SAFETY: every key in `keys` was set above,\n// so the lookup cannot miss\n// and the value is a `Pool`.\nconst pool = map.get(key) as Pool;\n",
                expected: &[],
            },
            Case {
                name: "a SAFETY block comment",
                path: "case.ts",
                code: "export class Pool {\n\t/*\n\t * SAFETY: the runtime sets this before any call,\n\t * and nothing clears it.\n\t * It is a `Context`.\n\t */\n\tprivate readonly context = global.context as Context;\n}\n",
                expected: &[],
            },
            Case {
                name: "an observability escape is not counted",
                path: "case.ts",
                code: "export function close(): void {\n\ttry {\n\t\trelease();\n\t\t// The handle may already be gone.\n\t\t// observability: swallowed because the resource is already closed\n\t} catch {}\n}\n",
                expected: &[],
            },
            Case {
                name: "an observability escape above two doc lines",
                path: "case.ts",
                code: "// One.\n// Two.\n// observability: swallowed because the request is best effort\nsend().catch(() => {});\n",
                expected: &[],
            },
            Case {
                name: "a license header at the top of the file",
                path: "case.ts",
                code: "/**\n * Copyright (c) Acme Inc.\n *\n * This source code is licensed under the MIT license found in the\n * LICENSE file in the root directory of this source tree.\n */\nexport const a = 1;\n",
                expected: &[],
            },
            Case {
                name: "a license header after a hashbang",
                path: "case.ts",
                code: "#!/usr/bin/env node\n// SPDX-License-Identifier: MIT\n// One.\n// Two.\nrun();\n",
                expected: &[],
            },
            Case { name: "a preserved comment", path: "case.ts", code: "/*!\n * Acme v1\n * One.\n * Two.\n */\nexport const a = 1;\n", expected: &[] },
            Case {
                name: "license words further down are a doc",
                path: "case.ts",
                code: "export const a = 1;\n/**\n * Checks the license.\n * One.\n * Two.\n */\nexport function check(): void {}\n",
                expected: &[(
                    20,
                    66,
                    "This doc comment runs 3 lines. Keep a doc comment to what the code can't say, in at most 2 lines; the code, its tests and the commit message carry the rest.",
                )],
            },
            Case {
                name: "test files are checked by default",
                path: "case.test.ts",
                code: "it(\"works\", () => {\n\t// One.\n\t// Two.\n\t// Three.\n\texpect(1).toBe(1);\n});\n",
                expected: &[(
                    21,
                    48,
                    "This doc comment runs 3 lines. Keep a doc comment to what the code can't say, in at most 2 lines; the code, its tests and the commit message carry the rest.",
                )],
            },
        ],
    );
}

/// `max` 1.
#[test]
fn max_one() {
    check(
        "anti-slop/max-doc-comment-lines",
        "[{\"max\":1}]",
        &[Case {
            name: "max 1",
            path: "case.ts",
            code: "// One.\n// Two.\nexport const a = 1;\n/** One. */\nexport const b = 1;\n",
            expected: &[(
                0,
                15,
                "This doc comment runs 2 lines. Keep a doc comment to what the code can't say, in at most 1 line; the code, its tests and the commit message carry the rest.",
            )],
        }],
    );
}

/// `max` 4: the motivating example's class doc passes, its constructor's doc does not.
#[test]
fn max_four() {
    check(
        "anti-slop/max-doc-comment-lines",
        "[{\"max\":4}]",
        &[Case {
            name: "max 4",
            path: "case.ts",
            code: "/**\n * The invocation's `ExecutionContext`, with a `waitUntil` whose rejection is observed: work handed to it, such as a failure\n * raised behind the response, reaches a failure boundary under the invocation's correlation, which writes its one line.\n * Every other member is the runtime's own.\n */\nexport class ObservedExecutionContext implements ExecutionContext {\n\t/** Builds the wrapper around the runtime's context. */\n\tprivate constructor(/* ... */) {}\n\n\t/** Called inside the invocation's correlation scope, whose UUID the background boundary keeps. */\n\tstatic of(context: ExecutionContext): ExecutionContext {\n\t\treturn context;\n\t}\n}\n",
            expected: &[(367, 422, "Drop the doc comment on this constructor. Say what callers need on the class, or on the factory they call.")],
        }],
    );
}

/// `ignoreTests`: test files and files in test directories are skipped.
#[test]
fn ignore_tests() {
    check(
        "anti-slop/max-doc-comment-lines",
        "[{\"ignoreTests\":true}]",
        &[
            Case {
                name: "a test file",
                path: "case.test.ts",
                code: "it(\"works\", () => {\n\t// One.\n\t// Two.\n\t// Three.\n\texpect(1).toBe(1);\n});\n",
                expected: &[],
            },
            Case { name: "a file in a tests directory", path: "tests/fixtures.ts", code: "// One.\n// Two.\n// Three.\nexport const a = 1;\n", expected: &[] },
            Case {
                name: "a source file",
                path: "case.ts",
                code: "// One.\n// Two.\n// Three.\nexport const a = 1;\n",
                expected: &[(
                    0,
                    25,
                    "This doc comment runs 3 lines. Keep a doc comment to what the code can't say, in at most 2 lines; the code, its tests and the commit message carry the rest.",
                )],
            },
        ],
    );
}

/// `ignorePatterns`: a matching comment, such as a perfectionist partition comment, is not counted.
#[test]
fn ignore_patterns() {
    check(
        "anti-slop/max-doc-comment-lines",
        "[{\"ignorePatterns\":[\"^\\\\s*Part:\"]}]",
        &[Case {
            name: "a matching comment is not counted",
            path: "case.ts",
            code: "export class Pool {\n\t// Part: internals\n\tprivate size = 1;\n\n\t// Not a partition.\n\tprivate name = \"\";\n}\n",
            expected: &[(61, 80, "Drop the doc comment on this private member. Only its class calls it, so let its name and body say what it does.")],
        }],
    );
}

/// Options outside the schema are rejected.
#[test]
fn rejects_invalid_options() {
    let factory = super::super::RULES.iter().find(|(name, _)| *name == "anti-slop/max-doc-comment-lines").map(|(_, factory)| *factory).expect("registered");

    for options in [
        r#"[{"max":0}]"#,
        r#"[{"max":1.5}]"#,
        r#"[{"max":"2"}]"#,
        r#"[{"ignoreTests":1}]"#,
        r#"[{"ignorePatterns":["("]}]"#,
        r#"[{"ignorePatterns":"x"}]"#,
        r#"[{"lines":2}]"#,
        "[2]",
        "[{}, {}]",
    ] {
        let options: Vec<serde_json::Value> = serde_json::from_str(options).expect("JSON");

        assert!(factory(&options).is_err(), "accepts {options:?}");
    }

    assert!(factory(&[serde_json::json!({ "max": 3.0, "ignoreTests": true, "ignorePatterns": ["^Part:"] })]).is_ok(), "rejects valid options");
}
