//! `quality/no-known-value-widening`: expectations recorded from `@nkzw/oxlint-config` 3.0.0's
//! `quality` plugin under oxlint 1.94.0: byte spans and exact messages.
//!
//! The first tables replay `anti-slop/no-known-value-widening`'s cases
//! (`anti_slop/tests/no_known_value_widening.rs`, same names) with the findings the
//! upstream plugin produced for them.
//! `type_bindings` is upstream's `test/type-bindings.test.js`, one case per
//! file with that file's findings for this rule.
//! `differential` holds fmtkit's own probes of how the plugin resolves
//! types, recorded the same way.

use super::{Case, check};

/// tests/no-known-value-widening.test.ts
#[test]
fn v1() {
    check(
        "quality/no-known-value-widening",
        "[]",
        &[
            Case { name: "inference keeps the evidence", path: "case.ts", code: "export const owner = { id: \"a\", total: 1 };\n", expected: &[] },
            Case {
                name: "named contract",
                path: "case.ts",
                code: "type Owner = { readonly id: string };\n\nexport const owner: Owner = { id: \"a\" };\n",
                expected: &[],
            },
            Case {
                name: "widened to an open record",
                path: "case.ts",
                code: "export const owner: Record<string, unknown> = { id: \"a\", total: 1 };\n",
                expected: &[(
                    46,
                    67,
                    "The explicit open dictionary type on binding `owner` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                )],
            },
        ],
    );
}

/// `anti-slop/no-known-value-widening`'s `targets` table.
#[test]
fn targets() {
    check(
        "quality/no-known-value-widening",
        "[]",
        &[
            Case {
                name: "unknown and object",
                path: "case.ts",
                code: "export const a: unknown = 1;\nexport const b: object = [1];\nexport const c: unknown = \"x\";\nexport const d: unknown = `x`;\nexport const e: unknown = -1;\n",
                expected: &[
                    (
                        26,
                        27,
                        "The explicit unknown type on binding `a` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        54,
                        57,
                        "The explicit object type on binding `b` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        85,
                        88,
                        "The explicit unknown type on binding `c` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        116,
                        119,
                        "The explicit unknown type on binding `d` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        147,
                        149,
                        "The explicit unknown type on binding `e` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                ],
            },
            Case {
                name: "anonymous object and index signature",
                path: "case.ts",
                code: "export const a: { id: string } = { id: \"a\" };\nexport const b: { [key: string]: number } = { a: 1 };\nexport const c: {} = { id: 1 };\n",
                expected: &[
                    (
                        33,
                        44,
                        "The explicit anonymous object type on binding `a` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        90,
                        98,
                        "The explicit open dictionary type on binding `b` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                ],
            },
            Case {
                name: "mapped type",
                path: "case.ts",
                code: "export const a: { [K in string]: number } = { a: 1 };\n",
                expected: &[(
                    44,
                    52,
                    "The explicit open dictionary type on binding `a` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                )],
            },
            Case {
                name: "transparent wrappers",
                path: "case.ts",
                code: "export const a: Readonly<Record<string, number>> = { a: 1 };\nexport const b: Partial<{ id: string }> = { id: \"a\" };\nexport const c: Readonly<unknown> = 1;\nexport const d: (unknown) = 1;\n",
                expected: &[
                    (
                        51,
                        59,
                        "The explicit open dictionary type on binding `a` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        103,
                        114,
                        "The explicit anonymous object type on binding `b` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        152,
                        153,
                        "The explicit unknown type on binding `c` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        183,
                        184,
                        "The explicit unknown type on binding `d` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                ],
            },
            Case {
                name: "shadowed Record",
                path: "case.ts",
                code: "import type { Record } from \"./types\";\n\nexport const a: Record<string, number> = { a: 1 };\n",
                expected: &[],
            },
            Case {
                name: "local Record alias shadows the built-in",
                path: "case.ts",
                code: "type Record<K, V> = { k: K; v: V };\n\nexport const a: Record<string, number> = { a: 1 };\n",
                expected: &[],
            },
            Case {
                name: "alias to unknown",
                path: "case.ts",
                code: "type Raw = unknown;\ntype Bag = object;\ntype Dict = { [key: string]: number };\ntype Literal = { id: string };\n\nexport const a: Raw = 1;\nexport const b: Bag = [1];\nexport const c: Dict = { a: 1 };\nexport const d: Literal = { id: \"a\" };\n",
                expected: &[
                    (
                        132,
                        133,
                        "The explicit unknown type on binding `a` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        157,
                        160,
                        "The explicit object type on binding `b` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        185,
                        193,
                        "The explicit open dictionary type on binding `c` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                ],
            },
            Case {
                name: "alias chains",
                path: "case.ts",
                code: "type Raw = unknown;\ntype Outer = Raw;\ntype Wrapped = Readonly<Outer>;\n\nexport const a: Outer = 1;\nexport const b: Wrapped = 1;\n",
                expected: &[
                    (
                        95,
                        96,
                        "The explicit unknown type on binding `a` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        124,
                        125,
                        "The explicit unknown type on binding `b` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                ],
            },
            Case {
                name: "generic container alias",
                path: "case.ts",
                code: "type Bag<T> = Record<string, T>;\ntype Box<T> = { value: T };\ntype Dflt<T = number> = { [key: string]: T };\n\nexport const a: Bag<number> = { a: 1 };\nexport const b: Box<number> = { value: 1 };\nexport const c: Dflt = { a: 1 };\nexport const d: Bag = { a: 1 };\n",
                expected: &[
                    (
                        138,
                        146,
                        "The explicit generic container type on binding `a` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        215,
                        223,
                        "The explicit generic container type on binding `c` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                ],
            },
            Case {
                name: "alias to mapped type with broad key",
                path: "case.ts",
                code: "type Dict<V> = { [K in PropertyKey]: V };\ntype Keyed = { [K in string | number]: number };\ntype Narrow = { [K in \"a\" | \"b\"]: number };\n\nexport const a: Keyed = { a: 1 };\nexport const b: Narrow = { a: 1, b: 2 };\n",
                expected: &[(
                    160,
                    168,
                    "The explicit open dictionary type on binding `a` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                )],
            },
            Case {
                name: "alias with substitution",
                path: "case.ts",
                code: "type Of<T> = T;\ntype Unk = Of<unknown>;\n\nexport const a: Unk = 1;\n",
                expected: &[(
                    63,
                    64,
                    "The explicit unknown type on binding `a` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                )],
            },
            Case { name: "recursive aliases do not loop", path: "case.ts", code: "type A = B;\ntype B = A;\n\nexport const a: A = 1;\n", expected: &[] },
        ],
    );
}

/// `anti-slop/no-known-value-widening`'s `evidence` table.
#[test]
fn evidence() {
    check(
        "quality/no-known-value-widening",
        "[]",
        &[
            Case {
                name: "known evidence kinds",
                path: "case.ts",
                code: "export const a: unknown = [1];\nexport const b: unknown = () => 1;\nexport const c: unknown = class {};\nexport const d: unknown = function () {};\nexport const e: unknown = new Map();\nexport const f: unknown = /x/u;\nexport const g: unknown = null;\nexport const h: unknown = 1n;\nexport const i: unknown = true;\nexport const j: unknown = typeof a;\n",
                expected: &[
                    (
                        26,
                        29,
                        "The explicit unknown type on binding `a` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        57,
                        64,
                        "The explicit unknown type on binding `b` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        92,
                        100,
                        "The explicit unknown type on binding `c` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        128,
                        142,
                        "The explicit unknown type on binding `d` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        170,
                        179,
                        "The explicit unknown type on binding `e` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        207,
                        211,
                        "The explicit unknown type on binding `f` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        239,
                        243,
                        "The explicit unknown type on binding `g` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        271,
                        273,
                        "The explicit unknown type on binding `h` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        301,
                        305,
                        "The explicit unknown type on binding `i` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        333,
                        341,
                        "The explicit unknown type on binding `j` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                ],
            },
            Case {
                name: "no evidence",
                path: "case.ts",
                code: "declare function read(): number;\ndeclare const value: number;\nexport const a: unknown = read();\nexport const b: unknown = value;\nexport const c: unknown = undefined;\nexport const d: unknown = a + 1;\n",
                expected: &[],
            },
            Case {
                name: "wrapped evidence",
                path: "case.ts",
                code: "export const a: unknown = ({ id: 1 });\nexport const b: unknown = ({ id: 1 } as const);\nexport const c: unknown = { id: 1 }!;\nexport const d: unknown = ({ id: 1 }) satisfies object;\nexport const e: unknown = <const>[1];\n",
                expected: &[
                    (
                        27,
                        36,
                        "The explicit unknown type on binding `a` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        66,
                        84,
                        "The explicit unknown type on binding `b` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        113,
                        123,
                        "The explicit unknown type on binding `c` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        151,
                        179,
                        "The explicit unknown type on binding `d` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        207,
                        217,
                        "The explicit unknown type on binding `e` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                ],
            },
            Case {
                name: "const binding evidence",
                path: "case.ts",
                code: "const base = { id: 1 };\nconst alias = base;\nexport const a: unknown = alias;\nexport const b: unknown = (base as never);\n",
                expected: &[
                    (
                        70,
                        75,
                        "The explicit unknown type on binding `a` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        104,
                        117,
                        "The explicit unknown type on binding `b` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                ],
            },
            Case {
                name: "let binding is not evidence",
                path: "case.ts",
                code: "let base = { id: 1 };\nexport const a: unknown = base;\nbase = { id: 2 };\n",
                expected: &[],
            },
            Case { name: "reassigned const-like evidence", path: "case.ts", code: "var base = { id: 1 };\nexport const a: unknown = base;\n", expected: &[] },
            Case {
                name: "destructured binding uses the declarator init",
                path: "case.ts",
                code: "const { id } = { id: 1 };\nconst [first] = [1];\nexport const a: unknown = id;\nexport const b: unknown = first;\n",
                expected: &[
                    (
                        73,
                        75,
                        "The explicit unknown type on binding `a` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        103,
                        108,
                        "The explicit unknown type on binding `b` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                ],
            },
            Case { name: "self reference does not loop", path: "case.ts", code: "const a: unknown = a;\n", expected: &[] },
            Case { name: "parameter is not evidence", path: "case.ts", code: "export function f(x: number): unknown {\n\treturn x;\n}\n", expected: &[] },
            Case {
                name: "empty object into dictionary is allowed",
                path: "case.ts",
                code: "type Bag<T> = Record<string, T>;\nexport const a: Record<string, number> = {};\nexport const b: Bag<number> = {};\nexport const c: unknown = {};\nexport const d: { [key: string]: number } = ({} as never);\n",
                expected: &[(
                    138,
                    140,
                    "The explicit unknown type on binding `c` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                )],
            },
        ],
    );
}

/// `anti-slop/no-known-value-widening`'s `flows` table.
#[test]
fn flows() {
    check(
        "quality/no-known-value-widening",
        "[]",
        &[
            Case {
                name: "class properties",
                path: "case.ts",
                code: "export class A {\n\ta: unknown = { id: 1 };\n\t\"b\": object = [1];\n\t#c: unknown = 1;\n\t[Symbol.iterator]: unknown = 1;\n\t2: unknown = 1;\n\t0x10: unknown = 1;\n\taccessor d: unknown = 1;\n\te: unknown;\n\tf = 1;\n}\n",
                expected: &[
                    (
                        31,
                        40,
                        "The explicit unknown type on property `a` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        57,
                        60,
                        "The explicit object type on property `b` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        77,
                        78,
                        "The explicit unknown type on property `c` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        110,
                        111,
                        "The explicit unknown type on property `Symbol.iterator` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        127,
                        128,
                        "The explicit unknown type on property `2` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        147,
                        148,
                        "The explicit unknown type on property `16` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        173,
                        174,
                        "The explicit unknown type on property `d` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                ],
            },
            Case {
                name: "assignments",
                path: "case.ts",
                code: "let a: unknown;\na = { id: 1 };\na += 1;\nlet b: number;\nb = 1;\nundeclared = 1;\nfunction g() {}\ng = () => {};\n",
                expected: &[(
                    20,
                    29,
                    "The explicit unknown type on binding `a` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                )],
            },
            Case {
                name: "returns",
                path: "case.ts",
                code: "export function a(): unknown {\n\treturn { id: 1 };\n}\nexport const b = function (): object {\n\treturn [1];\n};\nexport const c = (): unknown => {\n\treturn 1;\n};\nexport const d = function named(): unknown {\n\treturn 1;\n};\nexport class E {\n\tm(): unknown {\n\t\treturn 1;\n\t}\n\t\"s\"(): unknown {\n\t\treturn 2;\n\t}\n\tf = (): unknown => {\n\t\treturn 3;\n\t};\n}\nexport default function (): unknown {\n\treturn 4;\n}\nexport const o = {\n\tm(): unknown {\n\t\treturn 5;\n\t},\n};\n",
                expected: &[
                    (
                        39,
                        48,
                        "The explicit unknown type on return value of `a` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        99,
                        102,
                        "The explicit object type on return value of `b` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        149,
                        150,
                        "The explicit unknown type on return value of `c` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        208,
                        209,
                        "The explicit unknown type on return value of `named` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        256,
                        257,
                        "The explicit unknown type on return value of `m` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        289,
                        290,
                        "The explicit unknown type on return value of `s` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        326,
                        327,
                        "The explicit unknown type on return value of `anonymous function` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        381,
                        382,
                        "The explicit unknown type on return value of `anonymous function` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        430,
                        431,
                        "The explicit unknown type on return value of `anonymous function` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                ],
            },
            Case {
                name: "expression arrow bodies",
                path: "case.ts",
                code: "export const a = (): unknown => ({ id: 1 });\nexport const b = ((): object => [1]);\nexport const c = (): unknown => read();\ndeclare function read(): number;\nexport const d = [1].map((): unknown => 1);\n",
                expected: &[
                    (
                        33,
                        42,
                        "The explicit unknown type on return value of `a` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        77,
                        80,
                        "The explicit object type on return value of `b` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        196,
                        197,
                        "The explicit unknown type on return value of `anonymous function` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                ],
            },
            Case {
                name: "assertions",
                path: "case.ts",
                code: "export const a = { id: 1 } as unknown;\nexport const b = <object>[1];\nexport const c = ({ id: 1 } as unknown) as object;\nexport const d = { id: 1 } as Record<string, unknown>;\nexport const e = {} as Record<string, unknown>;\n",
                expected: &[
                    (
                        17,
                        26,
                        "The explicit unknown type on assertion discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        64,
                        67,
                        "The explicit object type on assertion discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        87,
                        107,
                        "The explicit object type on assertion discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        137,
                        146,
                        "The explicit open dictionary type on assertion discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                ],
            },
            Case { name: "return outside function", path: "case.ts", code: "export {};\n", expected: &[] },
            Case {
                name: "nested function names",
                path: "case.ts",
                code: "export function outer(): void {\n\tconst inner = (): unknown => {\n\t\tfunction deep(): object {\n\t\t\treturn [1];\n\t\t}\n\t\treturn deep;\n\t};\n\tinner();\n}\n",
                expected: &[(
                    102,
                    105,
                    "The explicit object type on return value of `deep` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
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
        "quality/no-known-value-widening",
        "[]",
        &[
            Case {
                name: "broad.ts",
                path: "broad.ts",
                code: "export function accept(value: Input): Result { return value; }\nexport type Bag = Record<string, Input>;\nexport function local<Input, Result>(value: Input): Result { throw value; }\ntype Input = object;\ntype Result = unknown;",
                expected: &[],
            },
            Case {
                name: "interfaces.ts",
                path: "interfaces.ts",
                code: "export type Outer = Record<string, Value>;\ninterface Value {}\nexport function count<Value extends string>(items: Record<string, Value>) {}\nfunction local() {\n  type Inner = Record<string, Value>;\n  interface Value { id: number }\n}\nfunction nested<Value extends string>() {\n  { interface Value {}; type Inner = Record<string, Value>; }\n}\ninterface Merged {}\ninterface Merged { id: number }\nexport type Combined = Record<string, Merged>;\nexport type Mapped = { [Value in 'key']: Record<string, Value> };\nexport type Inferred<T> = T extends Promise<infer Value>\n  ? Record<string, Value>\n  : Record<string, Value>;",
                expected: &[],
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
                expected: &[
                    (
                        36,
                        41,
                        "The explicit unknown type on argument for parameter `input` of `check` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        227,
                        232,
                        "The explicit unknown type on binding `raw` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                ],
            },
        ],
    );
}

/// Differential probes: generic aliases, unions, nesting, shadowed
/// built-ins and parameter forms.
#[test]
fn differential() {
    check(
        "quality/no-known-value-widening",
        "[]",
        &[
            Case {
                name: "predicate call literal",
                path: "case.ts",
                code: "function check(input: unknown): input is string { return typeof input === 'string'; }\ncheck('a');\ncheck(1 as number);\ncheck(x);\n",
                expected: &[
                    (
                        92,
                        95,
                        "The explicit unknown type on argument for parameter `input` of `check` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        104,
                        115,
                        "The explicit unknown type on argument for parameter `input` of `check` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                ],
            },
            Case {
                name: "predicate const arrow",
                path: "case.ts",
                code: "const isA = (v: unknown): v is string => true;\nconst o = { a: 1 };\nisA(o);\nisA(o as any);\n",
                expected: &[(
                    71,
                    72,
                    "The explicit unknown type on argument for parameter `v` of `isA` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                )],
            },
            Case {
                name: "predicate typed variable",
                path: "case.ts",
                code: "function check(input: unknown): input is string { return true; }\nconst a: string = 'x';\nlet b: unknown = 1;\nlet c: Record<string, unknown> = {};\ncheck(a);\ncheck(b);\ncheck(c);\n",
                expected: &[
                    (
                        105,
                        106,
                        "The explicit unknown type on binding `b` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        151,
                        152,
                        "The explicit unknown type on argument for parameter `input` of `check` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        171,
                        172,
                        "The explicit unknown type on argument for parameter `input` of `check` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                ],
            },
            Case {
                name: "predicate call result",
                path: "case.ts",
                code: "function check(input: unknown): input is string { return true; }\nfunction make(): string { return ''; }\nfunction raw(): unknown { return 1; }\ncheck(make());\ncheck(raw());\n",
                expected: &[
                    (
                        137,
                        138,
                        "The explicit unknown type on return value of `raw` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        148,
                        154,
                        "The explicit unknown type on argument for parameter `input` of `check` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                ],
            },
            Case {
                name: "predicate param name",
                path: "case.ts",
                code: "function check(this: void, a: number, input: unknown | string): input is string { return true; }\ncheck(1, 'x');\n",
                expected: &[],
            },
            Case {
                name: "predicate method not tracked",
                path: "case.ts",
                code: "const o = { check(input: unknown): input is string { return true; } };\no.check('x');\n",
                expected: &[],
            },
            Case {
                name: "predicate iife",
                path: "case.ts",
                code: "((v: unknown): v is string => true)('x');\n",
                expected: &[(
                    36,
                    39,
                    "The explicit unknown type on argument for parameter `v` of `anonymous function` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                )],
            },
            Case {
                name: "predicate parameter annotated",
                path: "case.ts",
                code: "function f(input: string) {\n\tfunction check(v: unknown): v is string { return true; }\n\tcheck(input);\n}\n",
                expected: &[(
                    93,
                    98,
                    "The explicit unknown type on argument for parameter `v` of `check` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                )],
            },
            Case {
                name: "predicate object pattern",
                path: "case.ts",
                code: "const { isA } = { isA: (v: unknown): v is string => true };\nisA('x');\n",
                expected: &[],
            },
            Case {
                name: "Record key narrow",
                path: "case.ts",
                code: "const a: Record<'a' | 'b', number> = { a: 1, b: 2 };\nconst b: Record<string, number> = { a: 1 };\nconst c: Record<'a' | string, number> = { a: 1 };\n",
                expected: &[
                    (
                        87,
                        95,
                        "The explicit open dictionary type on binding `b` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        137,
                        145,
                        "The explicit open dictionary type on binding `c` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                ],
            },
            Case {
                name: "Record key alias",
                path: "case.ts",
                code: "type K = 'a';\ntype S = string;\nconst a: Record<K, number> = { a: 1 };\nconst b: Record<S, number> = { a: 1 };\nconst c: Record<PropertyKey, number> = { a: 1 };\n",
                expected: &[
                    (
                        99,
                        107,
                        "The explicit open dictionary type on binding `b` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        148,
                        156,
                        "The explicit open dictionary type on binding `c` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                ],
            },
            Case {
                name: "Record missing key",
                path: "case.ts",
                code: "const a: Record = { a: 1 };\n",
                expected: &[(
                    18,
                    26,
                    "The explicit open dictionary type on binding `a` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                )],
            },
            Case {
                name: "mapped union key",
                path: "case.ts",
                code: "type M = { [K in 'a' | string]: number };\nconst a: M = { a: 1 };\n",
                expected: &[(
                    55,
                    63,
                    "The explicit open dictionary type on binding `a` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                )],
            },
            Case {
                name: "generic container not dictionary",
                path: "case.ts",
                code: "type Box<T> = { value: T };\ntype Dict<T> = Record<string, T>;\ntype Named<T> = Record<'a', T>;\nconst a: Box<number> = { value: 1 };\nconst b: Dict<number> = { a: 1 };\nconst c: Named<number> = { a: 1 };\n",
                expected: &[(
                    155,
                    163,
                    "The explicit generic container type on binding `b` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                )],
            },
            Case {
                name: "generic container index literal",
                path: "case.ts",
                code: "type L<T> = { [k: string]: T };\nconst a: L<number> = { a: 1 };\n",
                expected: &[(
                    53,
                    61,
                    "The explicit generic container type on binding `a` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                )],
            },
            Case {
                name: "local alias scoping",
                path: "case.ts",
                code: "type B = string;\nfunction f() {\n\ttype B = unknown;\n\tconst x: B = 1;\n}\nconst y: B = 'a';\n",
                expected: &[(
                    65,
                    66,
                    "The explicit unknown type on binding `x` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                )],
            },
            Case {
                name: "shadowed Record",
                path: "case.ts",
                code: "type Record<K, V> = { k: K; v: V };\nconst a: Record<string, number> = { k: 'a', v: 1 };\n",
                expected: &[],
            },
            Case { name: "shadowed Readonly", path: "case.ts", code: "function f<Readonly>() {\n\tconst a: Readonly<unknown> = 1;\n}\n", expected: &[] },
            Case {
                name: "type param shadows alias",
                path: "case.ts",
                code: "type U = unknown;\nfunction f<U>(v: U) {\n\tconst a: U = 1 as U;\n}\n",
                expected: &[],
            },
            Case {
                name: "assertion unknown",
                path: "case.ts",
                code: "const a = 1 as unknown;\nconst b = { a: 1 } as object;\n",
                expected: &[
                    (
                        10,
                        11,
                        "The explicit unknown type on assertion discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        34,
                        42,
                        "The explicit object type on assertion discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                ],
            },
            Case {
                name: "return predicate",
                path: "case.ts",
                code: "function f(): Record<string, number> { return { a: 1 }; }\n",
                expected: &[(
                    46,
                    54,
                    "The explicit open dictionary type on return value of `f` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                )],
            },
            Case {
                name: "predicate redeclared",
                path: "case.ts",
                code: "function check(input: unknown): input is string { return true; }\nfunction check(input: unknown): input is string;\ncheck('a');\n",
                expected: &[],
            },
            Case {
                name: "predicate local shadow",
                path: "case.ts",
                code: "function check(input: unknown): input is string { return true; }\nfunction g(check: (v: unknown) => boolean) { check('a'); }\n",
                expected: &[],
            },
            Case {
                name: "predicate parenthesized callee",
                path: "case.ts",
                code: "function check(input: unknown): input is string { return true; }\n(check)('a');\n",
                expected: &[(
                    73,
                    76,
                    "The explicit unknown type on argument for parameter `input` of `check` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                )],
            },
            Case {
                name: "predicate spread arg",
                path: "case.ts",
                code: "function check(input: unknown): input is string { return true; }\ncheck(...['a']);\n",
                expected: &[],
            },
            Case {
                name: "predicate asserts",
                path: "case.ts",
                code: "function check(input: unknown): asserts input is string {}\ncheck('a');\n",
                expected: &[(
                    65,
                    68,
                    "The explicit unknown type on argument for parameter `input` of `check` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                )],
            },
            Case {
                name: "predicate arg satisfies",
                path: "case.ts",
                code: "function check(input: unknown): input is string { return true; }\ncheck({ a: 1 } satisfies object);\ncheck(<string>'x');\ncheck('x'!);\n",
                expected: &[
                    (
                        71,
                        96,
                        "The explicit unknown type on argument for parameter `input` of `check` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        105,
                        116,
                        "The explicit unknown type on argument for parameter `input` of `check` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        125,
                        129,
                        "The explicit unknown type on argument for parameter `input` of `check` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                ],
            },
            Case {
                name: "predicate default param",
                path: "case.ts",
                code: "function check(input: unknown = 1): input is string { return true; }\ncheck('a');\n",
                expected: &[(
                    75,
                    78,
                    "The explicit unknown type on argument for parameter `input` of `check` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                )],
            },
            Case {
                name: "predicate dict typed var",
                path: "case.ts",
                code: "function check(input: unknown): input is string { return true; }\nconst d: { [k: string]: unknown } = {};\nconst e: {} = {};\ncheck(d);\ncheck(e);\n",
                expected: &[(
                    129,
                    130,
                    "The explicit unknown type on argument for parameter `input` of `check` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                )],
            },
            Case {
                name: "predicate let",
                path: "case.ts",
                code: "let check = (input: unknown): input is string => true;\ncheck('a');\n",
                expected: &[(
                    61,
                    64,
                    "The explicit unknown type on argument for parameter `input` of `check` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                )],
            },
            Case {
                name: "predicate declared before name",
                path: "case.ts",
                code: "check('a');\nconst check = function (input: unknown): input is string { return true; };\n",
                expected: &[(
                    6,
                    9,
                    "The explicit unknown type on argument for parameter `input` of `check` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                )],
            },
            Case {
                name: "predicate typed param variable",
                path: "case.ts",
                code: "function check(input: unknown): input is string { return true; }\nfunction g(p: number, q: unknown, ...r: string[]) {\n\tcheck(p);\n\tcheck(q);\n\tcheck(r);\n}\n",
                expected: &[
                    (
                        124,
                        125,
                        "The explicit unknown type on argument for parameter `input` of `check` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        146,
                        147,
                        "The explicit unknown type on argument for parameter `input` of `check` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                ],
            },
            Case {
                name: "class properties and accessors",
                path: "case.ts",
                code: "export class C {\n\ta: unknown = 1;\n\tb: Record<string, number> = { a: 1 };\n\tc: Record<'x', number> = { x: 1 };\n\taccessor d: object = {};\n}\n",
                expected: &[
                    (
                        31,
                        32,
                        "The explicit unknown type on property `a` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        63,
                        71,
                        "The explicit open dictionary type on property `b` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        131,
                        133,
                        "The explicit object type on property `d` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                ],
            },
            Case {
                name: "assignment to annotated",
                path: "case.ts",
                code: "let a: unknown;\na = 1;\nlet b: Record<'k', number>;\nb = { k: 1 };\nlet c: Record<string, number>;\nc = { k: 1 };\n",
                expected: &[
                    (
                        20,
                        21,
                        "The explicit unknown type on binding `a` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        100,
                        108,
                        "The explicit open dictionary type on binding `c` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                ],
            },
            Case {
                name: "returns",
                path: "case.ts",
                code: "export function f(): unknown { return 1; }\nexport const g = (): Record<'a', number> => ({ a: 1 });\nexport const h = (): Record<string, number> => ({ a: 1 });\n",
                expected: &[
                    (
                        38,
                        39,
                        "The explicit unknown type on return value of `f` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        147,
                        155,
                        "The explicit open dictionary type on return value of `h` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                ],
            },
            Case {
                name: "generic container broad key",
                path: "case.ts",
                code: "type Dict<T> = Record<string, T>;\ntype Keyed<K extends string, V> = Record<K, V>;\nconst a: Dict<number> = { a: 1 };\nconst b: Keyed<'a', number> = { a: 1 };\nconst c: Keyed<string, number> = { a: 1 };\n",
                expected: &[
                    (
                        106,
                        114,
                        "The explicit generic container type on binding `a` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        189,
                        197,
                        "The explicit generic container type on binding `c` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                ],
            },
            Case {
                name: "generic literal and mapped",
                path: "case.ts",
                code: "type L<T> = { [k: string]: T };\ntype M<T> = { [K in 'a']: T };\ntype N<T> = { [K in string]: T };\nconst a: L<number> = { a: 1 };\nconst b: M<number> = { a: 1 };\nconst c: N<number> = { a: 1 };\n",
                expected: &[
                    (
                        118,
                        126,
                        "The explicit generic container type on binding `a` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        180,
                        188,
                        "The explicit generic container type on binding `c` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                ],
            },
            Case {
                name: "record key union alias some",
                path: "case.ts",
                code: "type K = 'a' | string;\nconst a: Record<K, number> = { a: 1 };\nconst b: Record<'a' | 'b', number> = { a: 1, b: 1 };\n",
                expected: &[(
                    52,
                    60,
                    "The explicit open dictionary type on binding `a` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                )],
            },
            Case { name: "record key generic alias", path: "case.ts", code: "type G<T> = T;\nconst a: Record<G<string>, number> = { a: 1 };\n", expected: &[] },
            Case {
                name: "Readonly Record",
                path: "case.ts",
                code: "const a: Readonly<Record<'a', number>> = { a: 1 };\nconst b: Readonly<Record<string, number>> = { a: 1 };\n",
                expected: &[(
                    95,
                    103,
                    "The explicit open dictionary type on binding `b` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                )],
            },
            Case {
                name: "predicate method call this param",
                path: "case.ts",
                code: "function check(this: void, input: unknown): input is string { return true; }\ncheck('a');\ncheck(1, 'a' as never);\n",
                expected: &[(
                    98,
                    110,
                    "The explicit unknown type on argument for parameter `input` of `check` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                )],
            },
            Case {
                name: "predicate overloads",
                path: "case.ts",
                code: "function check(input: string): input is string;\nfunction check(input: unknown): input is string;\nfunction check(input: unknown): input is string { return true; }\ncheck('a');\n",
                expected: &[],
            },
            Case {
                name: "predicate declared function",
                path: "case.ts",
                code: "declare function check(input: unknown): input is string;\ncheck('a');\ncheck([1]);\n",
                expected: &[
                    (
                        63,
                        66,
                        "The explicit unknown type on argument for parameter `input` of `check` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        75,
                        78,
                        "The explicit unknown type on argument for parameter `input` of `check` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                ],
            },
            Case {
                name: "predicate union unknown",
                path: "case.ts",
                code: "function check(input: string | unknown): input is string { return true; }\nfunction other(input: object): input is string { return true; }\ncheck(1);\nother({});\n",
                expected: &[(
                    144,
                    145,
                    "The explicit unknown type on argument for parameter `input` of `check` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                )],
            },
            Case {
                name: "predicate argument variables",
                path: "case.ts",
                code: "function check(input: unknown): input is string { return true; }\nconst a = { x: 1 };\nconst b = a;\nlet c = 1;\nconst d = c;\nconst [e] = [1];\ncheck(b);\ncheck(d);\ncheck(e);\n",
                expected: &[
                    (
                        145,
                        146,
                        "The explicit unknown type on argument for parameter `input` of `check` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        165,
                        166,
                        "The explicit unknown type on argument for parameter `input` of `check` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                ],
            },
            Case {
                name: "predicate as unsafe",
                path: "case.ts",
                code: "function check(input: unknown): input is string { return true; }\ncheck(1 as unknown);\ncheck({} as object);\ncheck({} as Record<string, unknown>);\ncheck({} as {});\n",
                expected: &[
                    (
                        71,
                        72,
                        "The explicit unknown type on assertion discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        92,
                        94,
                        "The explicit object type on assertion discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        113,
                        142,
                        "The explicit unknown type on argument for parameter `input` of `check` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                ],
            },
            Case {
                name: "predicate typed param alias",
                path: "case.ts",
                code: "type U = unknown;\nfunction check(input: U): input is string { return true; }\ncheck('a');\n",
                expected: &[],
            },
            Case {
                name: "predicate var function",
                path: "case.ts",
                code: "var check = function (input: unknown): input is string { return true; };\ncheck('a');\n",
                expected: &[(
                    79,
                    82,
                    "The explicit unknown type on argument for parameter `input` of `check` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                )],
            },
            Case {
                name: "predicate function with name expression",
                path: "case.ts",
                code: "const c = function check(input: unknown): input is string { return true; };\nc('a');\n",
                expected: &[(
                    78,
                    81,
                    "The explicit unknown type on argument for parameter `input` of `check` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                )],
            },
            Case {
                name: "predicate in class method",
                path: "case.ts",
                code: "class C {\n\tcheck(input: unknown): input is string { return true; }\n\tm() { this.check('a'); }\n}\n",
                expected: &[],
            },
            Case {
                name: "predicate generic",
                path: "case.ts",
                code: "function check<T>(input: unknown): input is T { return true; }\ncheck<string>('a');\n",
                expected: &[(
                    77,
                    80,
                    "The explicit unknown type on argument for parameter `input` of `check` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                )],
            },
            Case {
                name: "predicate arg call of unknown fn",
                path: "case.ts",
                code: "function check(input: unknown): input is string { return true; }\ncheck(make());\ncheck((() => 1)());\nconst make2 = (): Record<string, unknown> => ({});\ncheck(make2());\n",
                expected: &[(
                    157,
                    164,
                    "The explicit unknown type on argument for parameter `input` of `check` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                )],
            },
            Case {
                name: "predicate typed var alias",
                path: "case.ts",
                code: "type Bag = {};\nfunction check(input: unknown): input is string { return true; }\nconst a: Bag = {};\nconst b: Array<unknown> = [];\ncheck(a);\ncheck(b);\n",
                expected: &[(
                    145,
                    146,
                    "The explicit unknown type on argument for parameter `input` of `check` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                )],
            },
            Case {
                name: "predicate in nested scope shadow alias",
                path: "case.ts",
                code: "type V = string;\nfunction check(input: unknown): input is string { return true; }\nfunction f() {\n\ttype V = unknown;\n\tconst v: V = 1;\n\tcheck(v);\n}\n",
                expected: &[(
                    130,
                    131,
                    "The explicit unknown type on binding `v` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                )],
            },
            Case {
                name: "predicate parameter destructure name",
                path: "case.ts",
                code: "function check({ a }: unknown): a is string { return true; }\ncheck({ a: 1 });\n",
                expected: &[],
            },
            Case {
                name: "assertion targets",
                path: "case.ts",
                code: "const a = { x: 1 } as Record<'x', number>;\nconst b = { x: 1 } as Record<string, number>;\nconst c = [1] as unknown as number[];\n",
                expected: &[(
                    53,
                    61,
                    "The explicit open dictionary type on assertion discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                )],
            },
            Case {
                name: "parenthesized targets",
                path: "case.ts",
                code: "const a: (unknown) = 1;\nconst b: (Record<('a'), number>) = { a: 1 };\nconst c: Record<(string), number> = { a: 1 };\nconst d = (1) as (unknown);\n",
                expected: &[
                    (
                        21,
                        22,
                        "The explicit unknown type on binding `a` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        105,
                        113,
                        "The explicit open dictionary type on binding `c` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        126,
                        127,
                        "The explicit unknown type on assertion discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                ],
            },
            Case {
                name: "predicate parenthesized parameter type",
                path: "case.ts",
                code: "function check(input: (unknown)): input is string { return true; }\ncheck(('a'));\ncheck((1 as number));\n",
                expected: &[
                    (
                        74,
                        77,
                        "The explicit unknown type on argument for parameter `input` of `check` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        88,
                        99,
                        "The explicit unknown type on argument for parameter `input` of `check` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                ],
            },
        ],
    );
}

/// An argument that mentions a type parameter is resolved where it was
/// written. Reading it inside the alias it is passed to recursed without end.
#[test]
fn compound_arguments_keep_their_scope() {
    check(
        "quality/no-known-value-widening",
        "[]",
        &[
            Case {
                name: "compound record keys",
                path: "case.ts",
                code: "type K<T> = Record<T | string, unknown>;\ntype O<T> = K<T | number>;\nexport type D = O<symbol>;\nexport const x: D = { a: 1 };\nexport const y = { a: 1 } as D;\n",
                expected: &[
                    (
                        115,
                        123,
                        "The explicit open dictionary type on binding `x` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        142,
                        150,
                        "The explicit open dictionary type on assertion discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                ],
            },
            Case {
                name: "nested generic aliases",
                path: "case.ts",
                code: "type Inner<T> = Record<string, T>;\ntype Outer<T> = Inner<T | null>;\nexport type D = Outer<number>;\nexport const x: D = { a: 1 };\nexport const y = { a: 1 } as D;\n",
                expected: &[
                    (
                        119,
                        127,
                        "The explicit open dictionary type on binding `x` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                    (
                        146,
                        154,
                        "The explicit open dictionary type on assertion discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.",
                    ),
                ],
            },
        ],
    );
}
