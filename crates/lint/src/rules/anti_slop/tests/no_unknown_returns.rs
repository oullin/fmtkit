//! `anti-slop/no-unknown-returns`: expectations recorded from the v1 plugin under oxlint 1.86.

use super::{Case, check};

/// tests/no-unknown-returns.test.ts
#[test]
fn v1() {
    check(
        "anti-slop/no-unknown-returns",
        "[]",
        &[
            Case { name: "domain return", path: "case.ts", code: "export function read(): string {\n\treturn \"value\";\n}\n", expected: &[] },
            Case {
                name: "promise of a domain type",
                path: "case.ts",
                code: "export async function read(): Promise<string> {\n\treturn \"value\";\n}\n",
                expected: &[],
            },
            Case {
                name: "unknown return",
                path: "case.ts",
                code: "export function read(): unknown {\n\treturn \"value\";\n}\n",
                expected: &[(24, 31, "This function exposes `unknown` to its caller. Parse the value at its boundary and return a named domain type.")],
            },
            Case {
                name: "promise of unknown",
                path: "case.ts",
                code: "export async function read(): Promise<unknown> {\n\treturn \"value\";\n}\n",
                expected: &[(30, 46, "This function exposes `unknown` to its caller. Parse the value at its boundary and return a named domain type.")],
            },
        ],
    );
}

#[test]
fn edges() {
    check(
        "anti-slop/no-unknown-returns",
        "[]",
        &[
            Case {
                name: "forms",
                path: "case.ts",
                code: "export const a = (): unknown => 1;\nexport const b = function (): (unknown) {\n\treturn 1;\n};\ndeclare function c(): PromiseLike<unknown>;\nexport interface I {\n\t(): unknown;\n\tnew (): unknown;\n\tm(): string | unknown;\n}\nexport type F = () => Promise<Promise<unknown>>;\nexport type G = new () => unknown;\nexport abstract class A {\n\tabstract m(): unknown;\n}\n",
                expected: &[
                    (21, 28, "This function exposes `unknown` to its caller. Parse the value at its boundary and return a named domain type."),
                    (66, 73, "This function exposes `unknown` to its caller. Parse the value at its boundary and return a named domain type."),
                    (113, 133, "This function exposes `unknown` to its caller. Parse the value at its boundary and return a named domain type."),
                    (161, 168, "This function exposes `unknown` to its caller. Parse the value at its boundary and return a named domain type."),
                    (179, 186, "This function exposes `unknown` to its caller. Parse the value at its boundary and return a named domain type."),
                    (194, 210, "This function exposes `unknown` to its caller. Parse the value at its boundary and return a named domain type."),
                    (236, 261, "This function exposes `unknown` to its caller. Parse the value at its boundary and return a named domain type."),
                    (289, 296, "This function exposes `unknown` to its caller. Parse the value at its boundary and return a named domain type."),
                    (339, 346, "This function exposes `unknown` to its caller. Parse the value at its boundary and return a named domain type."),
                ],
            },
            Case {
                name: "aliases",
                path: "case.ts",
                code: "type Raw = unknown;\ntype Outer = Raw | string;\ntype Gen<T> = unknown;\ntype Loop = Loop;\n\nexport function a(): Raw {\n\treturn 1;\n}\nexport function b(): Promise<Outer> {\n\treturn Promise.resolve(1);\n}\nexport function c(): Gen<number> {\n\treturn 1;\n}\nexport function d(): Loop {\n\treturn 1 as never;\n}\nexport function e(): Promise {\n\treturn 1 as never;\n}\n",
                expected: &[
                    (110, 113, "This function exposes `unknown` to its caller. Parse the value at its boundary and return a named domain type."),
                    (150, 164, "This function exposes `unknown` to its caller. Parse the value at its boundary and return a named domain type."),
                ],
            },
            Case {
                name: "shadowed by type parameters",
                path: "case.ts",
                code: "type T = unknown;\n\nexport function a<T>(): T {\n\treturn 1 as never;\n}\nexport type M = { [T in string]: () => T };\nexport type C = number extends infer T ? () => T : () => T;\n",
                expected: &[(170, 171, "This function exposes `unknown` to its caller. Parse the value at its boundary and return a named domain type.")],
            },
            Case { name: "predicates", path: "case.ts", code: "export function a(v: number): v is unknown {\n\treturn true;\n}\n", expected: &[] },
        ],
    );
}
