//! `quality/no-reflect-get`: expectations recorded from `@nkzw/oxlint-config` 3.0.0's
//! `quality` plugin under oxlint 1.94.0: byte spans and exact messages.
//!
//! The first tables replay `anti-slop/no-reflect-get`'s cases
//! (`anti_slop/tests/no_reflect_get.rs`, same names) with the findings the
//! upstream plugin produced for them.

use super::{Case, check};

/// tests/no-reflect-get.test.ts
#[test]
fn v1() {
    check(
        "quality/no-reflect-get",
        "[]",
        &[
            Case {
                name: "typed property access",
                path: "case.ts",
                code: "export function read(owner: { name: string }): string {\n\treturn owner.name;\n}\n",
                expected: &[],
            },
            Case {
                name: "unrelated get method",
                path: "case.ts",
                code: "export function read(owner: Map<string, string>): string | undefined {\n\treturn owner.get(\"name\");\n}\n",
                expected: &[],
            },
            Case {
                name: "Reflect.get",
                path: "case.ts",
                code: "export function read(owner: { name: string }): unknown {\n\treturn Reflect.get(owner, \"name\");\n}\n",
                expected: &[(65, 91, "Replace `Reflect.get` with typed property access. Parse dynamic input into a named domain type before reading it.")],
            },
            Case {
                name: "computed Reflect access",
                path: "case.ts",
                code: "export function read(owner: { name: string }): unknown {\n\treturn Reflect[\"get\"](owner, \"name\");\n}\n",
                expected: &[(65, 94, "Replace `Reflect.get` with typed property access. Parse dynamic input into a named domain type before reading it.")],
            },
        ],
    );
}

/// `anti-slop/no-reflect-get`'s `edges` table.
#[test]
fn edges() {
    check(
        "quality/no-reflect-get",
        "[]",
        &[
            Case {
                name: "shadowed in a function",
                path: "case.ts",
                code: "export function f(Reflect: Map<string, number>): number | undefined {\n\treturn Reflect.get(\"a\");\n}\nexport const b = Reflect.get({}, \"a\");\n",
                expected: &[(115, 135, "Replace `Reflect.get` with typed property access. Parse dynamic input into a named domain type before reading it.")],
            },
            Case { name: "other methods", path: "case.ts", code: "Reflect.apply(() => 1, undefined, []);\nReflect.getPrototypeOf({});\n", expected: &[] },
        ],
    );
}
