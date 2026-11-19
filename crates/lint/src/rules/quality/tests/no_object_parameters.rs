//! `quality/no-object-parameters`: expectations recorded from `@nkzw/oxlint-config` 3.0.0's
//! `quality` plugin under oxlint 1.94.0: byte spans and exact messages.
//!
//! The first tables replay `anti-slop/no-object-parameters`'s cases
//! (`anti_slop/tests/no_object_parameters.rs`, same names) with the findings the
//! upstream plugin produced for them.
//! `type_bindings` is upstream's `test/type-bindings.test.js`, one case per
//! file with that file's findings for this rule.
//! `differential` holds fmtkit's own probes of how the plugin resolves
//! types, recorded the same way.

use super::{Case, check};

/// tests/no-object-parameters.test.ts
#[test]
fn v1() {
    check(
        "quality/no-object-parameters",
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

/// `anti-slop/no-object-parameters`'s `owners` table.
#[test]
fn owners() {
    check(
        "quality/no-object-parameters",
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
                        "Parameter `x` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
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
                        "Parameter `d` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                    (
                        80,
                        86,
                        "Parameter `e` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                    (
                        94,
                        100,
                        "Parameter `c` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
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
                        "Parameter `{ a }` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
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
                        146,
                        155,
                        "Parameter `c` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                    (
                        160,
                        161,
                        "Parameter `d` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                    (
                        175,
                        184,
                        "Parameter `f` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
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

/// test/type-bindings.test.js: forward declarations, local predicates and
/// shadows resolve independently in each file.
#[test]
fn type_bindings() {
    check(
        "quality/no-object-parameters",
        "[]",
        &[
            Case {
                name: "broad.ts",
                path: "broad.ts",
                code: "export function accept(value: Input): Result { return value; }\nexport type Bag = Record<string, Input>;\nexport function local<Input, Result>(value: Input): Result { throw value; }\ntype Input = object;\ntype Result = unknown;",
                expected: &[(
                    30,
                    35,
                    "Parameter `value` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
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
                expected: &[(
                    161,
                    167,
                    "Parameter `input` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
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
        "quality/no-object-parameters",
        "[]",
        &[
            Case {
                name: "generic alias instantiation",
                path: "case.ts",
                code: "type G<T> = object;\nexport function f(a: G<string>, b: G): void {}\n",
                expected: &[(
                    41,
                    50,
                    "Parameter `a` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                )],
            },
            Case {
                name: "generic alias default",
                path: "case.ts",
                code: "type G<T = string> = object;\nexport function f(a: G, b: G<number>): void {}\n",
                expected: &[
                    (
                        50,
                        51,
                        "Parameter `a` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                    (
                        56,
                        65,
                        "Parameter `b` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                ],
            },
            Case {
                name: "generic alias passes argument through",
                path: "case.ts",
                code: "type Id<T> = T;\nexport function f(a: Id<object>, b: Id<string>, c: Id<Id<object>>): void {}\n",
                expected: &[(
                    37,
                    47,
                    "Parameter `a` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                )],
            },
            Case {
                name: "generic alias union",
                path: "case.ts",
                code: "type Maybe<T> = T | undefined;\nexport function f(a: Maybe<object>, b: Maybe<string>): void {}\n",
                expected: &[(
                    52,
                    65,
                    "Parameter `a` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                )],
            },
            Case {
                name: "alias with type args to plain alias",
                path: "case.ts",
                code: "type A = object;\nexport function f(a: A<number>, b: A<string, number>): void {}\n",
                expected: &[
                    (
                        38,
                        47,
                        "Parameter `a` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                    (
                        52,
                        69,
                        "Parameter `b` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                ],
            },
            Case {
                name: "type parameter shadows alias",
                path: "case.ts",
                code: "type O = object;\nexport function f<O>(a: O): void {}\nexport function g(a: O): void {}\n",
                expected: &[(
                    74,
                    75,
                    "Parameter `a` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                )],
            },
            Case {
                name: "type parameter passed as argument",
                path: "case.ts",
                code: "type Id<T> = T;\nexport function f<T>(a: Id<T>): void {}\n",
                expected: &[],
            },
            Case {
                name: "nested alias in function",
                path: "case.ts",
                code: "export function outer(): void {\n\ttype Local = object;\n\tfunction inner(a: Local): void {}\n}\n",
                expected: &[(
                    73,
                    78,
                    "Parameter `a` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                )],
            },
            Case {
                name: "nested alias in block shadows module alias",
                path: "case.ts",
                code: "type A = string;\n{\n\ttype A = object;\n\tfunction f(a: A): void {}\n}\nfunction g(a: A): void {}\n",
                expected: &[(
                    52,
                    53,
                    "Parameter `a` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                )],
            },
            Case {
                name: "alias in namespace",
                path: "case.ts",
                code: "declare namespace NS {\n\ttype A = object;\n\tfunction f(a: A): void;\n}\nexport function g(a: NS.A): void {}\n",
                expected: &[(
                    56,
                    57,
                    "Parameter `a` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                )],
            },
            Case {
                name: "interface shadows alias",
                path: "case.ts",
                code: "type A = object;\nfunction f(): void {\n\tinterface A { x: number }\n\tfunction g(a: A): void {}\n}\n",
                expected: &[],
            },
            Case {
                name: "forward alias declaration",
                path: "case.ts",
                code: "export function f(a: Later): void {}\ntype Later = object;\n",
                expected: &[(
                    21,
                    26,
                    "Parameter `a` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                )],
            },
            Case {
                name: "parameter text forms",
                path: "case.ts",
                code: "export function f({ a, b }: object, [c, d]: object, { e } = {} as object, ...[g]: object[]): void {}\n",
                expected: &[
                    (
                        28,
                        34,
                        "Parameter `{ a, b }` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                    (
                        44,
                        50,
                        "Parameter `[c, d]` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                ],
            },
            Case {
                name: "parameter destructured default",
                path: "case.ts",
                code: "export function f({ a }: object = {}, [b]: object = [], c: object = {}): void {}\n",
                expected: &[
                    (
                        25,
                        31,
                        "Parameter `{ a }` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                    (
                        43,
                        49,
                        "Parameter `[b]` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                    (
                        59,
                        65,
                        "Parameter `c` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                ],
            },
            Case {
                name: "parameter property forms",
                path: "case.ts",
                code: "export class C {\n\tconstructor(public a: object, protected readonly b: object = {}, private c?: object) {}\n}\n",
                expected: &[
                    (
                        40,
                        46,
                        "Parameter `a` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                    (
                        70,
                        76,
                        "Parameter `b` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                    (
                        95,
                        101,
                        "Parameter `c` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                ],
            },
            Case {
                name: "this parameter",
                path: "case.ts",
                code: "export function f(this: object): void {}\n",
                expected: &[(
                    24,
                    30,
                    "Parameter `this` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                )],
            },
            Case {
                name: "rest with object array is not object",
                path: "case.ts",
                code: "export function f(...a: object): void {}\n",
                expected: &[(
                    24,
                    30,
                    "Parameter `a` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                )],
            },
            Case {
                name: "object pattern with comments",
                path: "case.ts",
                code: "export function f({ a /* x */ }   :   object): void {}\n",
                expected: &[(
                    38,
                    44,
                    "Parameter `{ a /* x */ }` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                )],
            },
            Case {
                name: "mapped type key shadows",
                path: "case.ts",
                code: "type K = object;\nexport type M = { [K in 'a']: (a: K) => void };\n",
                expected: &[],
            },
            Case {
                name: "infer shadows",
                path: "case.ts",
                code: "type K = object;\nexport type M<T> = T extends Promise<infer K> ? (a: K) => void : (a: K) => void;\n",
                expected: &[(
                    86,
                    87,
                    "Parameter `a` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                )],
            },
            Case {
                name: "generic chain",
                path: "case.ts",
                code: "type A<T> = B<T>;\ntype B<U> = U | null;\nexport function f(a: A<object>, b: A<number>): void {}\n",
                expected: &[(
                    61,
                    70,
                    "Parameter `a` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                )],
            },
            Case { name: "self referential generic", path: "case.ts", code: "type L<T> = L<T>;\nexport function f(a: L<object>): void {}\n", expected: &[] },
            Case {
                name: "generic default references earlier param",
                path: "case.ts",
                code: "type P<A, B = A> = B;\nexport function f(a: P<object>, b: P<string>): void {}\n",
                expected: &[(
                    43,
                    52,
                    "Parameter `a` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                )],
            },
            Case { name: "missing required argument", path: "case.ts", code: "type P<A, B> = B;\nexport function f(a: P<object>): void {}\n", expected: &[] },
            Case {
                name: "class shadows alias",
                path: "case.ts",
                code: "type A = object;\nfunction f(): void {\n\tclass A {}\n\tfunction g(a: A): void {}\n}\n",
                expected: &[],
            },
            Case {
                name: "enum and import binding",
                path: "case.ts",
                code: "import type { A } from './a';\ntype B = object;\nexport function f(a: A, b: B): void {}\n",
                expected: &[(
                    74,
                    75,
                    "Parameter `b` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                )],
            },
            Case {
                name: "arrow params in nested scope",
                path: "case.ts",
                code: "const x = (a: Q): void => {};\nfunction h() {\n\ttype Q = object;\n\tconst y = (b: Q): void => {};\n}\n",
                expected: &[(
                    78,
                    79,
                    "Parameter `b` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                )],
            },
            Case {
                name: "switch scope alias",
                path: "case.ts",
                code: "function h(v: number) {\n\tswitch (v) {\n\t\tcase 1:\n\t\t\ttype S = object;\n\t\t\tconst f = (a: S): void => {};\n\t}\n}\n",
                expected: &[(
                    85,
                    86,
                    "Parameter `a` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                )],
            },
            Case {
                name: "static block alias",
                path: "case.ts",
                code: "class C {\n\tstatic {\n\t\ttype S = object;\n\t\tconst f = (a: S): void => {};\n\t}\n}\n",
                expected: &[(
                    55,
                    56,
                    "Parameter `a` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                )],
            },
            Case {
                name: "substitution leaks into nested alias",
                path: "case.ts",
                code: "type Outer<T> = Inner;\ntype Inner = T;\nexport function f(a: Outer<object>): void {}\n",
                expected: &[(
                    60,
                    73,
                    "Parameter `a` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                )],
            },
            Case {
                name: "outer type parameter in alias body",
                path: "case.ts",
                code: "export function f<T>(): void {\n\ttype A = T;\n\tfunction g(a: A): void {}\n}\n",
                expected: &[],
            },
            Case {
                name: "class type parameter hides alias",
                path: "case.ts",
                code: "type T = object;\nexport class C<T> {\n\tm(a: T): void {}\n}\nexport class D {\n\tm(a: T): void {}\n}\n",
                expected: &[(
                    80,
                    81,
                    "Parameter `a` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                )],
            },
            Case {
                name: "interface method signature generics",
                path: "case.ts",
                code: "type O = object;\nexport interface I {\n\tm<O>(a: O): void;\n\tn(a: O): void;\n\tnew (a: O): I;\n\t(a: O): void;\n}\n",
                expected: &[
                    (
                        63,
                        64,
                        "Parameter `a` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                    (
                        82,
                        83,
                        "Parameter `a` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                    (
                        94,
                        95,
                        "Parameter `a` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                ],
            },
            Case {
                name: "function type alias",
                path: "case.ts",
                code: "type O = object;\nexport type F = (a: O) => void;\nexport type G = new (a: O) => void;\nexport type H = <O>(a: O) => void;\n",
                expected: &[
                    (
                        37,
                        38,
                        "Parameter `a` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                    (
                        73,
                        74,
                        "Parameter `a` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                ],
            },
            Case {
                name: "default using another alias",
                path: "case.ts",
                code: "type O = object;\ntype D<T = O> = T;\nexport function f(a: D, b: D<string>): void {}\n",
                expected: &[(
                    57,
                    58,
                    "Parameter `a` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                )],
            },
            Case {
                name: "union of generic",
                path: "case.ts",
                code: "type M<T> = T | null;\nexport function f(a: M<string> | M<object>, b: (M<number>)): void {}\n",
                expected: &[(
                    43,
                    64,
                    "Parameter `a` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                )],
            },
            Case {
                name: "alias of alias generic",
                path: "case.ts",
                code: "type O = object;\ntype W<T> = T;\nexport function f(a: W<O>, b: W<W<O>>): void {}\n",
                expected: &[(
                    53,
                    57,
                    "Parameter `a` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                )],
            },
            Case {
                name: "generic alias shadowed by type parameter",
                path: "case.ts",
                code: "type W<T> = object;\nexport function f<W>(a: W): void {}\n",
                expected: &[],
            },
            Case {
                name: "too many type arguments",
                path: "case.ts",
                code: "type W<T> = T;\nexport function f(a: W<object, string>): void {}\n",
                expected: &[(
                    36,
                    53,
                    "Parameter `a` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                )],
            },
            Case {
                name: "namespace qualified generic",
                path: "case.ts",
                code: "namespace N {\n\texport type W<T> = T;\n}\nexport function f(a: N.W<object>): void {}\n",
                expected: &[],
            },
            Case {
                name: "nested namespaces",
                path: "case.ts",
                code: "namespace A {\n\ttype O = object;\n\texport namespace B {\n\t\texport function f(a: O): void {}\n\t}\n}\n",
                expected: &[(
                    77,
                    78,
                    "Parameter `a` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                )],
            },
            Case {
                name: "import shadows alias in function",
                path: "case.ts",
                code: "import type { O } from './o';\nfunction g() {\n\ttype O = object;\n\tconst f = (a: O): void => {};\n}\nexport function h(a: O): void {}\n",
                expected: &[(
                    78,
                    79,
                    "Parameter `a` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                )],
            },
            Case {
                name: "enum shadows",
                path: "case.ts",
                code: "type E = object;\nfunction g() {\n\tenum E { A }\n\tconst f = (a: E): void => {};\n}\n",
                expected: &[],
            },
            Case {
                name: "for loop block alias",
                path: "case.ts",
                code: "for (const x of []) {\n\ttype O = object;\n\tconst f = (a: O): void => {};\n}\n",
                expected: &[(
                    55,
                    56,
                    "Parameter `a` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                )],
            },
            Case {
                name: "catch block alias",
                path: "case.ts",
                code: "try {} catch {\n\ttype O = object;\n\tconst f = (a: O): void => {};\n}\n",
                expected: &[(
                    48,
                    49,
                    "Parameter `a` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                )],
            },
            Case { name: "class expression name", path: "case.ts", code: "type K = object;\nconst X = class K {\n\tm(a: K): void {}\n};\n", expected: &[] },
            Case {
                name: "arrow expression body class",
                path: "case.ts",
                code: "type K = object;\nconst f = () => class K {};\nexport function g(a: K): void {}\n",
                expected: &[],
            },
            Case {
                name: "parameter text with this and comments",
                path: "case.ts",
                code: "export function f(this: object, /* c */ { a }: object, [ b , c ]: object): void {}\n",
                expected: &[
                    (
                        24,
                        30,
                        "Parameter `this` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                    (
                        47,
                        53,
                        "Parameter `{ a }` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                    (
                        66,
                        72,
                        "Parameter `[ b , c ]` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                ],
            },
            Case {
                name: "optional pattern",
                path: "case.ts",
                code: "declare function f({ a }?: object, [b]?: object): void;\n",
                expected: &[
                    (
                        27,
                        33,
                        "Parameter `{ a }?` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                    (
                        41,
                        47,
                        "Parameter `[b]?` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                ],
            },
            Case {
                name: "parameter property destructure",
                path: "case.ts",
                code: "export class C {\n\tconstructor(readonly a: object, override b: object) {}\n}\nclass B { constructor(b: object) {} }\n",
                expected: &[
                    (
                        42,
                        48,
                        "Parameter `a` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                    (
                        62,
                        68,
                        "Parameter `b` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                    (
                        100,
                        106,
                        "Parameter `b` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                ],
            },
            Case {
                name: "union with object through alias param",
                path: "case.ts",
                code: "type U<T> = string | T;\nexport function f(a: U<object>): void {}\n",
                expected: &[(
                    45,
                    54,
                    "Parameter `a` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                )],
            },
            Case {
                name: "intersection is not object",
                path: "case.ts",
                code: "type W<T> = T & {};\nexport function f(a: W<object>, b: object & {}): void {}\n",
                expected: &[],
            },
            Case {
                name: "mapped type param name",
                path: "case.ts",
                code: "type O = object;\nexport type M = { [K in 'a']: (a: O) => K };\n",
                expected: &[(
                    51,
                    52,
                    "Parameter `a` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                )],
            },
            Case {
                name: "conditional infer",
                path: "case.ts",
                code: "type O = object;\nexport type C<T> = T extends Array<infer O> ? (a: O) => void : (a: O) => void;\n",
                expected: &[(
                    84,
                    85,
                    "Parameter `a` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                )],
            },
            Case {
                name: "parenthesized forms",
                path: "case.ts",
                code: "type A = (object);\ntype G<T> = (T);\nexport function f(a: (A), b: G<(object)>, c: ((G<object>) | string)): void {}\n",
                expected: &[
                    (
                        58,
                        59,
                        "Parameter `a` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                    (
                        65,
                        76,
                        "Parameter `b` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                    (
                        82,
                        102,
                        "Parameter `c` uses the broad `object` type. Accept a named owner type; parse external input at its boundary before calling this function.",
                    ),
                ],
            },
            Case {
                name: "readonly not unwrapped",
                path: "case.ts",
                code: "export function f(a: readonly object[], b: Readonly<object>): void {}\n",
                expected: &[],
            },
        ],
    );
}
