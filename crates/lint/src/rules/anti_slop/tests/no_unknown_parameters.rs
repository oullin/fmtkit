//! `anti-slop/no-unknown-parameters`: expectations recorded from the v1 plugin under oxlint 1.86.

use super::{Case, check};

/// tests/no-unknown-parameters.test.ts
#[test]
fn v1() {
    check(
        "anti-slop/no-unknown-parameters",
        "[]",
        &[
            Case {
                name: "domain parameter",
                path: "case.ts",
                code: "export function widen(value: string): number {\n\treturn value.length;\n}\n",
                expected: &[],
            },
            Case {
                name: "cause is the sanctioned exception",
                path: "case.ts",
                code: "export function wrap(cause: unknown): Error {\n\treturn new Error(\"failed\", { cause });\n}\n",
                expected: &[],
            },
            Case {
                name: "unknown parameter",
                path: "case.ts",
                code: "export function decode(payload: unknown): string {\n\treturn String(payload);\n}\n",
                expected: &[(
                    32,
                    39,
                    "Parameter `payload` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function.",
                )],
            },
            Case {
                name: "unknown arrow parameter",
                path: "case.ts",
                code: "export const decode = (payload: unknown): string => String(payload);\n",
                expected: &[(
                    32,
                    39,
                    "Parameter `payload` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function.",
                )],
            },
        ],
    );
}

#[test]
fn edges() {
    check(
        "anti-slop/no-unknown-parameters",
        "[]",
        &[
            Case {
                name: "parameter forms",
                path: "case.ts",
                code: "export function f(this: unknown, { a }: unknown, [b]: unknown, d: unknown = 1, f: (unknown), e?: unknown, ...c: unknown): void {}\n",
                expected: &[
                    (
                        24,
                        31,
                        "Parameter `this` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function.",
                    ),
                    (
                        40,
                        47,
                        "Parameter `{ a }` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function.",
                    ),
                    (
                        54,
                        61,
                        "Parameter `[b]` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function.",
                    ),
                    (
                        66,
                        73,
                        "Parameter `d` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function.",
                    ),
                    (
                        83,
                        90,
                        "Parameter `f` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function.",
                    ),
                    (
                        97,
                        104,
                        "Parameter `e` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function.",
                    ),
                    (
                        112,
                        119,
                        "Parameter `c` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function.",
                    ),
                ],
            },
            Case {
                name: "destructured parameters are named by their pattern, as upstream does",
                path: "case.ts",
                code: "export function f({ a }:unknown = {}, ...[b] : unknown): void {}\n",
                expected: &[
                    (
                        24,
                        31,
                        "Parameter `{ a }` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function.",
                    ),
                    (
                        47,
                        54,
                        "Parameter `[b]` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function.",
                    ),
                ],
            },
            Case {
                name: "rest named cause and defaults named cause",
                path: "case.ts",
                code: "export function f(...cause: unknown): void {}\nexport function g(cause: unknown = 1): void {}\nexport class C {\n\tconstructor(private cause: unknown, public other: unknown) {}\n}\n",
                expected: &[(
                    161,
                    168,
                    "Parameter `other` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function.",
                )],
            },
            Case {
                name: "signatures",
                path: "case.ts",
                code: "export interface I {\n\t(a: unknown): void;\n\tnew (b: unknown): I;\n\tm(c: unknown): void;\n}\nexport type F = (d: unknown) => void;\nexport type G = new (e: unknown) => I;\ndeclare function h(f: unknown): void;\nexport abstract class A {\n\tabstract m(g: unknown): void;\n}\n",
                expected: &[
                    (
                        26,
                        33,
                        "Parameter `a` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function.",
                    ),
                    (
                        51,
                        58,
                        "Parameter `b` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function.",
                    ),
                    (
                        70,
                        77,
                        "Parameter `c` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function.",
                    ),
                    (
                        108,
                        115,
                        "Parameter `d` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function.",
                    ),
                    (
                        150,
                        157,
                        "Parameter `e` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function.",
                    ),
                    (
                        187,
                        194,
                        "Parameter `f` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function.",
                    ),
                    (
                        244,
                        251,
                        "Parameter `g` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function.",
                    ),
                ],
            },
            Case {
                name: "a union containing unknown is flagged, as upstream does; an alias is not",
                path: "case.ts",
                code: "export function f(a: unknown | null, b: Raw): void {}\ntype Raw = unknown;\n",
                expected: &[(
                    21,
                    35,
                    "Parameter `a` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function.",
                )],
            },
        ],
    );
}

/// src/rules/no-unknown-parameters.test.ts
#[test]
fn upstream_current() {
    check(
        "anti-slop/no-unknown-parameters",
        "[]",
        &[
            Case {
                name: "valid 0: function enrich(cause: unknown): void {}",
                path: "file.ts",
                code: "function enrich(cause: unknown): void {}",
                expected: &[],
            },
            Case {
                name: "valid 1: function enrich(cause: Error | unknown): void {}",
                path: "file.ts",
                code: "function enrich(cause: Error | unknown): void {}",
                expected: &[],
            },
            Case {
                name: "valid 2: function isString(value: unknown): value is string { return t",
                path: "file.ts",
                code: "function isString(value: unknown): value is string { return true; }",
                expected: &[],
            },
            Case {
                name: "valid 3: const isString = (value: unknown): value is string => true;",
                path: "file.ts",
                code: "const isString = (value: unknown): value is string => true;",
                expected: &[],
            },
            Case {
                name: "valid 4: function assertString(value: unknown): asserts value is strin",
                path: "file.ts",
                code: "function assertString(value: unknown): asserts value is string {}",
                expected: &[],
            },
            Case {
                name: "valid 5: type Guard = (value: unknown) => value is string;",
                path: "file.ts",
                code: "type Guard = (value: unknown) => value is string;",
                expected: &[],
            },
            Case {
                name: "valid 6: declare function isString(value: unknown): value is string;",
                path: "file.ts",
                code: "declare function isString(value: unknown): value is string;",
                expected: &[],
            },
            Case {
                name: "valid 7: type Guards = { isString(value: unknown): value is string };",
                path: "file.ts",
                code: "type Guards = { isString(value: unknown): value is string };",
                expected: &[],
            },
            Case {
                name: "valid 8: function parse(value: string | number): void {}",
                path: "file.ts",
                code: "function parse(value: string | number): void {}",
                expected: &[],
            },
            Case {
                name: "invalid 9: function parse(value: unknown): void {}",
                path: "file.ts",
                code: "function parse(value: unknown): void {}",
                expected: &[(
                    22,
                    29,
                    "Parameter `value` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function.",
                )],
            },
            Case {
                name: "invalid 10: function parse(value: string | unknown): void {}",
                path: "file.ts",
                code: "function parse(value: string | unknown): void {}",
                expected: &[(
                    22,
                    38,
                    "Parameter `value` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function.",
                )],
            },
            Case {
                name: "invalid 11: function parse(value: string | (number | unknown)): void {",
                path: "file.ts",
                code: "function parse(value: string | (number | unknown)): void {}",
                expected: &[(
                    22,
                    49,
                    "Parameter `value` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function.",
                )],
            },
            Case {
                name: "invalid 12: function isString(value: unknown, context: unknown): value",
                path: "file.ts",
                code: "function isString(value: unknown, context: unknown): value is string { return true; }",
                expected: &[(
                    43,
                    50,
                    "Parameter `context` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function.",
                )],
            },
            Case {
                name: "invalid 13: export function parse({ value }: unknown = {}): void {}",
                path: "file.ts",
                code: "export function parse({ value }: unknown = {}): void {}",
                expected: &[(
                    33,
                    40,
                    "Parameter `{ value }` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function.",
                )],
            },
        ],
    );
}

/// Edge cases recorded from the upstream plugin under oxlint 1.86.
#[test]
fn upstream_edges() {
    check(
        "anti-slop/no-unknown-parameters",
        "[]",
        &[
            Case {
                name: "edge 0: function f(value: (unknown) | string, { a }: string | unkno",
                path: "file.ts",
                code: "function f(value: (unknown) | string, { a }: string | unknown, ...[b]: unknown | null): void {}",
                expected: &[
                    (
                        18,
                        36,
                        "Parameter `value` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function.",
                    ),
                    (
                        45,
                        61,
                        "Parameter `{ a }` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function.",
                    ),
                    (
                        71,
                        85,
                        "Parameter `[b]` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function.",
                    ),
                ],
            },
            Case {
                name: "edge 1: function f(this: unknown): this is string { return true; }",
                path: "file.ts",
                code: "function f(this: unknown): this is string { return true; }",
                expected: &[(
                    17,
                    24,
                    "Parameter `this` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function.",
                )],
            },
            Case {
                name: "edge 2: function f(value: unknown): asserts value {}",
                path: "file.ts",
                code: "function f(value: unknown): asserts value {}",
                expected: &[],
            },
            Case {
                name: "edge 3: class C { isS(value: unknown): value is string { return tru",
                path: "file.ts",
                code: "class C { isS(value: unknown): value is string { return true; } }",
                expected: &[],
            },
            Case {
                name: "edge 4: function f(value: unknown): asserts this is string {}",
                path: "file.ts",
                code: "function f(value: unknown): asserts this is string {}",
                expected: &[(
                    18,
                    25,
                    "Parameter `value` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function.",
                )],
            },
            Case {
                name: "edge 5: function f(value: unknown, cause: string | unknown): void {",
                path: "file.ts",
                code: "function f(value: unknown, cause: string | unknown): void {}",
                expected: &[(
                    18,
                    25,
                    "Parameter `value` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function.",
                )],
            },
            Case {
                name: "edge 6: function f(...value: unknown): value is string { return tru",
                path: "file.ts",
                code: "function f(...value: unknown): value is string { return true; }",
                expected: &[],
            },
            Case {
                name: "edge 7: function f(value: unknown = 1): value is string { return tr",
                path: "file.ts",
                code: "function f(value: unknown = 1): value is string { return true; }",
                expected: &[],
            },
            Case {
                name: "edge 8: class C { constructor(private value: unknown) {} }",
                path: "file.ts",
                code: "class C { constructor(private value: unknown) {} }",
                expected: &[(
                    37,
                    44,
                    "Parameter `value` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function.",
                )],
            },
            Case {
                name: "edge 9: function f(value: unknown & string): void {}",
                path: "file.ts",
                code: "function f(value: unknown & string): void {}",
                expected: &[],
            },
            Case { name: "edge 10: function f(value: unknown[]): void {}", path: "file.ts", code: "function f(value: unknown[]): void {}", expected: &[] },
            Case {
                name: "edge 11: function f({ a }: unknown = {}, [b]: unknown = []): void {",
                path: "file.ts",
                code: "function f({ a }: unknown = {}, [b]: unknown = []): void {}",
                expected: &[
                    (
                        18,
                        25,
                        "Parameter `{ a }` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function.",
                    ),
                    (
                        37,
                        44,
                        "Parameter `[b]` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function.",
                    ),
                ],
            },
            Case {
                name: "edge 12: function f(...[b]: unknown): void {}",
                path: "file.ts",
                code: "function f(...[b]: unknown): void {}",
                expected: &[(
                    19,
                    26,
                    "Parameter `[b]` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function.",
                )],
            },
            Case {
                name: "edge 13: function f(value: string | (unknown)): void {}",
                path: "file.ts",
                code: "function f(value: string | (unknown)): void {}",
                expected: &[(
                    18,
                    36,
                    "Parameter `value` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function.",
                )],
            },
            Case {
                name: "edge 14: function f({ a }?: unknown): void {}",
                path: "file.ts",
                code: "function f({ a }?: unknown): void {}",
                expected: &[(
                    19,
                    26,
                    "Parameter `{ a }?` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function.",
                )],
            },
            Case {
                name: "edge 15: type Assert = (value: unknown) => asserts value;",
                path: "file.ts",
                code: "type Assert = (value: unknown) => asserts value;",
                expected: &[],
            },
            Case {
                name: "edge 16: const f = function (other: unknown, value: unknown): value",
                path: "file.ts",
                code: "const f = function (other: unknown, value: unknown): value is string { return true; };",
                expected: &[(
                    27,
                    34,
                    "Parameter `other` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function.",
                )],
            },
            Case {
                name: "edge 17: function f(value: unknown): (string | unknown) { return va",
                path: "file.ts",
                code: "function f(value: unknown): (string | unknown) { return value; }",
                expected: &[(
                    18,
                    25,
                    "Parameter `value` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function.",
                )],
            },
            Case {
                name: "edge 18: function f(value: unknown): typeof value { return value; }",
                path: "file.ts",
                code: "function f(value: unknown): typeof value { return value; }",
                expected: &[(
                    18,
                    25,
                    "Parameter `value` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function.",
                )],
            },
        ],
    );
}
