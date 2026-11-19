//! `quality/no-reflect-apply`: expectations recorded from `@nkzw/oxlint-config` 3.0.0's
//! `quality` plugin under oxlint 1.94.0: byte spans and exact messages.
//!
//! The first tables replay `anti-slop/no-reflect-apply`'s cases
//! (`anti_slop/tests/no_reflect_apply.rs`, same names) with the findings the
//! upstream plugin produced for them.

use super::{Case, check};

/// tests/no-reflect-apply.test.ts
#[test]
fn v1() {
    check(
        "quality/no-reflect-apply",
        "[]",
        &[
            Case { name: "direct call", path: "case.ts", code: "export const value = ((a: number): number => a)(1);\n", expected: &[] },
            Case { name: "unrelated member call", path: "case.ts", code: "export const value = Math.max(1, 2);\n", expected: &[] },
            Case {
                name: "local Reflect binding",
                path: "case.ts",
                code: "const Reflect = { apply: (): number => 1 };\n\nexport const value = Reflect.apply();\n",
                expected: &[],
            },
            Case {
                name: "Reflect.apply",
                path: "case.ts",
                code: "export function call(fn: (a: number) => number, a: number): number {\n\treturn Reflect.apply(fn, undefined, [a]);\n}\n",
                expected: &[(77, 110, "Replace `Reflect.apply` with a typed function call. Model dynamic dispatch behind a named interface.")],
            },
            Case {
                name: "computed Reflect access",
                path: "case.ts",
                code: "export function call(fn: (a: number) => number, a: number): number {\n\treturn Reflect[\"apply\"](fn, undefined, [a]);\n}\n",
                expected: &[(77, 113, "Replace `Reflect.apply` with a typed function call. Model dynamic dispatch behind a named interface.")],
            },
        ],
    );
}

/// `anti-slop/no-reflect-apply`'s `edges` table.
#[test]
fn edges() {
    check(
        "quality/no-reflect-apply",
        "[]",
        &[
            Case { name: "other Reflect methods", path: "case.ts", code: "Reflect.get({}, \"a\");\nReflect.has({}, \"a\");\n", expected: &[] },
            Case {
                name: "optional and parenthesized",
                path: "case.ts",
                code: "declare const fn: () => void;\nReflect?.apply(fn, undefined, []);\n(Reflect).apply(fn, undefined, []);\n(Reflect.apply)(fn, undefined, []);\n",
                expected: &[
                    (30, 63, "Replace `Reflect.apply` with a typed function call. Model dynamic dispatch behind a named interface."),
                    (65, 99, "Replace `Reflect.apply` with a typed function call. Model dynamic dispatch behind a named interface."),
                    (101, 135, "Replace `Reflect.apply` with a typed function call. Model dynamic dispatch behind a named interface."),
                ],
            },
            Case { name: "not called", path: "case.ts", code: "export const apply = Reflect.apply;\n", expected: &[] },
            Case {
                name: "super calls are ignored",
                path: "case.ts",
                code: "class A extends Object {\n\tconstructor() {\n\t\tsuper();\n\t}\n}\nexport { A };\n",
                expected: &[],
            },
        ],
    );
}
