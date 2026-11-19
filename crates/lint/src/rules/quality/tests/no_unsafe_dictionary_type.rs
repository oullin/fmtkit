//! `quality/no-unsafe-dictionary-type`: expectations recorded from `@nkzw/oxlint-config` 3.0.0's
//! `quality` plugin under oxlint 1.94.0: byte spans and exact messages.
//!
//! The first tables replay `anti-slop/no-unsafe-dictionary-type`'s cases
//! (`anti_slop/tests/no_unsafe_dictionary_type.rs`, same names) with the findings the
//! upstream plugin produced for them.
//! `type_bindings` is upstream's `test/type-bindings.test.js`, one case per
//! file with that file's findings for this rule.
//! `differential` holds fmtkit's own probes of how the plugin resolves
//! types, recorded the same way.

use super::{Case, check};

/// tests/no-unsafe-dictionary-type.test.ts
#[test]
fn v1() {
    check(
        "quality/no-unsafe-dictionary-type",
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

/// `anti-slop/no-unsafe-dictionary-type`'s `value_kinds` table.
#[test]
fn value_kinds() {
    check(
        "quality/no-unsafe-dictionary-type",
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

/// `anti-slop/no-unsafe-dictionary-type`'s `aliases` table.
#[test]
fn aliases() {
    check(
        "quality/no-unsafe-dictionary-type",
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
                        121,
                        151,
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

/// `anti-slop/no-unsafe-dictionary-type`'s `positions` table.
#[test]
fn positions() {
    check(
        "quality/no-unsafe-dictionary-type",
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

/// test/type-bindings.test.js: forward declarations, local predicates and
/// shadows resolve independently in each file.
#[test]
fn type_bindings() {
    check(
        "quality/no-unsafe-dictionary-type",
        "[]",
        &[
            Case {
                name: "broad.ts",
                path: "broad.ts",
                code: "export function accept(value: Input): Result { return value; }\nexport type Bag = Record<string, Input>;\nexport function local<Input, Result>(value: Input): Result { throw value; }\ntype Input = object;\ntype Result = unknown;",
                expected: &[(
                    81,
                    102,
                    "This dictionary's object value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                )],
            },
            Case {
                name: "interfaces.ts",
                path: "interfaces.ts",
                code: "export type Outer = Record<string, Value>;\ninterface Value {}\nexport function count<Value extends string>(items: Record<string, Value>) {}\nfunction local() {\n  type Inner = Record<string, Value>;\n  interface Value { id: number }\n}\nfunction nested<Value extends string>() {\n  { interface Value {}; type Inner = Record<string, Value>; }\n}\ninterface Merged {}\ninterface Merged { id: number }\nexport type Combined = Record<string, Merged>;\nexport type Mapped = { [Value in 'key']: Record<string, Value> };\nexport type Inferred<T> = T extends Promise<infer Value>\n  ? Record<string, Value>\n  : Record<string, Value>;",
                expected: &[
                    (
                        20,
                        41,
                        "This dictionary's empty-object value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        310,
                        331,
                        "This dictionary's empty-object value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        589,
                        610,
                        "This dictionary's empty-object value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                ],
            },
            Case {
                name: "narrow.ts",
                path: "narrow.ts",
                code: "export function accept(value: Input): Result { return value; }\nexport type Bag = Record<string, Input>;\nexport function local<Input, Result>(value: Input): Result { throw value; }\ntype Input = string;\ntype Result = number;",
                expected: &[],
            },
            Case {
                name: "predicate.ts",
                path: "predicate.ts",
                code: "const value = { name: \"ok\" };\ncheck(value);\nfunction check(input: unknown): input is string { return typeof input === \"string\"; }\nfunction nested(check: (input: object) => boolean) { return check(value); }\nconst raw: unknown = value;\nexport const cast = raw as string;\nfunction keep(raw: unknown) { return raw as string; }",
                expected: &[],
            },
        ],
    );
}

/// Differential probes: generic aliases, unions, nesting, shadowed
/// built-ins and parameter forms.
#[test]
fn differential() {
    check(
        "quality/no-unsafe-dictionary-type",
        "[]",
        &[
            Case {
                name: "shadowed Omit function",
                path: "case.ts",
                code: "export function Omit(): void {}\nexport type B = Omit<Record<string, any>, 'a'>;\n",
                expected: &[(
                    48,
                    78,
                    "This dictionary's any value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                )],
            },
            Case {
                name: "shadowed Pick class",
                path: "case.ts",
                code: "export class Pick {}\nexport type A = Pick<Record<string, unknown>, 'a'>;\n",
                expected: &[(
                    42,
                    65,
                    "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                )],
            },
            Case {
                name: "shadowed Record function",
                path: "case.ts",
                code: "export function Record(): void {}\nexport type A = Record<string, unknown>;\n",
                expected: &[(
                    50,
                    73,
                    "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                )],
            },
            Case {
                name: "shadowed Readonly variable",
                path: "case.ts",
                code: "const Readonly = 1;\nexport type A = Readonly<Record<string, unknown>>;\n",
                expected: &[(
                    36,
                    69,
                    "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                )],
            },
            Case {
                name: "shadowed Partial in nested scope",
                path: "case.ts",
                code: "function f() {\n\ttype Partial<T> = T;\n\ttype A = Partial<Record<string, unknown>>;\n}\ntype B = Partial<Record<string, unknown>>;\n",
                expected: &[
                    (
                        47,
                        79,
                        "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        92,
                        124,
                        "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                ],
            },
            Case {
                name: "shadowed Record type param",
                path: "case.ts",
                code: "export type A<Record> = { a: Record };\nexport function f<Record>(): Record<string, unknown> { return {}; }\n",
                expected: &[],
            },
            Case {
                name: "local interface empty",
                path: "case.ts",
                code: "function f() {\n\tinterface V {}\n\ttype A = Record<string, V>;\n}\n",
                expected: &[(
                    41,
                    58,
                    "This dictionary's empty-object value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                )],
            },
            Case {
                name: "nested alias consumer",
                path: "case.ts",
                code: "function f() {\n\ttype A = Record<string, unknown>;\n\tconst x: A = {};\n}\n",
                expected: &[(
                    25,
                    48,
                    "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                )],
            },
            Case {
                name: "namespace alias consumer",
                path: "case.ts",
                code: "namespace N {\n\texport type A = Record<string, unknown>;\n\tconst x: A = {};\n}\nconst y: N.A = {};\n",
                expected: &[(
                    31,
                    54,
                    "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                )],
            },
            Case {
                name: "type parameter constraint",
                path: "case.ts",
                code: "export function f<T extends Record<string, unknown>>(x: T): T { return x; }\n",
                expected: &[],
            },
            Case {
                name: "alias generic",
                path: "case.ts",
                code: "type Dict<V> = Record<string, V>;\nexport type A = Dict<unknown>;\nexport type B = Dict<string>;\nconst c: Dict<any> = {};\n",
                expected: &[
                    (
                        50,
                        63,
                        "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        104,
                        113,
                        "This dictionary's any value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                ],
            },
            Case {
                name: "import shadows Record",
                path: "case.ts",
                code: "import { Record } from './r';\nexport type A = Record<string, unknown>;\n",
                expected: &[],
            },
            Case { name: "duplicate interface", path: "case.ts", code: "interface V {}\ninterface V {}\nexport type A = Record<string, V>;\n", expected: &[] },
            Case {
                name: "class shadows interface name",
                path: "case.ts",
                code: "interface V {}\nfunction f() {\n\tclass V {}\n\ttype A = Record<string, V>;\n}\n",
                expected: &[],
            },
            Case {
                name: "alias shadowed by type param",
                path: "case.ts",
                code: "type U = unknown;\nexport type A<U> = Record<string, U>;\nexport type B = Record<string, U>;\n",
                expected: &[(
                    72,
                    89,
                    "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                )],
            },
            Case {
                name: "PropertyKey shadow",
                path: "case.ts",
                code: "type PropertyKey = 'a';\nexport type A = { [k: string]: unknown };\n",
                expected: &[(
                    40,
                    64,
                    "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                )],
            },
            Case { name: "enum shadow", path: "case.ts", code: "enum Record { A }\nexport type A = Record<string, unknown>;\n", expected: &[] },
            Case {
                name: "forward alias",
                path: "case.ts",
                code: "export type A = Record<string, Later>;\ntype Later = unknown;\n",
                expected: &[(
                    16,
                    37,
                    "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                )],
            },
            Case {
                name: "index signature in class",
                path: "case.ts",
                code: "export class C {\n\t[k: string]: unknown;\n}\nexport interface I {\n\t[k: string]: any;\n}\n",
                expected: &[
                    (
                        18,
                        39,
                        "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        64,
                        81,
                        "This dictionary's any value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                ],
            },
            Case {
                name: "mapped type with infer",
                path: "case.ts",
                code: "export type M<T> = T extends Record<string, infer V> ? V : never;\n",
                expected: &[],
            },
            Case {
                name: "generic alias with default unsafe",
                path: "case.ts",
                code: "type Dict<V = unknown> = Record<string, V>;\nexport type A = Dict;\nexport type B = Dict<string>;\nconst c: Dict = {};\n",
                expected: &[(
                    60,
                    64,
                    "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                )],
            },
            Case {
                name: "readonly record",
                path: "case.ts",
                code: "export type A = Readonly<Record<string, unknown>>;\nexport type B = readonly (Record<string, any>)[];\nexport type C = Partial<{ [k: string]: object }>;\n",
                expected: &[
                    (
                        16,
                        49,
                        "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        77,
                        96,
                        "This dictionary's any value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        117,
                        149,
                        "This dictionary's object value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                ],
            },
            Case {
                name: "pick of alias",
                path: "case.ts",
                code: "type Bag = Record<string, unknown>;\nexport type A = Pick<Bag, 'a'>;\nexport type B = Omit<Bag, 'a'>;\n",
                expected: &[
                    (
                        11,
                        34,
                        "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        52,
                        66,
                        "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        84,
                        98,
                        "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                ],
            },
            Case {
                name: "constraint nested deep",
                path: "case.ts",
                code: "export function f<T extends { a: Record<string, unknown> }>(x: T): void {}\nexport type C<T extends Record<string, any> = Record<string, any>> = T;\n",
                expected: &[(
                    121,
                    140,
                    "This dictionary's any value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                )],
            },
            Case {
                name: "interface index signature aliased value",
                path: "case.ts",
                code: "type U = unknown;\nexport interface I {\n\t[k: string]: U;\n}\nexport class C {\n\t[k: string]: object;\n}\n",
                expected: &[
                    (
                        40,
                        55,
                        "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        76,
                        96,
                        "This dictionary's object value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                ],
            },
            Case {
                name: "alias consumer inside alias",
                path: "case.ts",
                code: "type Bag = Record<string, unknown>;\nexport type A = Bag;\nexport type B = { bag: Bag };\nexport function f(b: Bag): void {}\n",
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
                        80,
                        83,
                        "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                ],
            },
            Case {
                name: "local Record alias",
                path: "case.ts",
                code: "function f() {\n\ttype Record<K, V> = { k: K; v: V };\n\ttype A = Record<string, unknown>;\n}\nexport type B = Record<string, unknown>;\n",
                expected: &[(
                    105,
                    128,
                    "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                )],
            },
            Case {
                name: "empty interface with extends",
                path: "case.ts",
                code: "interface Base { a: 1 }\ninterface V extends Base {}\ninterface W { a?: never }\nexport type A = Record<string, V>;\nexport type B = Record<string, W>;\n",
                expected: &[(
                    129,
                    146,
                    "This dictionary's empty-object value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                )],
            },
            Case { name: "class value", path: "case.ts", code: "class V {}\nexport type A = Record<string, V>;\n", expected: &[] },
            Case {
                name: "intersection values",
                path: "case.ts",
                code: "export type A = Record<string, unknown & {}>;\nexport type B = Record<string, any & string>;\nexport type C = Record<string, object & string>;\n",
                expected: &[
                    (
                        16,
                        44,
                        "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        62,
                        90,
                        "This dictionary's any value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                ],
            },
            Case {
                name: "mapped key over alias",
                path: "case.ts",
                code: "type U = unknown;\nexport type M = { [K in string]: U };\nexport type N<U> = { [K in string]: U };\n",
                expected: &[(
                    34,
                    54,
                    "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                )],
            },
            Case {
                name: "nested dictionaries",
                path: "case.ts",
                code: "export type A = Record<string, Record<string, unknown>>;\nexport type B = { [k: string]: { [j: string]: any } };\n",
                expected: &[
                    (
                        31,
                        54,
                        "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        88,
                        108,
                        "This dictionary's any value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                ],
            },
            Case {
                name: "namespace import shadow",
                path: "case.ts",
                code: "import * as Record from './r';\nexport type A = Record<string, unknown>;\n",
                expected: &[],
            },
            Case {
                name: "default import shadow",
                path: "case.ts",
                code: "import Partial from './p';\nexport type A = Partial<Record<string, unknown>>;\n",
                expected: &[(
                    51,
                    74,
                    "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                )],
            },
            Case {
                name: "shadowed wrapper by function",
                path: "case.ts",
                code: "function Partial() {}\nexport type A = Partial<Record<string, unknown>>;\n",
                expected: &[(
                    38,
                    70,
                    "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                )],
            },
            Case {
                name: "type param named like interface",
                path: "case.ts",
                code: "interface V {}\nexport type A<V> = Record<string, V>;\nexport function f<V>(): Record<string, V> { return {}; }\n",
                expected: &[],
            },
            Case {
                name: "parenthesized dictionaries",
                path: "case.ts",
                code: "export type A = (Record<string, unknown>);\nexport type B = ({ [k: string]: any });\nexport type C = Array<(Record<string, object>)>;\n",
                expected: &[
                    (
                        17,
                        40,
                        "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        60,
                        80,
                        "This dictionary's any value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        106,
                        128,
                        "This dictionary's object value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                ],
            },
            Case {
                name: "parenthesized inside dictionary",
                path: "case.ts",
                code: "export type A = Record<string, (Record<string, unknown>)>;\nexport type B = { [k: string]: ({ [j: string]: any }) };\n",
                expected: &[
                    (
                        32,
                        55,
                        "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        91,
                        111,
                        "This dictionary's any value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                ],
            },
            Case {
                name: "readonly array of record",
                path: "case.ts",
                code: "export type A = readonly Record<string, unknown>[];\nexport type B = ReadonlyArray<Record<string, any>>;\n",
                expected: &[
                    (
                        25,
                        48,
                        "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        82,
                        101,
                        "This dictionary's any value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                ],
            },
            Case {
                name: "keyof dictionary",
                path: "case.ts",
                code: "export type A = keyof Record<string, unknown>;\nexport type B = keyof { [k: string]: any };\n",
                expected: &[
                    (
                        22,
                        45,
                        "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        69,
                        89,
                        "This dictionary's any value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                ],
            },
            Case {
                name: "union of dictionaries",
                path: "case.ts",
                code: "export type A = Record<string, unknown> | Record<string, any>;\nexport type B = (Record<string, unknown> | string)[];\n",
                expected: &[
                    (
                        16,
                        39,
                        "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        42,
                        61,
                        "This dictionary's any value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        80,
                        103,
                        "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                ],
            },
            Case {
                name: "parenthesized value",
                path: "case.ts",
                code: "export type A = Record<string, (unknown)>;\nexport type B = Record<string, (string | (any))>;\n",
                expected: &[
                    (
                        16,
                        41,
                        "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        59,
                        91,
                        "This dictionary's union value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                ],
            },
            Case {
                name: "readonly parenthesized",
                path: "case.ts",
                code: "export type A = readonly (string)[];\nexport type B = Readonly<(Record<string, unknown>)>;\n",
                expected: &[(
                    53,
                    88,
                    "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                )],
            },
        ],
    );
}

/// An argument that mentions a type parameter is resolved where it was
/// written. Reading it inside the alias it is passed to recursed without end.
#[test]
fn compound_arguments_keep_their_scope() {
    check(
        "quality/no-unsafe-dictionary-type",
        "[]",
        &[
            Case {
                name: "nested generic aliases",
                path: "case.ts",
                code: "type Inner<T> = Record<string, T>;\ntype Outer<T> = Inner<T | null>;\nexport type D = Outer<number>;\nexport type E = Outer<unknown>;\nexport type F = Inner<unknown>;\n",
                expected: &[
                    (
                        115,
                        129,
                        "This dictionary's union value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        147,
                        161,
                        "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                ],
            },
            Case {
                name: "compound record keys",
                path: "case.ts",
                code: "type K<T> = Record<T | string, unknown>;\ntype O<T> = K<T | number>;\nexport type D = O<symbol>;\nexport const x: D = {};\n",
                expected: &[
                    (
                        12,
                        39,
                        "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        53,
                        66,
                        "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                    (
                        84,
                        93,
                        "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                    ),
                ],
            },
            Case {
                name: "explicit arguments resolve where they are written",
                path: "case.ts",
                code: "type A = string;\ntype Dict<A, B> = Record<string, B>;\nexport type D = Dict<unknown, A>;\n",
                expected: &[],
            },
            Case {
                name: "defaults see earlier parameters, not the caller",
                path: "case.ts",
                code: "type Pair<T, U = T> = Record<string, U>;\ntype Use<U> = Pair<number>;\nexport type A = Use<unknown>;\nexport type B = Pair<unknown>;\n",
                expected: &[(
                    115,
                    128,
                    "This dictionary's unknown value type gives callers no concrete value contract. Use an owner/schema-derived value type; parse external payloads before insertion.",
                )],
            },
        ],
    );
}
