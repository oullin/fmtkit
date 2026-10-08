//! `anti-slop/no-unsafe-dictionary-type`: expectations recorded from the v1 plugin under oxlint 1.86.

use super::{Case, check};

/// tests/no-unsafe-dictionary-type.test.ts
#[test]
fn v1() {
    check(
        "anti-slop/no-unsafe-dictionary-type",
        "[]",
        &[
            Case { name: "concrete value type", path: "case.ts", code: "export type Totals = Record<string, number>;\n", expected: &[] },
            Case {
                name: "unknown value type",
                path: "case.ts",
                code: "export type Totals = Record<string, unknown>;\n",
                expected: &[(
                    21,
                    44,
                    "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                )],
            },
            Case {
                name: "index signature of unknown",
                path: "case.ts",
                code: "export type Totals = { [key: string]: unknown };\n",
                expected: &[(
                    21,
                    47,
                    "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                )],
            },
            Case {
                name: "union containing unknown",
                path: "case.ts",
                code: "export type Totals = Record<string, number | unknown>;\n",
                expected: &[(
                    21,
                    53,
                    "This dictionary's union value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                )],
            },
        ],
    );
}

#[test]
fn value_kinds() {
    check(
        "anti-slop/no-unsafe-dictionary-type",
        "[]",
        &[
            Case {
                name: "escape hatches",
                path: "case.ts",
                code: "export type A = Record<string, any>;\nexport type B = Record<string, object>;\nexport type C = Record<string, {}>;\nexport type D = Record<string, { a?: never }>;\nexport type E = Record<string, { a?: never; b: number }>;\nexport type F = Record<string, { a: never }>;\n",
                expected: &[
                    (
                        16,
                        35,
                        "This dictionary's any value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        53,
                        75,
                        "This dictionary's object value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        93,
                        111,
                        "This dictionary's empty-object value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        129,
                        158,
                        "This dictionary's empty-object value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                ],
            },
            Case {
                name: "intersections",
                path: "case.ts",
                code: "export type A = Record<string, unknown & any>;\nexport type B = Record<string, unknown & object>;\nexport type C = Record<string, unknown & string>;\nexport type D = Record<string, string & any>;\n",
                expected: &[
                    (
                        16,
                        45,
                        "This dictionary's any value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        63,
                        95,
                        "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        163,
                        191,
                        "This dictionary's any value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                ],
            },
            Case {
                name: "wrappers",
                path: "case.ts",
                code: "export type A = Record<string, Readonly<unknown>>;\nexport type B = Readonly<Record<string, unknown>>;\nexport type C = Partial<{ [key: string]: object }>;\nexport type D = Record<string, (unknown)>;\nexport type E = Record<string, readonly unknown[]>;\nexport type F = Required<Record<string, NonNullable<any>>>;\n",
                expected: &[
                    (
                        16,
                        49,
                        "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        67,
                        100,
                        "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        118,
                        152,
                        "This dictionary's object value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        170,
                        195,
                        "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        265,
                        307,
                        "This dictionary's any value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                ],
            },
            Case {
                name: "pick and omit",
                path: "case.ts",
                code: "export type A = Pick<Record<string, unknown>, \"a\">;\nexport type B = Omit<{ [key: string]: any }, \"a\">;\n",
                expected: &[
                    (
                        16,
                        50,
                        "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        68,
                        101,
                        "This dictionary's any value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                ],
            },
            Case {
                name: "mapped types",
                path: "case.ts",
                code: "export type A = { [K in string]: unknown };\nexport type B = { [K in string]?: number };\nexport type C = { [K in string] };\n",
                expected: &[(
                    16,
                    42,
                    "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                )],
            },
            Case {
                name: "interfaces",
                path: "case.ts",
                code: "interface Empty {}\ninterface Never {\n\ta?: never;\n}\ninterface Full {\n\ta: number;\n}\ninterface Merged {}\ninterface Merged {}\ninterface Child extends Full {}\n\nexport type A = Record<string, Empty>;\nexport type B = Record<string, Never>;\nexport type C = Record<string, Full>;\nexport type D = Record<string, Merged>;\nexport type E = Record<string, Child>;\n",
                expected: &[
                    (
                        171,
                        192,
                        "This dictionary's empty-object value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        210,
                        231,
                        "This dictionary's empty-object value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                ],
            },
        ],
    );
}

#[test]
fn aliases() {
    check(
        "anti-slop/no-unsafe-dictionary-type",
        "[]",
        &[
            Case {
                name: "value aliases",
                path: "case.ts",
                code: "type Raw = unknown;\ntype Loose = Raw | number;\nexport type A = Record<string, Raw>;\nexport type B = Record<string, Loose>;\n",
                expected: &[
                    (
                        63,
                        82,
                        "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        100,
                        121,
                        "This dictionary's union value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                ],
            },
            Case {
                name: "dictionary aliases and consumers",
                path: "case.ts",
                code: "type Bag = Record<string, unknown>;\nexport type A = Bag;\nexport const b: Bag = {};\nexport function f(x: Bag): Bag {\n\treturn x;\n}\nexport type C = Readonly<Bag>;\n",
                expected: &[
                    (
                        11,
                        34,
                        "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        52,
                        55,
                        "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        145,
                        158,
                        "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                ],
            },
            Case {
                name: "generic aliases",
                path: "case.ts",
                code: "type Dict<V> = Record<string, V>;\ntype WithDefault<V = unknown> = { [key: string]: V };\ntype Same<T> = T;\nexport type A = Dict<unknown>;\nexport type B = Dict<number>;\nexport type C = WithDefault;\nexport type D = Dict;\nexport type E = Dict<Same<any>>;\nexport type F = Same<Record<string, object>>;\n",
                expected: &[
                    (
                        122,
                        135,
                        "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        183,
                        194,
                        "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        234,
                        249,
                        "This dictionary's any value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        267,
                        295,
                        "This dictionary's object value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                ],
            },
            Case {
                name: "substitution chains",
                path: "case.ts",
                code: "type Outer<T> = Inner<T>;\ntype Inner<U> = Record<string, U>;\ntype Pass<T> = Record<string, T>;\ntype Deep<T> = Pass<T>;\nexport type A = Outer<unknown>;\nexport type B = Deep<any>;\nexport type C = Outer<string>;\n",
                expected: &[
                    (
                        135,
                        149,
                        "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        167,
                        176,
                        "This dictionary's any value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                ],
            },
            Case {
                name: "self referential arguments",
                path: "case.ts",
                code: "type Wrap<T> = Record<string, T>;\nexport type A<T> = Wrap<T>;\nexport function f<T>(x: Wrap<T>): T {\n\treturn x as never;\n}\n",
                expected: &[],
            },
            Case {
                name: "recursive aliases",
                path: "case.ts",
                code: "type Loop = Loop;\ntype Tree = Record<string, Tree>;\nexport type A = Record<string, Loop>;\nexport type B = Tree;\n",
                expected: &[],
            },
            Case {
                name: "shadowed built-ins",
                path: "case.ts",
                code: "import type { Record } from \"./types\";\nexport type A = Record<string, unknown>;\n",
                expected: &[],
            },
            Case {
                name: "locally declared Partial",
                path: "case.ts",
                code: "type Partial<T> = T;\ninterface Readonly {}\nexport type A = Partial<Record<string, unknown>>;\nexport type B = Record<string, Readonly>;\n",
                expected: &[
                    (
                        59,
                        91,
                        "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        109,
                        133,
                        "This dictionary's empty-object value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                ],
            },
            Case {
                name: "class and function shadow",
                path: "case.ts",
                code: "export class Pick {}\nexport function Omit(): void {}\nexport type A = Pick<Record<string, unknown>, \"a\">;\nexport type B = Omit<Record<string, any>, \"a\">;\n",
                expected: &[
                    (
                        74,
                        97,
                        "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        126,
                        145,
                        "This dictionary's any value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                ],
            },
            Case {
                name: "enum shadows and duplicates",
                path: "case.ts",
                code: "enum Required {}\nexport type A = Required<Record<string, unknown>>;\n",
                expected: &[(
                    42,
                    65,
                    "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                )],
            },
        ],
    );
}

#[test]
fn positions() {
    check(
        "anti-slop/no-unsafe-dictionary-type",
        "[]",
        &[
            Case {
                name: "nested dictionaries report the outermost",
                path: "case.ts",
                code: "export type A = Record<string, Record<string, unknown>>;\nexport type B = { [key: string]: { [key: string]: any } };\nexport type C = Array<Record<string, unknown>>;\nexport type D = Record<string, unknown>[];\n",
                expected: &[
                    (
                        31,
                        54,
                        "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        90,
                        112,
                        "This dictionary's any value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        138,
                        161,
                        "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        180,
                        203,
                        "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                ],
            },
            Case {
                name: "interface and class index signatures",
                path: "case.ts",
                code: "export interface I {\n\t[key: string]: unknown;\n}\nexport interface J {\n\t[key: string]: number;\n}\nexport class C {\n\t[key: string]: any;\n}\n",
                expected: &[
                    (
                        22,
                        45,
                        "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        113,
                        132,
                        "This dictionary's any value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                ],
            },
            Case {
                name: "annotations and assertions",
                path: "case.ts",
                code: "export const a: Record<string, unknown> = {};\nexport const b = {} as { [key: string]: object };\nexport function f(x: Readonly<Record<string, {}>>): void {}\n",
                expected: &[
                    (
                        16,
                        39,
                        "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        69,
                        94,
                        "This dictionary's object value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        117,
                        145,
                        "This dictionary's empty-object value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                ],
            },
            Case {
                name: "inside alias with type args",
                path: "case.ts",
                code: "type Box<T> = { value: T };\nexport type A = Box<Record<string, unknown>>;\nexport type B = Box<{ [key: string]: unknown }>;\n",
                expected: &[
                    (
                        48,
                        71,
                        "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        94,
                        120,
                        "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                ],
            },
            Case {
                name: "qualified names are skipped",
                path: "case.ts",
                code: "declare namespace NS {\n\ttype R = Record<string, unknown>;\n}\nexport type A = NS.R;\n",
                expected: &[(
                    33,
                    56,
                    "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                )],
            },
        ],
    );
}
