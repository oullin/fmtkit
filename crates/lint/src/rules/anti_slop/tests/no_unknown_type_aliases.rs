//! `anti-slop/no-unknown-type-aliases`: expectations recorded from the v1 plugin under oxlint 1.86.

use super::{Case, check};

/// tests/no-unknown-type-aliases.test.ts
#[test]
fn v1() {
    check(
        "anti-slop/no-unknown-type-aliases",
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

#[test]
fn edges() {
    check(
        "anti-slop/no-unknown-type-aliases",
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
                expected: &[(
                    32,
                    33,
                    "Type alias `G` hides `unknown`. Keep `unknown` explicit at the parsing boundary or on an allowed `cause` field; otherwise use the parsed owner type.",
                )],
            },
            Case { name: "cycles", path: "case.ts", code: "type A = B;\ntype B = A;\ntype S = S;\n", expected: &[] },
            Case {
                name: "nested aliases are ignored",
                path: "case.ts",
                code: "declare namespace NS {\n\ttype A = unknown;\n}\nexport function f(): void {\n\ttype B = unknown;\n}\n",
                expected: &[],
            },
            Case { name: "default exports are not unwrapped", path: "case.ts", code: "export default interface I {}\ntype A = I;\n", expected: &[] },
        ],
    );
}
