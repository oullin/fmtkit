//! `anti-slop/no-unreasoned-test-skip`: adapted from the skip half of t3code's `t3code/no-test-timeout-or-unreasoned-skip`.

use super::{Case, check};

/// The rule's behaviour, written for fmtkit rather than recorded from t3code's plugin.
#[test]
fn adapted() {
    check(
        "anti-slop/no-unreasoned-test-skip",
        "[]",
        &[
            Case {
                name: "it.skip",
                path: "case.test.ts",
                code: "it.skip(\"x\", () => {});\n",
                expected: &[(0, 7, "Say why this test is skipped: link the issue that tracks it (`#123` or a URL) in a comment above it.")],
            },
            Case {
                name: "describe.todo",
                path: "case.test.ts",
                code: "describe.todo(\"later\");\n",
                expected: &[(0, 13, "Say why this test is skipped: link the issue that tracks it (`#123` or a URL) in a comment above it.")],
            },
            Case {
                name: "xit",
                path: "case.test.ts",
                code: "xit(\"x\", () => {});\n",
                expected: &[(0, 3, "Say why this test is skipped: link the issue that tracks it (`#123` or a URL) in a comment above it.")],
            },
            Case {
                name: "skip option",
                path: "case.test.ts",
                code: "test(\"x\", { skip: true }, () => {});\n",
                expected: &[(0, 35, "Say why this test is skipped: link the issue that tracks it (`#123` or a URL) in a comment above it.")],
            },
            Case {
                name: "computed key",
                path: "case.test.ts",
                code: "it[\"todo\"](\"x\");\n",
                expected: &[(0, 10, "Say why this test is skipped: link the issue that tracks it (`#123` or a URL) in a comment above it.")],
            },
            Case {
                name: "skip then each",
                path: "case.test.ts",
                code: "it.skip.each(cases)(\"%s\", () => {});\n",
                expected: &[(0, 7, "Say why this test is skipped: link the issue that tracks it (`#123` or a URL) in a comment above it.")],
            },
            Case {
                name: "modifier chain",
                path: "case.test.ts",
                code: "it.concurrent.skip(\"x\", () => {});\n",
                expected: &[(0, 18, "Say why this test is skipped: link the issue that tracks it (`#123` or a URL) in a comment above it.")],
            },
            Case {
                name: "comment without a link",
                path: "case.test.ts",
                code: "// Flaky on CI.\nit.skip(\"x\", () => {});\n",
                expected: &[(16, 23, "Say why this test is skipped: link the issue that tracks it (`#123` or a URL) in a comment above it.")],
            },
            Case { name: "issue number above", path: "case.test.ts", code: "// Flaky on CI until #123 lands.\nit.skip(\"x\", () => {});\n", expected: &[] },
            Case { name: "issue link above", path: "case.test.ts", code: "// https://github.com/acme/app/issues/9\ntest.todo(\"x\");\n", expected: &[] },
            Case {
                name: "linked skip inside a suite",
                path: "case.test.ts",
                code: "describe(\"suite\", () => {\n\t// Blocked on #41.\n\tit.skip(\"x\", () => {});\n});\n",
                expected: &[],
            },
            Case { name: "computed skip option", path: "case.test.ts", code: "test(\"x\", { skip: isWindows }, () => {});\n", expected: &[] },
            Case { name: "skipIf is a gate", path: "case.test.ts", code: "describe.skipIf(isWindows)(\"x\", () => {});\n", expected: &[] },
            Case { name: "skip off a non-test object", path: "case.test.ts", code: "list.skip(1);\n", expected: &[] },
            Case { name: "not a test file", path: "case.ts", code: "it.skip(\"x\", () => {});\n", expected: &[] },
        ],
    );
}
