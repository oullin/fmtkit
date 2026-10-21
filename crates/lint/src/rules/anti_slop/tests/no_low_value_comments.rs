//! `anti-slop/no-low-value-comments`: adapted from t3code's `t3code/no-low-value-comments`.

use super::{Case, check};

/// The rule's behaviour, written for fmtkit rather than recorded from t3code's plugin.
#[test]
fn adapted() {
    check(
        "anti-slop/no-low-value-comments",
        "[]",
        &[
            Case {
                name: "section divider",
                path: "case.ts",
                code: "// ---------- helpers ----------\nexport const a = 1;\n",
                expected: &[(0, 32, "Drop the section divider; split the file or let the code's structure show the grouping.")],
            },
            Case {
                name: "block divider",
                path: "case.ts",
                code: "/* ===================== */\nexport const a = 1;\n",
                expected: &[(0, 27, "Drop the section divider; split the file or let the code's structure show the grouping.")],
            },
            Case {
                name: "test step label",
                path: "case.test.ts",
                code: "it(\"x\", () => {\n\t// Arrange\n\tconst a = 1;\n\n\texpect(a).toBe(1);\n});\n",
                expected: &[(17, 27, "Drop the test step label.")],
            },
            Case {
                name: "unlinked TODO",
                path: "case.ts",
                code: "// TODO: handle errors\nexport const a = 1;\n",
                expected: &[(0, 22, "Link the issue that tracks this, or drop the marker.")],
            },
            Case { name: "linked TODO", path: "case.ts", code: "// TODO(#12): handle errors\nexport const a = 1;\n", expected: &[] },
            Case {
                name: "JSDoc type in TypeScript",
                path: "case.ts",
                code: "/**\n * Greets someone by name.\n * @param {string} name Who to greet.\n */\nexport function greet(name: string): string {\n\treturn name;\n}\n",
                expected: &[(0, 72, "Drop the JSDoc type; TypeScript already has it.")],
            },
            Case {
                name: "JSDoc type in JavaScript",
                path: "case.js",
                code: "/**\n * Greets someone by name.\n * @param {string} name Who to greet.\n */\nexport function greet(name) {\n\treturn name;\n}\n",
                expected: &[],
            },
            Case {
                name: "empty JSDoc tag",
                path: "case.ts",
                code: "/**\n * Greets someone.\n * @returns\n */\nexport function greet(): string {\n\treturn \"hi\";\n}\n",
                expected: &[(0, 38, "Drop the JSDoc tag that has no description.")],
            },
            Case {
                name: "commented-out code",
                path: "case.ts",
                code: "// const a = compute();\n// a.run();\nexport const b = 1;\n",
                expected: &[(0, 35, "Delete commented-out code; git keeps the history.")],
            },
            Case {
                name: "prose that mentions code",
                path: "case.ts",
                code: "// compute() runs once per file, so cache it.\nexport const b = 1;\n",
                expected: &[],
            },
            Case {
                name: "restates the name",
                path: "case.ts",
                code: "// Get the user name\nexport function getUserName(): string {\n\treturn \"\";\n}\n",
                expected: &[(0, 20, "Drop the comment; it only repeats the name below it. Say why, or say nothing.")],
            },
            Case {
                name: "restates a JSDoc name",
                path: "case.ts",
                code: "/** The user names. */\nexport const userNames: Array<string> = [];\n",
                expected: &[(0, 22, "Drop the comment; it only repeats the name below it. Say why, or say nothing.")],
            },
            Case {
                name: "says why",
                path: "case.ts",
                code: "// Cached: the lookup hits the network.\nexport function getUserName(): string {\n\treturn \"\";\n}\n",
                expected: &[],
            },
            Case { name: "trailing comment", path: "case.ts", code: "export const a = 1; // the user name\nexport const userName = \"\";\n", expected: &[] },
            Case { name: "statement keyword", path: "case.ts", code: "// If\nif (a) {\n\tb();\n}\n", expected: &[] },
            Case {
                name: "tool directives",
                path: "case.ts",
                code: "// prettier-ignore\nexport const a = [1,2];\n/*! (c) Acme --------- */\nexport const b = /* @__PURE__ */ make();\n",
                expected: &[],
            },
            Case {
                name: "separate indents are separate comments",
                path: "case.ts",
                code: "// Rebuilds on change because the cache key is the path.\n\t// a.run();\nexport const b = 1;\n",
                expected: &[(58, 69, "Delete commented-out code; git keeps the history.")],
            },
        ],
    );
}
