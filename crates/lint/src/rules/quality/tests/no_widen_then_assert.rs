//! `quality/no-widen-then-assert`: expectations recorded from `@nkzw/oxlint-config` 3.0.0's
//! `quality` plugin under oxlint 1.94.0: byte spans and exact messages.
//!
//! The first tables replay `anti-slop/no-widen-then-assert`'s cases
//! (`anti_slop/tests/no_widen_then_assert.rs`, same names) with the findings the
//! upstream plugin produced for them.
//! `type_bindings` is upstream's `test/type-bindings.test.js`, one case per
//! file with that file's findings for this rule.
//! `differential` holds fmtkit's own probes of how the plugin resolves
//! types, recorded the same way.

use super::{Case, check};

/// tests/no-widen-then-assert.test.ts
#[test]
fn v1() {
    check(
        "quality/no-widen-then-assert",
        "[]",
        &[
            Case {
                name: "precise type survives",
                path: "case.ts",
                code: "export function read(): string {\n\tconst owner = { id: \"a\" };\n\n\treturn owner.id;\n}\n",
                expected: &[],
            },
            Case {
                name: "named contract, no assertion",
                path: "case.ts",
                code: "type Owner = { readonly id: string };\n\nexport function read(): Owner {\n\tconst owner: Owner = { id: \"a\" };\n\n\treturn owner;\n}\n",
                expected: &[],
            },
            Case {
                name: "widened to unknown then asserted back",
                path: "case.ts",
                code: "type Owner = { readonly id: string };\n\nexport function read(): Owner {\n\tconst owner: unknown = { id: \"a\" };\n\n\treturn owner as Owner;\n}\n",
                expected: &[(
                    117,
                    131,
                    "Binding \"owner\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.",
                )],
            },
            Case {
                name: "widened at module scope",
                path: "case.ts",
                code: "type Owner = { readonly id: string };\n\nconst owner: unknown = { id: \"a\" };\n\nexport const found = owner as Owner;\n",
                expected: &[(
                    97,
                    111,
                    "Binding \"owner\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.",
                )],
            },
        ],
    );
}

/// tests/no-widen-then-assert.test.ts: open-record and object widening belong to the neighbours.
#[test]
fn v1_neighbours() {
    check(
        "quality/no-widen-then-assert",
        "[]",
        &[
            Case {
                name: "open record",
                path: "case.ts",
                code: "type Owner = { readonly id: string };\n\nexport function read(): Owner {\n\tconst owner: Record<string, unknown> = { id: \"a\" };\n\n\treturn owner as Owner;\n}\n",
                expected: &[],
            },
            Case {
                name: "object",
                path: "case.ts",
                code: "type Owner = { readonly id: string };\n\nexport function read(): Owner {\n\tconst owner: object = { id: \"a\" };\n\n\treturn owner as Owner;\n}\n",
                expected: &[],
            },
        ],
    );
}

/// `anti-slop/no-widen-then-assert`'s `broad_kinds` table.
#[test]
fn broad_kinds() {
    check(
        "quality/no-widen-then-assert",
        "[]",
        &[
            Case {
                name: "any and angle brackets",
                path: "case.ts",
                code: "const a: any = 1;\nexport const b = <number>a;\nexport const c = (a) as string;\n",
                expected: &[
                    (
                        35,
                        44,
                        "Binding \"a\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.",
                    ),
                    (
                        63,
                        76,
                        "Binding \"a\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.",
                    ),
                ],
            },
            Case {
                name: "widened through an initializer assertion",
                path: "case.ts",
                code: "const a = { id: 1 } as unknown;\nexport const b = a as { id: number };\nconst c = ({ id: 1 }) as object;\nexport const d = c as { id: number };\nconst e = [1] as Record<string, unknown>;\nexport const f = e as Record<string, number>;\n",
                expected: &[
                    (
                        49,
                        68,
                        "Binding \"a\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.",
                    ),
                    (
                        120,
                        139,
                        "Binding \"c\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.",
                    ),
                    (
                        200,
                        227,
                        "Binding \"e\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.",
                    ),
                ],
            },
            Case {
                name: "object kind needs a definitely object assertion",
                path: "case.ts",
                code: "const a: object = { id: 1 };\nexport const b = a as string;\nexport const c = a as number[];\nexport const d = a as () => void;\nexport const e = a as new () => void;\nexport const f = a as { [K in string]: 1 };\nexport const g = a as [number];\nexport const h = a as {};\nexport const i = a as { id: number } & { b: 1 };\nexport const j = a as readonly number[];\nexport const k = a as object;\nexport const l = a as unknown;\n",
                expected: &[
                    (
                        76,
                        89,
                        "Binding \"a\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.",
                    ),
                    (
                        108,
                        123,
                        "Binding \"a\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.",
                    ),
                    (
                        142,
                        161,
                        "Binding \"a\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.",
                    ),
                    (
                        180,
                        205,
                        "Binding \"a\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.",
                    ),
                    (
                        224,
                        237,
                        "Binding \"a\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.",
                    ),
                    (
                        282,
                        312,
                        "Binding \"a\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.",
                    ),
                    (
                        331,
                        353,
                        "Binding \"a\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.",
                    ),
                ],
            },
            Case {
                name: "record kind needs a narrower record",
                path: "case.ts",
                code: "const a: Record<string, unknown> = { id: 1 };\nexport const b = a as { id: number };\nexport const c = a as Record<string, number>;\nexport const d = a as Readonly<Record<string, number>>;\nexport const e = a as { [key: string]: number };\nexport const f = a as Record<string, any>;\nexport const g = a as Map<string, number>;\nexport const h = a as Record<string>;\n",
                expected: &[
                    (
                        63,
                        82,
                        "Binding \"a\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.",
                    ),
                    (
                        101,
                        128,
                        "Binding \"a\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.",
                    ),
                    (
                        147,
                        184,
                        "Binding \"a\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.",
                    ),
                ],
            },
            Case {
                name: "record forms",
                path: "case.ts",
                code: "const a: Readonly<Record<PropertyKey, any>> = {};\nexport const b = a as Record<string, number>;\nconst c: { [key: string | number]: unknown } = {};\nexport const d = a as Record<string, number>;\nconst e: { [key: string]: unknown; b: 1 } = { b: 1 };\nexport const f = e as Record<string, number>;\nconst g: Record<`x${string}`, unknown> = {};\nexport const h = g as Record<string, number>;\n",
                expected: &[
                    (
                        67,
                        94,
                        "Binding \"a\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.",
                    ),
                    (
                        164,
                        191,
                        "Binding \"a\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.",
                    ),
                ],
            },
            Case {
                name: "same syntax as the evidence",
                path: "case.ts",
                code: "type Box = { id: number };\nconst a: object = { id: 1 } as Box;\nexport const b = a as Box;\nconst c: Record<string, unknown> = { id: 1 } as Box;\nexport const d = c as  Box;\nconst e: object = 1 as (Box);\nexport const f = e as Box;\n",
                expected: &[
                    (
                        80,
                        88,
                        "Binding \"a\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.",
                    ),
                    (
                        160,
                        169,
                        "Binding \"c\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.",
                    ),
                    (
                        218,
                        226,
                        "Binding \"e\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.",
                    ),
                ],
            },
        ],
    );
}

/// `anti-slop/no-widen-then-assert`'s `evidence` table.
#[test]
fn evidence() {
    check(
        "quality/no-widen-then-assert",
        "[]",
        &[
            Case {
                name: "evidence kinds",
                path: "case.ts",
                code: "const a: unknown = \"x\";\nexport const b = a as string;\nconst c: unknown = `x`;\nexport const d = c as string;\nconst e: unknown = new Map();\nexport const f = e as Map<string, number>;\nconst g: unknown = () => 1;\nexport const h = g as () => number;\nconst i: unknown = read();\nexport const j = i as number;\ndeclare function read(): number;\nconst k: unknown = -1;\nexport const l = k as number;\n",
                expected: &[
                    (
                        41,
                        52,
                        "Binding \"a\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.",
                    ),
                    (
                        95,
                        106,
                        "Binding \"c\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.",
                    ),
                    (
                        155,
                        179,
                        "Binding \"e\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.",
                    ),
                    (
                        226,
                        243,
                        "Binding \"g\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.",
                    ),
                ],
            },
            Case {
                name: "evidence through bindings",
                path: "case.ts",
                code: "const base = { id: 1 };\nconst a: unknown = base;\nexport const b = a as { id: number };\nlet loose = { id: 1 };\nconst c: unknown = loose;\nexport const d = c as { id: number };\nconst typed: { id: number } = { id: 1 };\nconst e: unknown = typed;\nexport const f = e as { id: number };\nconst broad: unknown = { id: 1 };\nconst g: unknown = broad;\nexport const h = g as { id: number };\n",
                expected: &[
                    (
                        66,
                        85,
                        "Binding \"a\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.",
                    ),
                    (
                        258,
                        277,
                        "Binding \"e\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.",
                    ),
                ],
            },
            Case {
                name: "parameters as evidence",
                path: "case.ts",
                code: "export function f(x: { id: number }, y: unknown, ...z: number[]): void {\n\tconst a: unknown = x;\n\tconst b: unknown = y;\n\tconst c: unknown = z;\n\tconsole.log(a as { id: number }, b as number, c as number[]);\n}\n",
                expected: &[(
                    155,
                    174,
                    "Binding \"a\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.",
                )],
            },
            Case {
                name: "evidence across function boundaries",
                path: "case.ts",
                code: "const outer = { id: 1 };\nexport function f(): void {\n\tconst a: unknown = outer;\n\tconsole.log(a as { id: number });\n}\nconst b: unknown = { id: 1 };\nexport function g(): unknown {\n\treturn b as { id: number };\n}\nexport const h = (): unknown => b as { id: number };\n",
                expected: &[],
            },
            Case {
                name: "assertion before declaration end",
                path: "case.ts",
                code: "const a: unknown = { id: 1 };\nconst b: unknown = (b as never);\nexport { a };\n",
                expected: &[],
            },
            Case {
                name: "binding is reassigned or not const",
                path: "case.ts",
                code: "let a: unknown = { id: 1 };\nexport const b = a as { id: number };\nvar c: unknown = { id: 1 };\nexport const d = c as { id: number };\n",
                expected: &[],
            },
            Case {
                name: "destructured and missing init",
                path: "case.ts",
                code: "const { a }: { a: unknown } = { a: 1 };\nexport const b = a as number;\ndeclare const c: unknown;\nexport const d = c as number;\n",
                expected: &[],
            },
            Case {
                name: "catch parameter",
                path: "case.ts",
                code: "try {\n\tvoid 0;\n} catch (error: unknown) {\n\tconst a: unknown = error;\n\tconsole.log(a as Error);\n}\n",
                expected: &[],
            },
            Case {
                name: "broad assertion is not narrowing",
                path: "case.ts",
                code: "const a: unknown = { id: 1 };\nexport const b = a as any;\nexport const c = a as Record<string, unknown>;\n",
                expected: &[],
            },
            Case { name: "self referential evidence", path: "case.ts", code: "const a: unknown = a;\nexport const b = a as number;\n", expected: &[] },
        ],
    );
}

/// test/type-bindings.test.js: forward declarations, local predicates and
/// shadows resolve independently in each file.
#[test]
fn type_bindings() {
    check(
        "quality/no-widen-then-assert",
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
                expected: &[(
                    254,
                    267,
                    "Binding \"raw\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.",
                )],
            },
        ],
    );
}

/// Differential probes: generic aliases, unions, nesting, shadowed
/// built-ins and parameter forms.
#[test]
fn differential() {
    check(
        "quality/no-widen-then-assert",
        "[]",
        &[
            Case {
                name: "basic forms",
                path: "case.ts",
                code: "const a: unknown = { x: 1 };\nexport const b = a as { x: number };\nconst c = 'x' as unknown;\nexport const d = c as string;\n",
                expected: &[
                    (
                        46,
                        64,
                        "Binding \"a\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.",
                    ),
                    (
                        109,
                        120,
                        "Binding \"c\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.",
                    ),
                ],
            },
            Case {
                name: "record and object",
                path: "case.ts",
                code: "const r: Record<string, unknown> = { a: 1 };\nexport const s = r as { a: number };\nconst o: object = [1];\nexport const p = o as number[];\n",
                expected: &[
                    (
                        62,
                        80,
                        "Binding \"r\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.",
                    ),
                    (
                        122,
                        135,
                        "Binding \"o\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.",
                    ),
                ],
            },
            Case {
                name: "property key alias",
                path: "case.ts",
                code: "type PropertyKey = 'a';\nconst r: Record<PropertyKey, unknown> = { a: 1 };\nexport const s = r as { a: number };\n",
                expected: &[(
                    91,
                    109,
                    "Binding \"r\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.",
                )],
            },
            Case { name: "function boundary", path: "case.ts", code: "const a: unknown = 1;\nfunction f() { return a as number; }\n", expected: &[] },
            Case {
                name: "readonly record forms",
                path: "case.ts",
                code: "const r: Readonly<Record<string, unknown>> = { a: 1 };\nexport const s = r as Record<string, number>;\nconst t: { [k: string]: any } = { a: 1 };\nexport const u = t as { a: number };\n",
                expected: &[
                    (
                        72,
                        99,
                        "Binding \"r\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.",
                    ),
                    (
                        160,
                        178,
                        "Binding \"t\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.",
                    ),
                ],
            },
            Case {
                name: "evidence same syntax",
                path: "case.ts",
                code: "const v = { a: 1 } as { a: number };\nconst w: unknown = v;\nexport const x = w as { a : number };\nconst o: object = v;\nexport const p = o as  { a: number };\n",
                expected: &[
                    (
                        76,
                        95,
                        "Binding \"w\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.",
                    ),
                    (
                        135,
                        154,
                        "Binding \"o\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.",
                    ),
                ],
            },
            Case {
                name: "nested function boundary",
                path: "case.ts",
                code: "function f() {\n\tconst a: unknown = 1;\n\treturn () => a as number;\n}\nfunction g() {\n\tconst a: unknown = 1;\n\treturn a as number;\n}\n",
                expected: &[(
                    113,
                    124,
                    "Binding \"a\" discards type evidence and later recreates it with an assertion. Keep the precise type from initialization through use; parse boundary input once.",
                )],
            },
            Case { name: "let is ignored", path: "case.ts", code: "let a: unknown = 1;\nexport const b = a as number;\n", expected: &[] },
        ],
    );
}
