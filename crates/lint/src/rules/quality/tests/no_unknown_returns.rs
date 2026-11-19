//! `quality/no-unknown-returns`: expectations recorded from `@nkzw/oxlint-config` 3.0.0's
//! `quality` plugin under oxlint 1.94.0: byte spans and exact messages.
//!
//! The first tables replay `anti-slop/no-unknown-returns`'s cases
//! (`anti_slop/tests/no_unknown_returns.rs`, same names) with the findings the
//! upstream plugin produced for them.
//! `type_bindings` is upstream's `test/type-bindings.test.js`, one case per
//! file with that file's findings for this rule.
//! `differential` holds fmtkit's own probes of how the plugin resolves
//! types, recorded the same way.

use super::{Case, check};

/// tests/no-unknown-returns.test.ts
#[test]
fn v1() {
    check(
        "quality/no-unknown-returns",
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

/// `anti-slop/no-unknown-returns`'s `edges` table.
#[test]
fn edges() {
    check(
        "quality/no-unknown-returns",
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
                    (218, 229, "This function exposes `unknown` to its caller. Parse the value at its boundary and return a named domain type."),
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

/// test/type-bindings.test.js: forward declarations, local predicates and
/// shadows resolve independently in each file.
#[test]
fn type_bindings() {
    check(
        "quality/no-unknown-returns",
        "[]",
        &[
            Case {
                name: "broad.ts",
                path: "broad.ts",
                code: "export function accept(value: Input): Result { return value; }\nexport type Bag = Record<string, Input>;\nexport function local<Input, Result>(value: Input): Result { throw value; }\ntype Input = object;\ntype Result = unknown;",
                expected: &[(38, 44, "This function exposes `unknown` to its caller. Parse the value at its boundary and return a named domain type.")],
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
        "quality/no-unknown-returns",
        "[]",
        &[
            Case {
                name: "generic alias",
                path: "case.ts",
                code: "type G<T> = unknown;\nexport function a(): G<number> { return 1; }\nexport function b(): G { return 1; }\n",
                expected: &[(42, 51, "This function exposes `unknown` to its caller. Parse the value at its boundary and return a named domain type.")],
            },
            Case {
                name: "generic identity",
                path: "case.ts",
                code: "type Id<T> = T;\nexport function a(): Id<unknown> { return 1; }\nexport function b(): Id<string> { return ''; }\nexport function c(): Promise<Id<unknown>> { return Promise.resolve(1); }\n",
                expected: &[
                    (37, 48, "This function exposes `unknown` to its caller. Parse the value at its boundary and return a named domain type."),
                    (131, 151, "This function exposes `unknown` to its caller. Parse the value at its boundary and return a named domain type."),
                ],
            },
            Case {
                name: "generic default",
                path: "case.ts",
                code: "type D<T = unknown> = T;\nexport function a(): D { return 1; }\nexport function b(): D<string> { return ''; }\n",
                expected: &[(46, 47, "This function exposes `unknown` to its caller. Parse the value at its boundary and return a named domain type.")],
            },
            Case {
                name: "alias of promise",
                path: "case.ts",
                code: "type P<T> = Promise<T>;\nexport function a(): P<unknown> { return Promise.resolve(1); }\ntype Q = Promise<unknown>;\nexport function b(): Q { return Promise.resolve(1); }\n",
                expected: &[
                    (45, 55, "This function exposes `unknown` to its caller. Parse the value at its boundary and return a named domain type."),
                    (135, 136, "This function exposes `unknown` to its caller. Parse the value at its boundary and return a named domain type."),
                ],
            },
            Case {
                name: "type parameter shadows",
                path: "case.ts",
                code: "type U = unknown;\nexport function a<U>(): U { return 1 as U; }\nexport function b(): U { return 1; }\n",
                expected: &[(84, 85, "This function exposes `unknown` to its caller. Parse the value at its boundary and return a named domain type.")],
            },
            Case {
                name: "type param as argument",
                path: "case.ts",
                code: "type Id<T> = T;\nexport function a<T>(): Id<T> { return 1 as T; }\n",
                expected: &[],
            },
            Case {
                name: "nested alias",
                path: "case.ts",
                code: "export function outer(): void {\n\ttype L = unknown;\n\tfunction inner(): L { return 1; }\n}\n",
                expected: &[(70, 71, "This function exposes `unknown` to its caller. Parse the value at its boundary and return a named domain type.")],
            },
            Case {
                name: "namespace alias",
                path: "case.ts",
                code: "declare namespace NS {\n\ttype L = unknown;\n\tfunction f(): L;\n}\n",
                expected: &[(57, 58, "This function exposes `unknown` to its caller. Parse the value at its boundary and return a named domain type.")],
            },
            Case {
                name: "forward alias",
                path: "case.ts",
                code: "export function f(): Later { return 1; }\ntype Later = unknown;\n",
                expected: &[(21, 26, "This function exposes `unknown` to its caller. Parse the value at its boundary and return a named domain type.")],
            },
            Case {
                name: "interface shadows",
                path: "case.ts",
                code: "type L = unknown;\nfunction o() {\n\tinterface L { a: 1 }\n\tfunction f(): L { return { a: 1 }; }\n}\n",
                expected: &[],
            },
            Case {
                name: "promiselike",
                path: "case.ts",
                code: "type G<T> = PromiseLike<T>;\nexport function a(): G<unknown> { return Promise.resolve(1); }\n",
                expected: &[(49, 59, "This function exposes `unknown` to its caller. Parse the value at its boundary and return a named domain type.")],
            },
            Case {
                name: "union and parens",
                path: "case.ts",
                code: "type Id<T> = (T | string);\nexport function a(): Id<unknown> { return 1; }\n",
                expected: &[(48, 59, "This function exposes `unknown` to its caller. Parse the value at its boundary and return a named domain type.")],
            },
            Case { name: "mapped key shadows", path: "case.ts", code: "type K = unknown;\nexport type M = { [K in 'a']: () => K };\n", expected: &[] },
            Case {
                name: "promise shadowed by alias",
                path: "case.ts",
                code: "type Promise<T> = T;\nexport function a(): Promise<unknown> { return 1; }\nexport function b(): Promise<string> { return ''; }\n",
                expected: &[(42, 58, "This function exposes `unknown` to its caller. Parse the value at its boundary and return a named domain type.")],
            },
            Case { name: "missing argument", path: "case.ts", code: "type P<A, B> = B;\nexport function a(): P<unknown> { return 1; }\n", expected: &[] },
            Case {
                name: "method signature and class method",
                path: "case.ts",
                code: "type U = unknown;\nexport interface I {\n\tm(): U;\n\tn<U>(): U;\n}\nexport class C {\n\tm(): U { return 1; }\n\tget g(): U { return 1; }\n}\n",
                expected: &[
                    (45, 46, "This function exposes `unknown` to its caller. Parse the value at its boundary and return a named domain type."),
                    (85, 86, "This function exposes `unknown` to its caller. Parse the value at its boundary and return a named domain type."),
                    (111, 112, "This function exposes `unknown` to its caller. Parse the value at its boundary and return a named domain type."),
                ],
            },
            Case {
                name: "constructor and function types",
                path: "case.ts",
                code: "type U = unknown;\nexport type F = () => U;\nexport type G = new () => Promise<U>;\nexport type H = () => PromiseLike<(U)>;\n",
                expected: &[
                    (40, 41, "This function exposes `unknown` to its caller. Parse the value at its boundary and return a named domain type."),
                    (69, 79, "This function exposes `unknown` to its caller. Parse the value at its boundary and return a named domain type."),
                    (103, 119, "This function exposes `unknown` to its caller. Parse the value at its boundary and return a named domain type."),
                ],
            },
            Case {
                name: "conditional is not followed",
                path: "case.ts",
                code: "export type C<T> = T extends string ? unknown : unknown;\nexport function f(): C<string> { return 1; }\n",
                expected: &[],
            },
            Case {
                name: "promise of generic default",
                path: "case.ts",
                code: "type D<T = unknown> = Promise<T>;\nexport function f(): D { return Promise.resolve(1); }\nexport function g(): D<string> { return Promise.resolve(''); }\n",
                expected: &[(55, 56, "This function exposes `unknown` to its caller. Parse the value at its boundary and return a named domain type.")],
            },
            Case {
                name: "promise nested",
                path: "case.ts",
                code: "export function f(): Promise<Promise<unknown>> { return Promise.resolve(Promise.resolve(1)); }\nexport function g(): Promise<string | unknown> { return Promise.resolve(1); }\n",
                expected: &[
                    (21, 46, "This function exposes `unknown` to its caller. Parse the value at its boundary and return a named domain type."),
                    (116, 141, "This function exposes `unknown` to its caller. Parse the value at its boundary and return a named domain type."),
                ],
            },
            Case {
                name: "substitution leak",
                path: "case.ts",
                code: "type Outer<T> = Inner;\ntype Inner = T;\nexport function f(): Outer<unknown> { return 1; }\n",
                expected: &[(60, 74, "This function exposes `unknown` to its caller. Parse the value at its boundary and return a named domain type.")],
            },
            Case {
                name: "default referencing earlier param",
                path: "case.ts",
                code: "type P<A, B = A> = B;\nexport function f(): P<unknown> { return 1; }\nexport function g(): P<string, unknown> { return 1; }\n",
                expected: &[
                    (43, 53, "This function exposes `unknown` to its caller. Parse the value at its boundary and return a named domain type."),
                    (89, 107, "This function exposes `unknown` to its caller. Parse the value at its boundary and return a named domain type."),
                ],
            },
            Case {
                name: "arrow and function expression",
                path: "case.ts",
                code: "type U = unknown;\nexport const a = (): U => 1;\nexport const b = function (): U { return 1; };\nexport const c = <U,>(): U => 1 as U;\n",
                expected: &[
                    (39, 40, "This function exposes `unknown` to its caller. Parse the value at its boundary and return a named domain type."),
                    (77, 78, "This function exposes `unknown` to its caller. Parse the value at its boundary and return a named domain type."),
                ],
            },
            Case { name: "type predicate return", path: "case.ts", code: "export function f(x: unknown): x is unknown { return true; }\n", expected: &[] },
            Case {
                name: "nested function shadows",
                path: "case.ts",
                code: "type U = unknown;\nexport function f() {\n\ttype U = string;\n\treturn function g(): U { return ''; };\n}\n",
                expected: &[],
            },
            Case {
                name: "promise type argument shadowed",
                path: "case.ts",
                code: "export function f<Promise>(): Promise { return 1 as never; }\ntype Promise2 = Promise<unknown>;\nexport function g(): Promise2 { return Promise.resolve(1); }\n",
                expected: &[(116, 124, "This function exposes `unknown` to its caller. Parse the value at its boundary and return a named domain type.")],
            },
            Case {
                name: "parenthesized promise",
                path: "case.ts",
                code: "type G<T> = (Promise<(T)>);\nexport function f(): (G<unknown>) { return Promise.resolve(1); }\nexport function g(): Promise<(unknown)> { return Promise.resolve(1); }\n",
                expected: &[
                    (50, 60, "This function exposes `unknown` to its caller. Parse the value at its boundary and return a named domain type."),
                    (114, 132, "This function exposes `unknown` to its caller. Parse the value at its boundary and return a named domain type."),
                ],
            },
        ],
    );
}
