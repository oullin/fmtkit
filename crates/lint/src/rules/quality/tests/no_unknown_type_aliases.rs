//! `quality/no-unknown-type-aliases`: expectations recorded from `@nkzw/oxlint-config` 3.0.0's
//! `quality` plugin under oxlint 1.94.0: byte spans and exact messages.
//!
//! The first tables replay `anti-slop/no-unknown-type-aliases`'s cases
//! (`anti_slop/tests/no_unknown_type_aliases.rs`, same names) with the findings the
//! upstream plugin produced for them.
//! `type_bindings` is upstream's `test/type-bindings.test.js`, one case per
//! file with that file's findings for this rule.
//! `differential` holds fmtkit's own probes of how the plugin resolves
//! types, recorded the same way.

use super::{Case, check};

/// tests/no-unknown-type-aliases.test.ts
#[test]
fn v1() {
    check(
        "quality/no-unknown-type-aliases",
        "[]",
        &[
            Case { name: "domain alias", path: "case.ts", code: "export type Payload = { readonly id: string };\n", expected: &[] },
            Case {
                name: "alias of unknown",
                path: "case.ts",
                code: "export type Payload = unknown;\n",
                expected: &[(
                    12,
                    19,
                    "Type alias `Payload` hides `unknown`. Keep `unknown` explicit at the parsing boundary or on an allowed `cause` field; otherwise use the parsed owner type.",
                )],
            },
            Case {
                name: "alias of an unknown alias",
                path: "case.ts",
                code: "type Raw = unknown;\n\nexport type Payload = Raw;\n",
                expected: &[
                    (
                        5,
                        8,
                        "Type alias `Raw` hides `unknown`. Keep `unknown` explicit at the parsing boundary or on an allowed `cause` field; otherwise use the parsed owner type.",
                    ),
                    (
                        33,
                        40,
                        "Type alias `Payload` hides `unknown`. Keep `unknown` explicit at the parsing boundary or on an allowed `cause` field; otherwise use the parsed owner type.",
                    ),
                ],
            },
        ],
    );
}

/// `anti-slop/no-unknown-type-aliases`'s `edges` table.
#[test]
fn edges() {
    check(
        "quality/no-unknown-type-aliases",
        "[]",
        &[
            Case {
                name: "parentheses and chains",
                path: "case.ts",
                code: "type A = (unknown);\ntype B = (A);\ntype C = B;\n",
                expected: &[
                    (
                        5,
                        6,
                        "Type alias `A` hides `unknown`. Keep `unknown` explicit at the parsing boundary or on an allowed `cause` field; otherwise use the parsed owner type.",
                    ),
                    (
                        25,
                        26,
                        "Type alias `B` hides `unknown`. Keep `unknown` explicit at the parsing boundary or on an allowed `cause` field; otherwise use the parsed owner type.",
                    ),
                    (
                        39,
                        40,
                        "Type alias `C` hides `unknown`. Keep `unknown` explicit at the parsing boundary or on an allowed `cause` field; otherwise use the parsed owner type.",
                    ),
                ],
            },
            Case {
                name: "unions and generics are not followed",
                path: "case.ts",
                code: "type A = unknown | string;\ntype G<T> = unknown;\ntype B = G<number>;\ntype C = G;\n",
                expected: &[
                    (
                        5,
                        6,
                        "Type alias `A` hides `unknown`. Keep `unknown` explicit at the parsing boundary or on an allowed `cause` field; otherwise use the parsed owner type.",
                    ),
                    (
                        32,
                        33,
                        "Type alias `G` hides `unknown`. Keep `unknown` explicit at the parsing boundary or on an allowed `cause` field; otherwise use the parsed owner type.",
                    ),
                    (
                        53,
                        54,
                        "Type alias `B` hides `unknown`. Keep `unknown` explicit at the parsing boundary or on an allowed `cause` field; otherwise use the parsed owner type.",
                    ),
                ],
            },
            Case { name: "cycles", path: "case.ts", code: "type A = B;\ntype B = A;\ntype S = S;\n", expected: &[] },
            Case {
                name: "nested aliases are ignored",
                path: "case.ts",
                code: "declare namespace NS {\n\ttype A = unknown;\n}\nexport function f(): void {\n\ttype B = unknown;\n}\n",
                expected: &[
                    (
                        29,
                        30,
                        "Type alias `A` hides `unknown`. Keep `unknown` explicit at the parsing boundary or on an allowed `cause` field; otherwise use the parsed owner type.",
                    ),
                    (
                        78,
                        79,
                        "Type alias `B` hides `unknown`. Keep `unknown` explicit at the parsing boundary or on an allowed `cause` field; otherwise use the parsed owner type.",
                    ),
                ],
            },
            Case { name: "default exports are not unwrapped", path: "case.ts", code: "export default interface I {}\ntype A = I;\n", expected: &[] },
        ],
    );
}

/// test/type-bindings.test.js: forward declarations, local predicates and
/// shadows resolve independently in each file.
#[test]
fn type_bindings() {
    check(
        "quality/no-unknown-type-aliases",
        "[]",
        &[
            Case {
                name: "broad.ts",
                path: "broad.ts",
                code: "export function accept(value: Input): Result { return value; }\nexport type Bag = Record<string, Input>;\nexport function local<Input, Result>(value: Input): Result { throw value; }\ntype Input = object;\ntype Result = unknown;",
                expected: &[(
                    206,
                    212,
                    "Type alias `Result` hides `unknown`. Keep `unknown` explicit at the parsing boundary or on an allowed `cause` field; otherwise use the parsed owner type.",
                )],
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
        "quality/no-unknown-type-aliases",
        "[]",
        &[
            Case {
                name: "union containing unknown",
                path: "case.ts",
                code: "type A = string | unknown;\ntype B = (unknown) | number;\ntype C = string | number;\n",
                expected: &[
                    (
                        5,
                        6,
                        "Type alias `A` hides `unknown`. Keep `unknown` explicit at the parsing boundary or on an allowed `cause` field; otherwise use the parsed owner type.",
                    ),
                    (
                        32,
                        33,
                        "Type alias `B` hides `unknown`. Keep `unknown` explicit at the parsing boundary or on an allowed `cause` field; otherwise use the parsed owner type.",
                    ),
                ],
            },
            Case {
                name: "generic instantiation",
                path: "case.ts",
                code: "type G<T> = T;\ntype A = G<unknown>;\ntype B = G<string>;\ntype C<T> = unknown;\n",
                expected: &[
                    (
                        20,
                        21,
                        "Type alias `A` hides `unknown`. Keep `unknown` explicit at the parsing boundary or on an allowed `cause` field; otherwise use the parsed owner type.",
                    ),
                    (
                        61,
                        62,
                        "Type alias `C` hides `unknown`. Keep `unknown` explicit at the parsing boundary or on an allowed `cause` field; otherwise use the parsed owner type.",
                    ),
                ],
            },
            Case {
                name: "generic alias itself",
                path: "case.ts",
                code: "type C<T> = unknown;\ntype D<T> = T;\ntype E<T = unknown> = T;\n",
                expected: &[(
                    5,
                    6,
                    "Type alias `C` hides `unknown`. Keep `unknown` explicit at the parsing boundary or on an allowed `cause` field; otherwise use the parsed owner type.",
                )],
            },
            Case {
                name: "nested",
                path: "case.ts",
                code: "function f() {\n\ttype A = unknown;\n\t{\n\t\ttype B = A;\n\t}\n}\nnamespace N {\n\texport type C = unknown;\n}\n",
                expected: &[
                    (
                        21,
                        22,
                        "Type alias `A` hides `unknown`. Keep `unknown` explicit at the parsing boundary or on an allowed `cause` field; otherwise use the parsed owner type.",
                    ),
                    (
                        44,
                        45,
                        "Type alias `B` hides `unknown`. Keep `unknown` explicit at the parsing boundary or on an allowed `cause` field; otherwise use the parsed owner type.",
                    ),
                    (
                        83,
                        84,
                        "Type alias `C` hides `unknown`. Keep `unknown` explicit at the parsing boundary or on an allowed `cause` field; otherwise use the parsed owner type.",
                    ),
                ],
            },
            Case {
                name: "exported and default",
                path: "case.ts",
                code: "export type A = unknown;\nexport type B = A | string;\n",
                expected: &[
                    (
                        12,
                        13,
                        "Type alias `A` hides `unknown`. Keep `unknown` explicit at the parsing boundary or on an allowed `cause` field; otherwise use the parsed owner type.",
                    ),
                    (
                        37,
                        38,
                        "Type alias `B` hides `unknown`. Keep `unknown` explicit at the parsing boundary or on an allowed `cause` field; otherwise use the parsed owner type.",
                    ),
                ],
            },
            Case {
                name: "cycle",
                path: "case.ts",
                code: "type A = B;\ntype B = A;\ntype C = C | unknown;\n",
                expected: &[(
                    29,
                    30,
                    "Type alias `C` hides `unknown`. Keep `unknown` explicit at the parsing boundary or on an allowed `cause` field; otherwise use the parsed owner type.",
                )],
            },
            Case { name: "intersection is not union", path: "case.ts", code: "type A = unknown & string;\ntype B = Promise<unknown>;\n", expected: &[] },
            Case {
                name: "reference to type parameter",
                path: "case.ts",
                code: "type F<unknownish> = unknownish;\ntype U = unknown;\ntype G<U> = U;\n",
                expected: &[(
                    38,
                    39,
                    "Type alias `U` hides `unknown`. Keep `unknown` explicit at the parsing boundary or on an allowed `cause` field; otherwise use the parsed owner type.",
                )],
            },
            Case {
                name: "shadowed by interface",
                path: "case.ts",
                code: "type U = unknown;\nfunction f() {\n\tinterface U { a: 1 }\n\ttype V = U;\n}\n",
                expected: &[(
                    5,
                    6,
                    "Type alias `U` hides `unknown`. Keep `unknown` explicit at the parsing boundary or on an allowed `cause` field; otherwise use the parsed owner type.",
                )],
            },
            Case {
                name: "declare module",
                path: "case.ts",
                code: "declare module 'x' {\n\ttype A = unknown;\n}\ndeclare global {\n\ttype B = unknown;\n}\n",
                expected: &[
                    (
                        27,
                        28,
                        "Type alias `A` hides `unknown`. Keep `unknown` explicit at the parsing boundary or on an allowed `cause` field; otherwise use the parsed owner type.",
                    ),
                    (
                        65,
                        66,
                        "Type alias `B` hides `unknown`. Keep `unknown` explicit at the parsing boundary or on an allowed `cause` field; otherwise use the parsed owner type.",
                    ),
                ],
            },
            Case {
                name: "generic default instantiation",
                path: "case.ts",
                code: "type D<T = unknown> = T;\ntype A = D;\ntype B = D<string>;\n",
                expected: &[(
                    30,
                    31,
                    "Type alias `A` hides `unknown`. Keep `unknown` explicit at the parsing boundary or on an allowed `cause` field; otherwise use the parsed owner type.",
                )],
            },
            Case {
                name: "generic substitution leak",
                path: "case.ts",
                code: "type Outer<T> = Inner;\ntype Inner = T;\ntype A = Outer<unknown>;\n",
                expected: &[(
                    44,
                    45,
                    "Type alias `A` hides `unknown`. Keep `unknown` explicit at the parsing boundary or on an allowed `cause` field; otherwise use the parsed owner type.",
                )],
            },
            Case {
                name: "parenthesized union",
                path: "case.ts",
                code: "type A = (string | (unknown));\ntype B = Array<unknown>;\ntype C = unknown[];\n",
                expected: &[(
                    5,
                    6,
                    "Type alias `A` hides `unknown`. Keep `unknown` explicit at the parsing boundary or on an allowed `cause` field; otherwise use the parsed owner type.",
                )],
            },
            Case {
                name: "alias inside class static block",
                path: "case.ts",
                code: "class C {\n\tstatic {\n\t\ttype A = unknown;\n\t}\n}\n",
                expected: &[(
                    27,
                    28,
                    "Type alias `A` hides `unknown`. Keep `unknown` explicit at the parsing boundary or on an allowed `cause` field; otherwise use the parsed owner type.",
                )],
            },
            Case {
                name: "alias in arrow body",
                path: "case.ts",
                code: "const f = () => {\n\ttype A = unknown;\n\ttype B = A | string;\n};\n",
                expected: &[
                    (
                        24,
                        25,
                        "Type alias `A` hides `unknown`. Keep `unknown` explicit at the parsing boundary or on an allowed `cause` field; otherwise use the parsed owner type.",
                    ),
                    (
                        43,
                        44,
                        "Type alias `B` hides `unknown`. Keep `unknown` explicit at the parsing boundary or on an allowed `cause` field; otherwise use the parsed owner type.",
                    ),
                ],
            },
            Case {
                name: "alias referencing import",
                path: "case.ts",
                code: "import type { X } from './x';\ntype A = X;\ntype B = unknown | X;\n",
                expected: &[(
                    47,
                    48,
                    "Type alias `B` hides `unknown`. Keep `unknown` explicit at the parsing boundary or on an allowed `cause` field; otherwise use the parsed owner type.",
                )],
            },
            Case { name: "alias of interface", path: "case.ts", code: "interface I {}\ntype A = I;\ntype unknownLike = I;\n", expected: &[] },
            Case {
                name: "alias shadowed in nested block",
                path: "case.ts",
                code: "type U = unknown;\nfunction f() {\n\ttype U = string;\n\ttype V = U;\n}\ntype W = U;\n",
                expected: &[
                    (
                        5,
                        6,
                        "Type alias `U` hides `unknown`. Keep `unknown` explicit at the parsing boundary or on an allowed `cause` field; otherwise use the parsed owner type.",
                    ),
                    (
                        71,
                        72,
                        "Type alias `W` hides `unknown`. Keep `unknown` explicit at the parsing boundary or on an allowed `cause` field; otherwise use the parsed owner type.",
                    ),
                ],
            },
            Case { name: "keyof and typeof", path: "case.ts", code: "type A = keyof unknown;\nconst x: unknown = 1;\ntype B = typeof x;\n", expected: &[] },
            Case {
                name: "self generic",
                path: "case.ts",
                code: "type L<T> = L<T> | T;\ntype A = L<unknown>;\n",
                expected: &[(
                    27,
                    28,
                    "Type alias `A` hides `unknown`. Keep `unknown` explicit at the parsing boundary or on an allowed `cause` field; otherwise use the parsed owner type.",
                )],
            },
            Case {
                name: "parenthesized generic",
                path: "case.ts",
                code: "type G<T> = ((T));\ntype A = (G<(unknown)>);\n",
                expected: &[(
                    24,
                    25,
                    "Type alias `A` hides `unknown`. Keep `unknown` explicit at the parsing boundary or on an allowed `cause` field; otherwise use the parsed owner type.",
                )],
            },
        ],
    );
}
