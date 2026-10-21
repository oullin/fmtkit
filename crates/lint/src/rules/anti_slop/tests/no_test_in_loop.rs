//! `anti-slop/no-test-in-loop`: adapted from t3code's `t3code/no-test-in-loop`.

use super::{Case, check};

/// The rule's behaviour, written for fmtkit rather than recorded from t3code's plugin.
#[test]
fn adapted() {
    check(
        "anti-slop/no-test-in-loop",
        "[]",
        &[
            Case {
                name: "it in a for-of loop",
                path: "case.test.ts",
                code: "for (const c of cases) {\n\tit(c.name, () => {});\n}\n",
                expected: &[(26, 28, "Do not call `it(…)` inside a for loop. Use `it.each(cases)(name, …)` instead.")],
            },
            Case {
                name: "describe in a for loop",
                path: "case.test.ts",
                code: "for (let i = 0; i < 3; i++) {\n\tdescribe(String(i), () => {});\n}\n",
                expected: &[(31, 39, "Do not call `describe(…)` inside a for loop. Use `describe.each(cases)(name, …)` instead.")],
            },
            Case {
                name: "test.concurrent in a for-in loop",
                path: "case.test.ts",
                code: "for (const key in table) {\n\ttest.concurrent(key, () => {});\n}\n",
                expected: &[(28, 43, "Do not call `test.concurrent(…)` inside a for loop. Use `test.concurrent.each(cases)(name, …)` instead.")],
            },
            Case { name: "it.each", path: "case.test.ts", code: "it.each(cases)(\"%s\", () => {});\n", expected: &[] },
            Case {
                name: "a loop inside a test",
                path: "case.test.ts",
                code: "it(\"sums\", () => {\n\tfor (const c of cases) {\n\t\texpect(c).toBe(1);\n\t}\n});\n",
                expected: &[],
            },
            Case {
                name: "a test in a callback inside a loop",
                path: "case.test.ts",
                code: "for (const c of cases) {\n\tgroups.forEach(() => it(c, () => {}));\n}\n",
                expected: &[],
            },
            Case { name: "a while loop", path: "case.test.ts", code: "while (next()) {\n\tit(\"x\", () => {});\n}\n", expected: &[] },
            Case { name: "not a test file", path: "case.ts", code: "for (const c of cases) {\n\tit(c.name, () => {});\n}\n", expected: &[] },
            Case {
                name: "node:test has no each",
                path: "case.test.ts",
                code: "import { it } from \"node:test\";\n\nfor (const c of cases) {\n\tit(c.name, () => {});\n}\n",
                expected: &[],
            },
            Case {
                name: "spec file",
                path: "case.spec.js",
                code: "for (const c of cases) {\n\ttest(c.name, () => {});\n}\n",
                expected: &[(26, 30, "Do not call `test(…)` inside a for loop. Use `test.each(cases)(name, …)` instead.")],
            },
        ],
    );
}
