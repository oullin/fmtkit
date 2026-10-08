//! `anti-slop/no-runtime-typeof`: expectations recorded from the v1 plugin under oxlint 1.86.

use super::{Case, check};

/// tests/no-runtime-typeof.test.ts
#[test]
fn v1() {
    check(
        "anti-slop/no-runtime-typeof",
        "[]",
        &[
            Case {
                name: "domain branch",
                path: "case.ts",
                code: "export function read(value: { kind: \"text\"; body: string }): string {\n\treturn value.body;\n}\n",
                expected: &[],
            },
            Case {
                name: "runtime narrowing",
                path: "case.ts",
                code: "export function read(value: string | number): string {\n\tif (typeof value === \"string\") {\n\t\treturn value;\n\t}\n\n\treturn String(value);\n}\n",
                expected: &[(60, 72, "A `typeof` check narrows a representation without establishing its contract. Parse input at its I/O boundary, then branch on the domain value.")],
            },
        ],
    );
}

/// tests/no-runtime-typeof.test.ts: allowInTypeGuards changes the verdict.
#[test]
fn v1_type_guards_disallowed() {
    check(
        "anti-slop/no-runtime-typeof",
        r#"[{"allowInTypeGuards":false}]"#,
        &[
            Case {
                name: "type guard",
                path: "case.ts",
                code: "export function isText(value: string | number): value is string {\n\treturn typeof value === \"string\";\n}\n",
                expected: &[(74, 86, "A `typeof` check narrows a representation without establishing its contract. Parse input at its I/O boundary, then branch on the domain value.")],
            },
        ],
    );
}

/// tests/no-runtime-typeof.test.ts: allowInTypeGuards changes the verdict.
#[test]
fn v1_type_guards_allowed() {
    check(
        "anti-slop/no-runtime-typeof",
        r#"[{"allowInTypeGuards":true}]"#,
        &[
            Case {
                name: "type guard",
                path: "case.ts",
                code: "export function isText(value: string | number): value is string {\n\treturn typeof value === \"string\";\n}\n",
                expected: &[],
            },
        ],
    );
}

#[test]
fn guards_by_default() {
    check(
        "anti-slop/no-runtime-typeof",
        "[]",
        &[
            Case {
                name: "guards",
                path: "case.ts",
                code: "export const a = (v: unknown): v is string => typeof v === \"string\";\nexport function b(v: unknown): asserts v is string {\n\tif (typeof v !== \"string\") throw new Error(\"x\");\n}\nexport function c(v: unknown): asserts v {\n\tif (typeof v === \"undefined\") throw new Error(\"x\");\n}\nexport function d(v: unknown): v is string {\n\tconst inner = (): boolean => typeof v === \"string\";\n\treturn inner();\n}\nexport function e(v: unknown): boolean {\n\tconst inner = function (): v is string {\n\t\treturn typeof v === \"string\";\n\t};\n\treturn inner();\n}\nexport class C {\n\tis(v: unknown): v is string {\n\t\treturn typeof v === \"string\";\n\t}\n}\nexport const top = typeof globalThis;\nexport type T = typeof top;\n",
                expected: &[(46, 54, "A `typeof` check narrows a representation without establishing its contract. Parse input at its I/O boundary, then branch on the domain value."), (127, 135, "A `typeof` check narrows a representation without establishing its contract. Parse input at its I/O boundary, then branch on the domain value."), (222, 230, "A `typeof` check narrows a representation without establishing its contract. Parse input at its I/O boundary, then branch on the domain value."), (347, 355, "A `typeof` check narrows a representation without establishing its contract. Parse input at its I/O boundary, then branch on the domain value."), (481, 489, "A `typeof` check narrows a representation without establishing its contract. Parse input at its I/O boundary, then branch on the domain value."), (584, 592, "A `typeof` check narrows a representation without establishing its contract. Parse input at its I/O boundary, then branch on the domain value."), (631, 648, "A `typeof` check narrows a representation without establishing its contract. Parse input at its I/O boundary, then branch on the domain value.")],
            },
        ],
    );
}

#[test]
fn guards_allowed() {
    check(
        "anti-slop/no-runtime-typeof",
        r#"[{"allowInTypeGuards":true}]"#,
        &[
            Case {
                name: "guards",
                path: "case.ts",
                code: "export const a = (v: unknown): v is string => typeof v === \"string\";\nexport function b(v: unknown): asserts v is string {\n\tif (typeof v !== \"string\") throw new Error(\"x\");\n}\nexport function c(v: unknown): asserts v {\n\tif (typeof v === \"undefined\") throw new Error(\"x\");\n}\nexport function d(v: unknown): v is string {\n\tconst inner = (): boolean => typeof v === \"string\";\n\treturn inner();\n}\nexport function e(v: unknown): boolean {\n\tconst inner = function (): v is string {\n\t\treturn typeof v === \"string\";\n\t};\n\treturn inner();\n}\nexport class C {\n\tis(v: unknown): v is string {\n\t\treturn typeof v === \"string\";\n\t}\n}\nexport const top = typeof globalThis;\nexport type T = typeof top;\n",
                expected: &[(347, 355, "A `typeof` check narrows a representation without establishing its contract. Parse input at its I/O boundary, then branch on the domain value."), (631, 648, "A `typeof` check narrows a representation without establishing its contract. Parse input at its I/O boundary, then branch on the domain value.")],
            },
        ],
    );
}
