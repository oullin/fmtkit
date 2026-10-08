//! `anti-slop/require-safety-comment-for-type-assertion`: expectations recorded from the v1 plugin under oxlint 1.86.

use super::{Case, check};

/// tests/require-safety-comment-for-type-assertion.test.ts
#[test]
fn v1() {
    check(
        "anti-slop/require-safety-comment-for-type-assertion",
        "[]",
        &[
            Case { name: "no assertion", path: "case.ts", code: "export const total: number = 1;\n", expected: &[] },
            Case {
                name: "const assertion needs no justification",
                path: "case.ts",
                code: "export const modes = [\"read\", \"write\"] as const;\n",
                expected: &[],
            },
            Case {
                name: "justified declaration",
                path: "case.ts",
                code: "declare const raw: string | number;\n\n// SAFETY: the caller has already parsed this as text.\nconst text = raw as string;\n\nexport { text };\n",
                expected: &[],
            },
            Case {
                name: "justified return",
                path: "case.ts",
                code: "declare const raw: string | number;\n\nexport function read(): string {\n\t// SAFETY: the caller has already parsed this as text.\n\treturn raw as string;\n}\n",
                expected: &[],
            },
            Case {
                name: "unjustified assertion",
                path: "case.ts",
                code: "declare const raw: string | number;\n\nexport const text = raw as string;\n",
                expected: &[(
                    57,
                    70,
                    "This type assertion has no `SAFETY:` justification. State the checked invariant immediately before the assertion or its containing statement.",
                )],
            },
        ],
    );
}

/// tests/require-safety-comment-for-type-assertion.test.ts: a comment above `export const` is not seen.
#[test]
fn v1_export_boundary() {
    check(
        "anti-slop/require-safety-comment-for-type-assertion",
        "[]",
        &[Case {
            name: "justified export",
            path: "case.ts",
            code: "declare const raw: string | number;\n\n// SAFETY: the caller has already parsed this as text.\nexport const text = raw as string;\n",
            expected: &[(
                112,
                125,
                "This type assertion has no `SAFETY:` justification. State the checked invariant immediately before the assertion or its containing statement.",
            )],
        }],
    );
}

#[test]
fn edges() {
    check(
        "anti-slop/require-safety-comment-for-type-assertion",
        "[]",
        &[
            Case {
                name: "inline block comment",
                path: "case.ts",
                code: "declare const raw: unknown;\nexport const a = /* SAFETY: checked */ raw as string;\nexport const b = /* SAFETY : spaced */ <string>raw;\n",
                expected: &[],
            },
            Case {
                name: "comment forms that do not count",
                path: "case.ts",
                code: "declare const raw: unknown;\n// NOTSAFETY: x\nconst a = raw as string;\n// safety: lower\nconst b = raw as string;\n// SAFETY x\nconst c = raw as string;\n/* SAFETY: */ export const d = raw as string;\nexport { a, b, c };\n",
                expected: &[
                    (
                        54,
                        67,
                        "This type assertion has no `SAFETY:` justification. State the checked invariant immediately before the assertion or its containing statement.",
                    ),
                    (
                        96,
                        109,
                        "This type assertion has no `SAFETY:` justification. State the checked invariant immediately before the assertion or its containing statement.",
                    ),
                    (
                        133,
                        146,
                        "This type assertion has no `SAFETY:` justification. State the checked invariant immediately before the assertion or its containing statement.",
                    ),
                    (
                        179,
                        192,
                        "This type assertion has no `SAFETY:` justification. State the checked invariant immediately before the assertion or its containing statement.",
                    ),
                ],
            },
            Case {
                name: "comment must be directly before",
                path: "case.ts",
                code: "declare const raw: unknown;\n// SAFETY: far\nconst x = 1;\nconst a = raw as string;\nexport { a, x };\n",
                expected: &[(
                    66,
                    79,
                    "This type assertion has no `SAFETY:` justification. State the checked invariant immediately before the assertion or its containing statement.",
                )],
            },
            Case {
                name: "stacked comments",
                path: "case.ts",
                code: "declare const raw: unknown;\n// SAFETY: first\n// another\n/* more */\nconst a = raw as string;\nexport { a };\n",
                expected: &[],
            },
            Case {
                name: "owners",
                path: "case.ts",
                code: "declare const raw: unknown;\ndeclare function use(v: string): void;\n// SAFETY: statement\nuse(raw as string);\nexport class C {\n\t// SAFETY: property\n\tp = raw as string;\n\tq = raw as string;\n}\nexport function f(): void {\n\t// SAFETY: throw\n\tthrow raw as Error;\n}\n",
                expected: &[(
                    171,
                    184,
                    "This type assertion has no `SAFETY:` justification. State the checked invariant immediately before the assertion or its containing statement.",
                )],
            },
            Case {
                name: "stops at the owner",
                path: "case.ts",
                code: "declare const raw: unknown;\n// SAFETY: outer\nexport function f(): string {\n\treturn raw as string;\n}\n",
                expected: &[(
                    83,
                    96,
                    "This type assertion has no `SAFETY:` justification. State the checked invariant immediately before the assertion or its containing statement.",
                )],
            },
            Case {
                name: "comment inside the expression",
                path: "case.ts",
                code: "declare const raw: unknown;\ndeclare function use(a: number, b: string): void;\nuse(\n\t1,\n\t// SAFETY: arg\n\traw as string,\n);\n",
                expected: &[],
            },
            Case {
                name: "comment after the assertion start does not count",
                path: "case.ts",
                code: "declare const raw: unknown;\nexport const a = raw as /* SAFETY: late */ string;\n",
                expected: &[(
                    45,
                    77,
                    "This type assertion has no `SAFETY:` justification. State the checked invariant immediately before the assertion or its containing statement.",
                )],
            },
            Case {
                name: "parenthesized assertion",
                path: "case.ts",
                code: "declare const raw: unknown;\n// SAFETY: wrapped\nconst a = (raw as string);\nconst b = /* SAFETY: inner */ (raw as string);\nexport { a, b };\n",
                expected: &[(
                    105,
                    118,
                    "This type assertion has no `SAFETY:` justification. State the checked invariant immediately before the assertion or its containing statement.",
                )],
            },
            Case {
                name: "nested assertions",
                path: "case.ts",
                code: "declare const raw: unknown;\n// SAFETY: both\nconst a = (raw as unknown) as string;\nexport { a };\n",
                expected: &[],
            },
            Case {
                name: "arrow body",
                path: "case.ts",
                code: "declare const raw: unknown;\nexport const f = (): string =>\n\t// SAFETY: arrow\n\traw as string;\n",
                expected: &[],
            },
        ],
    );
}
