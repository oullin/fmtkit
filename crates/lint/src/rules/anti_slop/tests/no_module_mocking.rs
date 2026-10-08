//! `anti-slop/no-module-mocking`: expectations recorded from the v1 plugin under oxlint 1.86.

use super::{Case, check};

/// tests/no-module-mocking.test.ts
#[test]
fn v1() {
    check(
        "anti-slop/no-module-mocking",
        "[]",
        &[
            Case {
                name: "injected fake",
                path: "case.ts",
                code: "export function build(clock: { now: () => number }): number {\n\treturn clock.now();\n}\n",
                expected: &[],
            },
            Case {
                name: "vitest module mock",
                path: "case.ts",
                code: "import { vi } from \"vitest\";\n\nvi.mock(\"#app/clock\");\n",
                expected: &[(30, 51, "Replace module mocking with dependency injection through a real interface, service layer, or faithful test implementation.")],
            },
            Case {
                name: "jest module mock",
                path: "case.ts",
                code: "import { jest } from \"@jest/globals\";\n\njest.mock(\"#app/clock\");\n",
                expected: &[(39, 62, "Replace module mocking with dependency injection through a real interface, service layer, or faithful test implementation.")],
            },
        ],
    );
}

#[test]
fn edges() {
    check(
        "anti-slop/no-module-mocking",
        "[]",
        &[
            Case {
                name: "global vi and jest",
                path: "case.ts",
                code: "vi.mock(\"a\");\njest.doMock(\"b\");\nvi.unstable_mockModule(\"c\");\njest.fn();\nvi.spyOn(console, \"log\");\n",
                expected: &[(0, 12, "Replace module mocking with dependency injection through a real interface, service layer, or faithful test implementation."), (14, 30, "Replace module mocking with dependency injection through a real interface, service layer, or faithful test implementation."), (32, 59, "Replace module mocking with dependency injection through a real interface, service layer, or faithful test implementation.")],
            },
            Case {
                name: "computed methods",
                path: "case.ts",
                code: "vi[\"mock\"](\"a\");\njest[\"doMock\"](\"b\");\nvi[\"fn\"]();\nvi[`mock`](\"c\");\n",
                expected: &[(0, 15, "Replace module mocking with dependency injection through a real interface, service layer, or faithful test implementation."), (17, 36, "Replace module mocking with dependency injection through a real interface, service layer, or faithful test implementation.")],
            },
            Case {
                name: "renamed imports",
                path: "case.ts",
                code: "import { vi as v } from \"vitest\";\nimport { jest as j } from \"@jest/globals\";\nimport { \"vi\" as w } from \"vitest\";\n\nv.mock(\"a\");\nj.mock(\"b\");\nw.mock(\"c\");\n",
                expected: &[(114, 125, "Replace module mocking with dependency injection through a real interface, service layer, or faithful test implementation."), (127, 138, "Replace module mocking with dependency injection through a real interface, service layer, or faithful test implementation."), (140, 151, "Replace module mocking with dependency injection through a real interface, service layer, or faithful test implementation.")],
            },
            Case {
                name: "imports from elsewhere",
                path: "case.ts",
                code: "import { vi } from \"./fake\";\nimport jest from \"@jest/globals\";\nimport * as v from \"vitest\";\n\nvi.mock(\"a\");\njest.mock(\"b\");\nv.vi.mock(\"c\");\n",
                expected: &[],
            },
            Case {
                name: "wrong imported name",
                path: "case.ts",
                code: "import { jest as vi } from \"vitest\";\n\nvi.mock(\"a\");\n",
                expected: &[],
            },
            Case {
                name: "local binding",
                path: "case.ts",
                code: "const vi = { mock: (path: string): string => path };\n\nvi.mock(\"a\");\n",
                expected: &[],
            },
            Case {
                name: "other globals",
                path: "case.ts",
                code: "mocker.mock(\"a\");\n",
                expected: &[],
            },
            Case {
                name: "parenthesized object",
                path: "case.ts",
                code: "(vi).mock(\"a\");\n(vi.mock)(\"b\");\nvi?.mock(\"c\");\n",
                expected: &[(0, 14, "Replace module mocking with dependency injection through a real interface, service layer, or faithful test implementation."), (16, 30, "Replace module mocking with dependency injection through a real interface, service layer, or faithful test implementation."), (32, 45, "Replace module mocking with dependency injection through a real interface, service layer, or faithful test implementation.")],
            },
            Case {
                name: "shadowed by a type only",
                path: "case.ts",
                code: "type vi = number;\n\nvi.mock(\"a\");\n",
                expected: &[],
            },
        ],
    );
}
