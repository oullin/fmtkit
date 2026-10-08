//! `anti-slop/no-known-value-widening`: expectations recorded from the v1 plugin under oxlint 1.86.

use super::{Case, check};

/// tests/no-known-value-widening.test.ts
#[test]
fn v1() {
    check(
        "anti-slop/no-known-value-widening",
        "[]",
        &[
            Case {
                name: "inference keeps the evidence",
                path: "case.ts",
                code: "export const owner = { id: \"a\", total: 1 };\n",
                expected: &[],
            },
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
                expected: &[(46, 67, "The explicit open dictionary type on binding `owner` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.")],
            },
        ],
    );
}

#[test]
fn targets() {
    check(
        "anti-slop/no-known-value-widening",
        "[]",
        &[
            Case {
                name: "unknown and object",
                path: "case.ts",
                code: "export const a: unknown = 1;\nexport const b: object = [1];\nexport const c: unknown = \"x\";\nexport const d: unknown = `x`;\nexport const e: unknown = -1;\n",
                expected: &[(26, 27, "The explicit unknown type on binding `a` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract."), (54, 57, "The explicit object type on binding `b` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract."), (85, 88, "The explicit unknown type on binding `c` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract."), (116, 119, "The explicit unknown type on binding `d` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract."), (147, 149, "The explicit unknown type on binding `e` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.")],
            },
            Case {
                name: "anonymous object and index signature",
                path: "case.ts",
                code: "export const a: { id: string } = { id: \"a\" };\nexport const b: { [key: string]: number } = { a: 1 };\nexport const c: {} = { id: 1 };\n",
                expected: &[(33, 44, "The explicit anonymous object type on binding `a` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract."), (90, 98, "The explicit open dictionary type on binding `b` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.")],
            },
            Case {
                name: "mapped type",
                path: "case.ts",
                code: "export const a: { [K in string]: number } = { a: 1 };\n",
                expected: &[(44, 52, "The explicit open dictionary type on binding `a` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.")],
            },
            Case {
                name: "transparent wrappers",
                path: "case.ts",
                code: "export const a: Readonly<Record<string, number>> = { a: 1 };\nexport const b: Partial<{ id: string }> = { id: \"a\" };\nexport const c: Readonly<unknown> = 1;\nexport const d: (unknown) = 1;\n",
                expected: &[(51, 59, "The explicit open dictionary type on binding `a` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract."), (103, 114, "The explicit anonymous object type on binding `b` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract."), (152, 153, "The explicit unknown type on binding `c` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract."), (183, 184, "The explicit unknown type on binding `d` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.")],
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
                expected: &[(132, 133, "The explicit unknown type on binding `a` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract."), (157, 160, "The explicit object type on binding `b` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract."), (185, 193, "The explicit open dictionary type on binding `c` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.")],
            },
            Case {
                name: "alias chains",
                path: "case.ts",
                code: "type Raw = unknown;\ntype Outer = Raw;\ntype Wrapped = Readonly<Outer>;\n\nexport const a: Outer = 1;\nexport const b: Wrapped = 1;\n",
                expected: &[(95, 96, "The explicit unknown type on binding `a` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract."), (124, 125, "The explicit unknown type on binding `b` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.")],
            },
            Case {
                name: "generic container alias",
                path: "case.ts",
                code: "type Bag<T> = Record<string, T>;\ntype Box<T> = { value: T };\ntype Dflt<T = number> = { [key: string]: T };\n\nexport const a: Bag<number> = { a: 1 };\nexport const b: Box<number> = { value: 1 };\nexport const c: Dflt = { a: 1 };\nexport const d: Bag = { a: 1 };\n",
                expected: &[(138, 146, "The explicit generic container type on binding `a` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract."), (215, 223, "The explicit generic container type on binding `c` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.")],
            },
            Case {
                name: "alias to mapped type with broad key",
                path: "case.ts",
                code: "type Dict<V> = { [K in PropertyKey]: V };\ntype Keyed = { [K in string | number]: number };\ntype Narrow = { [K in \"a\" | \"b\"]: number };\n\nexport const a: Keyed = { a: 1 };\nexport const b: Narrow = { a: 1, b: 2 };\n",
                expected: &[(160, 168, "The explicit open dictionary type on binding `a` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.")],
            },
            Case {
                name: "alias with substitution",
                path: "case.ts",
                code: "type Of<T> = T;\ntype Unk = Of<unknown>;\n\nexport const a: Unk = 1;\n",
                expected: &[(63, 64, "The explicit unknown type on binding `a` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.")],
            },
            Case {
                name: "recursive aliases do not loop",
                path: "case.ts",
                code: "type A = B;\ntype B = A;\n\nexport const a: A = 1;\n",
                expected: &[],
            },
        ],
    );
}

#[test]
fn evidence() {
    check(
        "anti-slop/no-known-value-widening",
        "[]",
        &[
            Case {
                name: "known evidence kinds",
                path: "case.ts",
                code: "export const a: unknown = [1];\nexport const b: unknown = () => 1;\nexport const c: unknown = class {};\nexport const d: unknown = function () {};\nexport const e: unknown = new Map();\nexport const f: unknown = /x/u;\nexport const g: unknown = null;\nexport const h: unknown = 1n;\nexport const i: unknown = true;\nexport const j: unknown = typeof a;\n",
                expected: &[(26, 29, "The explicit unknown type on binding `a` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract."), (57, 64, "The explicit unknown type on binding `b` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract."), (92, 100, "The explicit unknown type on binding `c` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract."), (128, 142, "The explicit unknown type on binding `d` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract."), (170, 179, "The explicit unknown type on binding `e` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract."), (207, 211, "The explicit unknown type on binding `f` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract."), (239, 243, "The explicit unknown type on binding `g` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract."), (271, 273, "The explicit unknown type on binding `h` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract."), (301, 305, "The explicit unknown type on binding `i` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract."), (333, 341, "The explicit unknown type on binding `j` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.")],
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
                expected: &[(27, 36, "The explicit unknown type on binding `a` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract."), (66, 84, "The explicit unknown type on binding `b` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract."), (113, 123, "The explicit unknown type on binding `c` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract."), (151, 179, "The explicit unknown type on binding `d` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract."), (207, 217, "The explicit unknown type on binding `e` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.")],
            },
            Case {
                name: "const binding evidence",
                path: "case.ts",
                code: "const base = { id: 1 };\nconst alias = base;\nexport const a: unknown = alias;\nexport const b: unknown = (base as never);\n",
                expected: &[(70, 75, "The explicit unknown type on binding `a` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract."), (104, 117, "The explicit unknown type on binding `b` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.")],
            },
            Case {
                name: "let binding is not evidence",
                path: "case.ts",
                code: "let base = { id: 1 };\nexport const a: unknown = base;\nbase = { id: 2 };\n",
                expected: &[],
            },
            Case {
                name: "reassigned const-like evidence",
                path: "case.ts",
                code: "var base = { id: 1 };\nexport const a: unknown = base;\n",
                expected: &[],
            },
            Case {
                name: "destructured binding uses the declarator init",
                path: "case.ts",
                code: "const { id } = { id: 1 };\nconst [first] = [1];\nexport const a: unknown = id;\nexport const b: unknown = first;\n",
                expected: &[(73, 75, "The explicit unknown type on binding `a` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract."), (103, 108, "The explicit unknown type on binding `b` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.")],
            },
            Case {
                name: "self reference does not loop",
                path: "case.ts",
                code: "const a: unknown = a;\n",
                expected: &[],
            },
            Case {
                name: "parameter is not evidence",
                path: "case.ts",
                code: "export function f(x: number): unknown {\n\treturn x;\n}\n",
                expected: &[],
            },
            Case {
                name: "empty object into dictionary is allowed",
                path: "case.ts",
                code: "type Bag<T> = Record<string, T>;\nexport const a: Record<string, number> = {};\nexport const b: Bag<number> = {};\nexport const c: unknown = {};\nexport const d: { [key: string]: number } = ({} as never);\n",
                expected: &[(138, 140, "The explicit unknown type on binding `c` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.")],
            },
        ],
    );
}

#[test]
fn flows() {
    check(
        "anti-slop/no-known-value-widening",
        "[]",
        &[
            Case {
                name: "class properties",
                path: "case.ts",
                code: "export class A {\n\ta: unknown = { id: 1 };\n\t\"b\": object = [1];\n\t#c: unknown = 1;\n\t[Symbol.iterator]: unknown = 1;\n\t2: unknown = 1;\n\t0x10: unknown = 1;\n\taccessor d: unknown = 1;\n\te: unknown;\n\tf = 1;\n}\n",
                expected: &[(31, 40, "The explicit unknown type on property `a` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract."), (57, 60, "The explicit object type on property `b` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract."), (77, 78, "The explicit unknown type on property `c` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract."), (110, 111, "The explicit unknown type on property `Symbol.iterator` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract."), (127, 128, "The explicit unknown type on property `2` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract."), (147, 148, "The explicit unknown type on property `16` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract."), (173, 174, "The explicit unknown type on property `d` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.")],
            },
            Case {
                name: "assignments",
                path: "case.ts",
                code: "let a: unknown;\na = { id: 1 };\na += 1;\nlet b: number;\nb = 1;\nundeclared = 1;\nfunction g() {}\ng = () => {};\n",
                expected: &[(20, 29, "The explicit unknown type on binding `a` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.")],
            },
            Case {
                name: "returns",
                path: "case.ts",
                code: "export function a(): unknown {\n\treturn { id: 1 };\n}\nexport const b = function (): object {\n\treturn [1];\n};\nexport const c = (): unknown => {\n\treturn 1;\n};\nexport const d = function named(): unknown {\n\treturn 1;\n};\nexport class E {\n\tm(): unknown {\n\t\treturn 1;\n\t}\n\t\"s\"(): unknown {\n\t\treturn 2;\n\t}\n\tf = (): unknown => {\n\t\treturn 3;\n\t};\n}\nexport default function (): unknown {\n\treturn 4;\n}\nexport const o = {\n\tm(): unknown {\n\t\treturn 5;\n\t},\n};\n",
                expected: &[(39, 48, "The explicit unknown type on return value of `a` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract."), (99, 102, "The explicit object type on return value of `b` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract."), (149, 150, "The explicit unknown type on return value of `c` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract."), (208, 209, "The explicit unknown type on return value of `named` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract."), (256, 257, "The explicit unknown type on return value of `m` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract."), (289, 290, "The explicit unknown type on return value of `s` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract."), (326, 327, "The explicit unknown type on return value of `anonymous function` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract."), (381, 382, "The explicit unknown type on return value of `anonymous function` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract."), (430, 431, "The explicit unknown type on return value of `anonymous function` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.")],
            },
            Case {
                name: "expression arrow bodies",
                path: "case.ts",
                code: "export const a = (): unknown => ({ id: 1 });\nexport const b = ((): object => [1]);\nexport const c = (): unknown => read();\ndeclare function read(): number;\nexport const d = [1].map((): unknown => 1);\n",
                expected: &[(33, 42, "The explicit unknown type on return value of `a` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract."), (77, 80, "The explicit object type on return value of `b` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract."), (196, 197, "The explicit unknown type on return value of `anonymous function` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.")],
            },
            Case {
                name: "assertions",
                path: "case.ts",
                code: "export const a = { id: 1 } as unknown;\nexport const b = <object>[1];\nexport const c = ({ id: 1 } as unknown) as object;\nexport const d = { id: 1 } as Record<string, unknown>;\nexport const e = {} as Record<string, unknown>;\n",
                expected: &[(17, 26, "The explicit unknown type on assertion discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract."), (64, 67, "The explicit object type on assertion discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract."), (87, 107, "The explicit object type on assertion discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract."), (137, 146, "The explicit open dictionary type on assertion discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.")],
            },
            Case {
                name: "return outside function",
                path: "case.ts",
                code: "export {};\n",
                expected: &[],
            },
            Case {
                name: "nested function names",
                path: "case.ts",
                code: "export function outer(): void {\n\tconst inner = (): unknown => {\n\t\tfunction deep(): object {\n\t\t\treturn [1];\n\t\t}\n\t\treturn deep;\n\t};\n\tinner();\n}\n",
                expected: &[(102, 105, "The explicit object type on return value of `deep` discards known type evidence. Keep inference, validate with `satisfies`, or use a named owner contract.")],
            },
        ],
    );
}
