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
                expected: &[(32, 39, "Parameter `payload` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function.")],
            },
            Case {
                name: "unknown arrow parameter",
                path: "case.ts",
                code: "export const decode = (payload: unknown): string => String(payload);\n",
                expected: &[(32, 39, "Parameter `payload` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function.")],
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
                expected: &[(24, 31, "Parameter `this` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function."), (40, 47, "Parameter `{ a }` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function."), (54, 61, "Parameter `[b]` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function."), (66, 73, "Parameter `d` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function."), (83, 90, "Parameter `f` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function."), (97, 104, "Parameter `e` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function."), (112, 119, "Parameter `c` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function.")],
            },
            Case {
                name: "destructured defaults keep their text",
                path: "case.ts",
                code: "export function f({ a }:unknown = {}, ...[b] : unknown): void {}\n",
                expected: &[(24, 31, "Parameter `{ a }:unknown = {}` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function."), (47, 54, "Parameter `...[b]` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function.")],
            },
            Case {
                name: "rest named cause and defaults named cause",
                path: "case.ts",
                code: "export function f(...cause: unknown): void {}\nexport function g(cause: unknown = 1): void {}\nexport class C {\n\tconstructor(private cause: unknown, public other: unknown) {}\n}\n",
                expected: &[(161, 168, "Parameter `other` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function.")],
            },
            Case {
                name: "signatures",
                path: "case.ts",
                code: "export interface I {\n\t(a: unknown): void;\n\tnew (b: unknown): I;\n\tm(c: unknown): void;\n}\nexport type F = (d: unknown) => void;\nexport type G = new (e: unknown) => I;\ndeclare function h(f: unknown): void;\nexport abstract class A {\n\tabstract m(g: unknown): void;\n}\n",
                expected: &[(26, 33, "Parameter `a` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function."), (51, 58, "Parameter `b` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function."), (70, 77, "Parameter `c` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function."), (108, 115, "Parameter `d` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function."), (150, 157, "Parameter `e` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function."), (187, 194, "Parameter `f` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function."), (244, 251, "Parameter `g` leaves input unparsed. Accept a named domain type; run the expected schema or parser at the I/O boundary before calling this function.")],
            },
            Case {
                name: "unions are not flagged",
                path: "case.ts",
                code: "export function f(a: unknown | null, b: Raw): void {}\ntype Raw = unknown;\n",
                expected: &[],
            },
        ],
    );
}
