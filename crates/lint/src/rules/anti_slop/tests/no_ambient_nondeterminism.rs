//! `anti-slop/no-ambient-nondeterminism`: expectations recorded from the v1 plugin under oxlint 1.86.

use super::{Case, check};

/// tests/no-ambient-nondeterminism.test.ts
#[test]
fn v1() {
    check(
        "anti-slop/no-ambient-nondeterminism",
        "[]",
        &[
            Case {
                name: "instant supplied by the caller",
                path: "case.ts",
                code: "export function stamp(at: number): string {\n\treturn String(at);\n}\n",
                expected: &[],
            },
            Case {
                name: "date built from an argument",
                path: "case.ts",
                code: "export function stamp(at: number): Date {\n\treturn new Date(at);\n}\n",
                expected: &[],
            },
            Case {
                name: "shadowed Math",
                path: "case.ts",
                code: "const Math = { random: (): number => 0 };\n\nexport const pick = Math.random();\n",
                expected: &[],
            },
            Case {
                name: "Date.now",
                path: "case.ts",
                code: "export function stamp(): string {\n\treturn String(Date.now());\n}\n",
                expected: &[(49, 59, "`Date.now()` makes this output depend on when and where it ran. Take the value as a parameter so the caller owns it.")],
            },
            Case {
                name: "bare new Date",
                path: "case.ts",
                code: "export function stamp(): Date {\n\treturn new Date();\n}\n",
                expected: &[(40, 50, "`new Date()` without an argument reads the ambient clock. Take the instant as a parameter so the caller owns it.")],
            },
            Case {
                name: "Math.random",
                path: "case.ts",
                code: "export function pick(): number {\n\treturn Math.random();\n}\n",
                expected: &[(
                    41,
                    54,
                    "`Math.random()` makes this output depend on when and where it ran. Take the value as a parameter so the caller owns it.",
                )],
            },
            Case {
                name: "computed Math access",
                path: "case.ts",
                code: "export function pick(): number {\n\treturn Math[\"random\"]();\n}\n",
                expected: &[(
                    41,
                    57,
                    "`Math.random()` makes this output depend on when and where it ran. Take the value as a parameter so the caller owns it.",
                )],
            },
            Case {
                name: "performance.now",
                path: "case.ts",
                code: "export function elapsed(): number {\n\treturn performance.now();\n}\n",
                expected: &[(
                    44,
                    61,
                    "`performance.now()` makes this output depend on when and where it ran. Take the value as a parameter so the caller owns it.",
                )],
            },
            Case {
                name: "crypto.randomUUID",
                path: "case.ts",
                code: "export function id(): string {\n\treturn crypto.randomUUID();\n}\n",
                expected: &[(
                    39,
                    58,
                    "`crypto.randomUUID()` makes this output depend on when and where it ran. Take the value as a parameter so the caller owns it.",
                )],
            },
        ],
    );
}

#[test]
fn edges() {
    check(
        "anti-slop/no-ambient-nondeterminism",
        "[]",
        &[
            Case {
                name: "getRandomValues and hrtime",
                path: "case.ts",
                code: "crypto.getRandomValues(new Uint8Array(4));\nprocess.hrtime();\n",
                expected: &[
                    (
                        0,
                        41,
                        "`crypto.getRandomValues()` makes this output depend on when and where it ran. Take the value as a parameter so the caller owns it.",
                    ),
                    (43, 59, "`process.hrtime()` makes this output depend on when and where it ran. Take the value as a parameter so the caller owns it."),
                ],
            },
            Case {
                name: "new Date without parentheses",
                path: "case.ts",
                code: "export const at = new Date;\n",
                expected: &[(18, 26, "`new Date()` without an argument reads the ambient clock. Take the instant as a parameter so the caller owns it.")],
            },
            Case {
                name: "parenthesized owner and callee",
                path: "case.ts",
                code: "export const a = (Math).random();\nexport const b = (Date.now)();\nexport const c = new (Date)();\n",
                expected: &[
                    (17, 32, "`Math.random()` makes this output depend on when and where it ran. Take the value as a parameter so the caller owns it."),
                    (51, 63, "`Date.now()` makes this output depend on when and where it ran. Take the value as a parameter so the caller owns it."),
                    (82, 94, "`new Date()` without an argument reads the ambient clock. Take the instant as a parameter so the caller owns it."),
                ],
            },
            Case {
                name: "optional chains",
                path: "case.ts",
                code: "export const a = Math?.random();\nexport const b = Date.now?.();\n",
                expected: &[
                    (17, 31, "`Math.random()` makes this output depend on when and where it ran. Take the value as a parameter so the caller owns it."),
                    (50, 62, "`Date.now()` makes this output depend on when and where it ran. Take the value as a parameter so the caller owns it."),
                ],
            },
            Case { name: "computed template key is not matched", path: "case.ts", code: "export const a = Math[`random`]();\n", expected: &[] },
            Case {
                name: "shadowed by a parameter",
                path: "case.ts",
                code: "export function f(Date: { now(): number }): number {\n\treturn Date.now();\n}\n",
                expected: &[],
            },
            Case {
                name: "shadowed by a type only",
                path: "case.ts",
                code: "type Math = { random(): number };\n\nexport const a = Math.random();\n",
                expected: &[(
                    52,
                    65,
                    "`Math.random()` makes this output depend on when and where it ran. Take the value as a parameter so the caller owns it.",
                )],
            },
            Case {
                name: "shadowed inside a nested scope only",
                path: "case.ts",
                code: "function f() {\n\tconst Math = { random: () => 1 };\n\treturn Math.random();\n}\nexport const a = Math.random() + f();\n",
                expected: &[(
                    92,
                    105,
                    "`Math.random()` makes this output depend on when and where it ran. Take the value as a parameter so the caller owns it.",
                )],
            },
            Case {
                name: "unrelated members",
                path: "case.ts",
                code: "export const a = Math.max(1, 2) + Date.parse(\"x\") + performance.mark(\"x\");\n",
                expected: &[],
            },
            Case {
                name: "private member is not matched",
                path: "case.ts",
                code: "class A {\n\t#now = 1;\n\tread(): number {\n\t\treturn this.#now;\n\t}\n}\nexport { A };\n",
                expected: &[],
            },
            Case {
                name: "nested calls report each",
                path: "case.ts",
                code: "export const a = String(Math.random() + Date.now());\n",
                expected: &[
                    (24, 37, "`Math.random()` makes this output depend on when and where it ran. Take the value as a parameter so the caller owns it."),
                    (40, 50, "`Date.now()` makes this output depend on when and where it ran. Take the value as a parameter so the caller owns it."),
                ],
            },
            Case { name: "globalThis access is not matched", path: "case.ts", code: "export const a = globalThis.Math.random();\n", expected: &[] },
            Case {
                name: "new Date with arguments in tsx",
                path: "case.tsx",
                code: "export const a = <div>{String(new Date(1))}{Date.now()}</div>;\n",
                expected: &[(44, 54, "`Date.now()` makes this output depend on when and where it ran. Take the value as a parameter so the caller owns it.")],
            },
            Case {
                name: "shadowed by an import",
                path: "case.ts",
                code: "import { performance } from \"node:perf_hooks\";\n\nexport const a = performance.now();\n",
                expected: &[],
            },
        ],
    );
}

/// fmtkit's additions: readings that seed a mock are allowed, as in
/// t3code's `t3code/no-wall-clock-in-tests`, and `process.hrtime.bigint()` is a reading.
#[test]
fn additions() {
    check(
        "anti-slop/no-ambient-nondeterminism",
        "[]",
        &[
            Case { name: "seeds fake time", path: "case.test.ts", code: "import { vi } from \"vitest\";\n\nvi.setSystemTime(Date.now());\n", expected: &[] },
            Case { name: "inside a vi.fn implementation", path: "case.test.ts", code: "export const now = vi.fn(() => Date.now());\n", expected: &[] },
            Case { name: "returned by a mock", path: "case.test.ts", code: "clock.mockReturnValue(new Date());\n", expected: &[] },
            Case {
                name: "inside a vi.mock factory",
                path: "case.test.ts",
                code: "vi.mock(\"./clock\", () => ({ now: () => performance.now() }));\n",
                expected: &[],
            },
            Case {
                name: "shadowed vi is not the framework",
                path: "case.test.ts",
                code: "const vi = { fn: <T>(f: T): T => f };\n\nexport const now = vi.fn(() => Date.now());\n",
                expected: &[(70, 80, "`Date.now()` makes this output depend on when and where it ran. Take the value as a parameter so the caller owns it.")],
            },
            Case {
                name: "measured by the test",
                path: "case.test.ts",
                code: "it(\"is fast\", () => {\n\tconst start = Date.now();\n\n\trun();\n\texpect(Date.now() - start).toBeLessThan(5);\n});\n",
                expected: &[
                    (37, 47, "`Date.now()` makes this output depend on when and where it ran. Take the value as a parameter so the caller owns it."),
                    (66, 76, "`Date.now()` makes this output depend on when and where it ran. Take the value as a parameter so the caller owns it."),
                ],
            },
            Case {
                name: "process.hrtime.bigint",
                path: "case.ts",
                code: "export const at = process.hrtime.bigint();\n",
                expected: &[(
                    18,
                    41,
                    "`process.hrtime.bigint()` makes this output depend on when and where it ran. Take the value as a parameter so the caller owns it.",
                )],
            },
            Case {
                name: "shadowed process",
                path: "case.ts",
                code: "const process = { hrtime: { bigint: (): bigint => 0n } };\n\nexport const at = process.hrtime.bigint();\n",
                expected: &[],
            },
        ],
    );
}
