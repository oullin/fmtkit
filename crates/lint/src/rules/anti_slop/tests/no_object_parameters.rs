//! `anti-slop/no-object-parameters`: expectations recorded from the v1 plugin under oxlint 1.86.

use super::{Case, check};

/// tests/no-object-parameters.test.ts
#[test]
fn v1() {
    check(
        "anti-slop/no-object-parameters",
        "[]",
        &[
            Case {
                name: "named contract",
                path: "case.ts",
                code: "export function read(owner: { readonly id: string }): string {\n\treturn owner.id;\n}\n",
                expected: &[],
            },
            Case {
                name: "object parameter",
                path: "case.ts",
                code: "export function read(owner: object): string {\n\treturn String(owner);\n}\n",
                expected: &[(
                    28,
                    34,
                    "Parameter `owner` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                )],
            },
            Case {
                name: "alias of object",
                path: "case.ts",
                code: "type Bag = object;\n\nexport function read(owner: Bag): string {\n\treturn String(owner);\n}\n",
                expected: &[(
                    48,
                    51,
                    "Parameter `owner` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                )],
            },
        ],
    );
}

#[test]
fn owners() {
    check(
        "anti-slop/no-object-parameters",
        "[]",
        &[
            Case {
                name: "every function kind",
                path: "case.ts",
                code: "export const a = (x: object): void => {};\nexport const b = function (x: object): void {};\ndeclare function c(x: object): void;\nexport class D {\n\tconstructor(private readonly x: object) {}\n\tm(x: object): void {}\n\tabstract?: (x: object) => void;\n}\nexport abstract class E {\n\tabstract m(x: object): void;\n}\nexport interface F {\n\t(x: object): void;\n\tnew (x: object): F;\n\tm(x: object): void;\n}\nexport type G = new (x: object) => F;\n",
                expected: &[
                    (
                        21,
                        27,
                        "Parameter `x` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                    (
                        72,
                        78,
                        "Parameter `x` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                    (
                        112,
                        118,
                        "Parameter `x` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                    (
                        177,
                        183,
                        "Parameter `private readonly x` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                    (
                        194,
                        200,
                        "Parameter `x` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                    (
                        227,
                        233,
                        "Parameter `x` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                    (
                        287,
                        293,
                        "Parameter `x` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                    (
                        330,
                        336,
                        "Parameter `x` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                    (
                        354,
                        360,
                        "Parameter `x` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                    (
                        372,
                        378,
                        "Parameter `x` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                    (
                        413,
                        419,
                        "Parameter `x` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                ],
            },
            Case {
                name: "parameter forms",
                path: "case.ts",
                code: "export function f(this: object, { a }: object, [b]: object, d: object = {}, e?: object, ...c: object): void {}\n",
                expected: &[
                    (
                        24,
                        30,
                        "Parameter `this` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                    (
                        39,
                        45,
                        "Parameter `{ a }` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                    (
                        52,
                        58,
                        "Parameter `[b]` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                    (
                        63,
                        69,
                        "Parameter `d: object = {}` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                    (
                        80,
                        86,
                        "Parameter `e` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                    (
                        94,
                        100,
                        "Parameter `...c` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                ],
            },
            Case {
                name: "parameter text keeps a default",
                path: "case.ts",
                code: "export function f({ a }:object = {}, [b] : object): void {}\n",
                expected: &[
                    (
                        24,
                        30,
                        "Parameter `{ a }:object = {}` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                    (
                        43,
                        49,
                        "Parameter `[b]` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                ],
            },
            Case {
                name: "unions and parentheses",
                path: "case.ts",
                code: "export function f(a: object | null, b: (object), c: string | (number | object), d: object[], e: Partial<object>): void {}\n",
                expected: &[
                    (
                        21,
                        34,
                        "Parameter `a` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                    (
                        40,
                        46,
                        "Parameter `b` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                    (
                        52,
                        78,
                        "Parameter `c` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                ],
            },
            Case {
                name: "alias forms",
                path: "case.ts",
                code: "type A = object;\ntype B = A;\ntype C = B | undefined;\ntype G<T> = object;\nexport type D = (C);\ntype Self = Self;\n\nexport function f(a: B, b: C, c: G<string>, d: D, e: Self, f: A<number>): void {}\n",
                expected: &[
                    (
                        134,
                        135,
                        "Parameter `a` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                    (
                        140,
                        141,
                        "Parameter `b` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                    (
                        160,
                        161,
                        "Parameter `d` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                ],
            },
            Case {
                name: "shadowed by type parameters",
                path: "case.ts",
                code: "type T = object;\ntype K = object;\ntype I = object;\n\nexport function f<T>(a: T): void {}\nexport class C<T> {\n\tm(a: T): void {}\n}\nexport type M = { [K in string]: (a: K) => void };\nexport type N = string extends infer I ? (a: I) => void : (a: I) => void;\nexport interface Q<T> {\n\tm(a: T): void;\n}\nexport type R<T> = (a: T) => void;\n",
                expected: &[(
                    241,
                    242,
                    "Parameter `a` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                )],
            },
            Case {
                name: "qualified names are not aliases",
                path: "case.ts",
                code: "declare namespace NS {\n\ttype A = object;\n}\nexport function f(a: NS.A): void {}\n",
                expected: &[],
            },
            Case {
                name: "non-exported default and other statements",
                path: "case.ts",
                code: "export default function (a: object): void {}\n",
                expected: &[(
                    28,
                    34,
                    "Parameter `a` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                )],
            },
        ],
    );
}
