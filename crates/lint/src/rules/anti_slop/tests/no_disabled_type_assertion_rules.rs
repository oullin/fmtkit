//! `anti-slop/no-disabled-type-assertion-rules`: expectations recorded from the upstream plugin
//! (github.com/k-mistele/anti-slop) under oxlint 1.86, one case per upstream
//! RuleTester case in `src/rules/no-disabled-type-assertion-rules.test.ts`.

use super::{Case, check};

/// src/rules/no-disabled-type-assertion-rules.test.ts
#[test]
fn upstream() {
    check(
        "anti-slop/no-disabled-type-assertion-rules",
        "[]",
        &[
            Case {
                name: "valid 0: // SAFETY: The parser checked it.",
                path: "file.ts",
                code: "// SAFETY: The parser checked it.\nconst id = value as UserId;",
                expected: &[],
            },
            Case {
                name: "valid 1: // oxlint-disable-next-line effecttsgo/return-effect-in-gen -",
                path: "file.ts",
                code: "// oxlint-disable-next-line effecttsgo/return-effect-in-gen -- the inner Effect is the result\nconst x = 1;",
                expected: &[],
            },
            Case {
                name: "valid 2: const x = 1; // eslint-disable-line no-console",
                path: "file.ts",
                code: "const x = 1; // eslint-disable-line no-console",
                expected: &[],
            },
            Case { name: "valid 3: /* oxlint-disable no-console */", path: "file.ts", code: "/* oxlint-disable no-console */\nconst x = 1;", expected: &[] },
            Case {
                name: "valid 4: // This mentions oxlint-disable-next-line typescript/no-unsaf",
                path: "file.ts",
                code: "// This mentions oxlint-disable-next-line typescript/no-unsafe-type-assertion in prose.\nconst x = 1;",
                expected: &[],
            },
            Case {
                name: "valid 5: // oxlint-disable-next-line local/no-type-assertion",
                path: "file.ts",
                code: "// oxlint-disable-next-line local/no-type-assertion\nconst id = value as UserId;",
                expected: &[],
            },
            Case {
                name: "invalid 6: // oxlint-disable-next-line typescript/consistent-type-asse",
                path: "file.ts",
                code: "// oxlint-disable-next-line typescript/consistent-type-assertions\nconst id = value as UserId;",
                expected: &[(
                    0,
                    65,
                    "Don't disable `typescript/consistent-type-assertions` inline. Remove the cast, or add a file override with its reason in the lint config.",
                )],
            },
            Case {
                name: "invalid 7: // oxlint-disable-next-line typescript/consistent-type-asse",
                path: "file.ts",
                code: "// oxlint-disable-next-line typescript/consistent-type-assertions, anti-slop/no-widen-then-assert -- reason\nconst id = value as UserId;",
                // Reordered, not changed: `check` compares the actual findings sorted
                // by (start, end, message), and upstream reports these two in list order.
                expected: &[
                    (
                        0,
                        107,
                        "Don't disable `anti-slop/no-widen-then-assert` inline. Remove the cast, or add a file override with its reason in the lint config.",
                    ),
                    (
                        0,
                        107,
                        "Don't disable `typescript/consistent-type-assertions` inline. Remove the cast, or add a file override with its reason in the lint config.",
                    ),
                ],
            },
            Case {
                name: "invalid 8: const id = value as UserId; // eslint-disable-line @typescr",
                path: "file.ts",
                code: "const id = value as UserId; // eslint-disable-line @typescript-eslint/no-unsafe-type-assertion",
                expected: &[(
                    28,
                    94,
                    "Don't disable `typescript/no-unsafe-type-assertion` inline. Remove the cast, or add a file override with its reason in the lint config.",
                )],
            },
            Case {
                name: "invalid 9: /* oxlint-disable anti-slop/no-chained-type-assertions */",
                path: "file.ts",
                code: "/* oxlint-disable anti-slop/no-chained-type-assertions */\nconst id = value as unknown as UserId;",
                expected: &[(
                    0,
                    57,
                    "Don't disable `anti-slop/no-chained-type-assertions` inline. Remove the cast, or add a file override with its reason in the lint config.",
                )],
            },
            Case {
                name: "invalid 11: // oxlint-disable-next-line",
                path: "file.ts",
                code: "// oxlint-disable-next-line\nconst id = value as UserId;",
                expected: &[(0, 27, "This disable comment names no rule, so it turns off the type-assertion rules too. Name the rules it disables.")],
            },
            Case { name: "invalid 12: /* eslint-disable */", path: "file.ts", code: "/* eslint-disable */\nconst id = value as UserId;", expected: &[] },
            Case {
                name: "invalid 13: // oxlint-disable-next-line -- just this once",
                path: "file.ts",
                code: "// oxlint-disable-next-line -- just this once\nconst id = value as UserId;",
                expected: &[(0, 45, "This disable comment names no rule, so it turns off the type-assertion rules too. Name the rules it disables.")],
            },
        ],
    );
}

/// src/rules/no-disabled-type-assertion-rules.test.ts, options `[{"additionalRules":["local/no-type-assertion"]}]`
#[test]
fn upstream_options_1() {
    check(
        "anti-slop/no-disabled-type-assertion-rules",
        "[{\"additionalRules\":[\"local/no-type-assertion\"]}]",
        &[Case {
            name: "invalid 10: // oxlint-disable-next-line local/no-type-assertion",
            path: "file.ts",
            code: "// oxlint-disable-next-line local/no-type-assertion\nconst id = value as UserId;",
            expected: &[(0, 51, "Don't disable `local/no-type-assertion` inline. Remove the cast, or add a file override with its reason in the lint config.")],
        }],
    );
}

/// Edge cases recorded from the upstream plugin under oxlint 1.86.
#[test]
fn edges() {
    check(
        "anti-slop/no-disabled-type-assertion-rules",
        "[]",
        &[
            Case {
                name: "edge 0: /* eslint-disable-next-line typescript/no-unsafe-type-asser",
                path: "file.ts",
                code: "/* eslint-disable-next-line typescript/no-unsafe-type-assertion */\nconst id = value as UserId;",
                expected: &[(
                    0,
                    66,
                    "Don't disable `typescript/no-unsafe-type-assertion` inline. Remove the cast, or add a file override with its reason in the lint config.",
                )],
            },
            Case { name: "edge 1: //oxlint-disable-line", path: "file.ts", code: "//oxlint-disable-line\nconst id = value as UserId;", expected: &[] },
            Case {
                name: "edge 2: // oxlint-disable-next-line typescript/no-unsafe-type-asser",
                path: "file.ts",
                code: "// oxlint-disable-next-line typescript/no-unsafe-type-assertion--no-space\nconst id = value as UserId;",
                expected: &[],
            },
            Case {
                name: "edge 3: // oxlint-disable-next-line typescript/no-unsafe-type-asser",
                path: "file.ts",
                code: "// oxlint-disable-next-line typescript/no-unsafe-type-assertion --\nconst id = value as UserId;",
                expected: &[(
                    0,
                    66,
                    "Don't disable `typescript/no-unsafe-type-assertion` inline. Remove the cast, or add a file override with its reason in the lint config.",
                )],
            },
            Case {
                name: "edge 4: // oxlint-disable-next-line ,",
                path: "file.ts",
                code: "// oxlint-disable-next-line ,\nconst id = value as UserId;",
                expected: &[(0, 29, "This disable comment names no rule, so it turns off the type-assertion rules too. Name the rules it disables.")],
            },
            Case {
                name: "edge 5: // oxlint-disable-next-lineX typescript/no-unsafe-type-asse",
                path: "file.ts",
                code: "// oxlint-disable-next-lineX typescript/no-unsafe-type-assertion\nconst id = value as UserId;",
                expected: &[],
            },
            Case {
                name: "edge 6: // eslint-disable-next-line @typescript-eslint/consistent-t",
                path: "file.ts",
                code: "// eslint-disable-next-line @typescript-eslint/consistent-type-assertions, typescript-eslint/no-unsafe-type-assertion\nconst id = value as UserId;",
                expected: &[
                    (
                        0,
                        117,
                        "Don't disable `typescript/consistent-type-assertions` inline. Remove the cast, or add a file override with its reason in the lint config.",
                    ),
                    (
                        0,
                        117,
                        "Don't disable `typescript/no-unsafe-type-assertion` inline. Remove the cast, or add a file override with its reason in the lint config.",
                    ),
                ],
            },
            Case {
                name: "edge 7: // oxlint-enable typescript/no-unsafe-type-assertion",
                path: "file.ts",
                code: "// oxlint-enable typescript/no-unsafe-type-assertion\nconst id = value as UserId;",
                expected: &[],
            },
            Case {
                name: "edge 8: /** oxlint-disable-next-line typescript/no-unsafe-type-asse",
                path: "file.ts",
                code: "/** oxlint-disable-next-line typescript/no-unsafe-type-assertion */\nconst id = value as UserId;",
                expected: &[],
            },
            Case {
                name: "edge 9: /*",
                path: "file.ts",
                code: "/*\n oxlint-disable-next-line typescript/consistent-type-assertions\n*/\nconst id = value as UserId;",
                expected: &[(
                    0,
                    69,
                    "Don't disable `typescript/consistent-type-assertions` inline. Remove the cast, or add a file override with its reason in the lint config.",
                )],
            },
            Case {
                name: "edge 10: // oxlint-disable-next-line typescript/consistent-type-ass",
                path: "file.ts",
                code: "// oxlint-disable-next-line typescript/consistent-type-assertions, typescript/consistent-type-assertions\nconst id = value as UserId;",
                expected: &[
                    (
                        0,
                        104,
                        "Don't disable `typescript/consistent-type-assertions` inline. Remove the cast, or add a file override with its reason in the lint config.",
                    ),
                    (
                        0,
                        104,
                        "Don't disable `typescript/consistent-type-assertions` inline. Remove the cast, or add a file override with its reason in the lint config.",
                    ),
                ],
            },
            Case {
                name: "edge 11: // oxlint-disable-next-line typescript/consistent-type-ass",
                path: "file.ts",
                code: "// oxlint-disable-next-line typescript/consistent-type-assertions -- reason -- more, anti-slop/no-widen-then-assert\nconst id = value as UserId;",
                expected: &[(
                    0,
                    115,
                    "Don't disable `typescript/consistent-type-assertions` inline. Remove the cast, or add a file override with its reason in the lint config.",
                )],
            },
            Case {
                name: "edge 12: // oxlint-disable-next-line typescript/consistent-type-ass",
                path: "file.ts",
                code: "// oxlint-disable-next-line typescript/consistent-type-assertions\t--\treason\nconst id = value as UserId;",
                expected: &[(
                    0,
                    75,
                    "Don't disable `typescript/consistent-type-assertions` inline. Remove the cast, or add a file override with its reason in the lint config.",
                )],
            },
            Case {
                name: "edge 13: // oxlint-disable-next-line typescript/consistent-type-ass",
                path: "file.ts",
                code: "// oxlint-disable-next-line typescript/consistent-type-assertions--reason\nconst id = value as UserId;",
                expected: &[],
            },
            Case {
                name: "edge 14: // oxlint-disable-next-line typescript/no-explicit-any, ef",
                path: "file.ts",
                code: "// oxlint-disable-next-line typescript/no-explicit-any, effecttsgo/unsafe-effect-type-assertion\nconst id = value as UserId;",
                expected: &[(
                    0,
                    95,
                    "Don't disable `effecttsgo/unsafe-effect-type-assertion` inline. Remove the cast, or add a file override with its reason in the lint config.",
                )],
            },
            Case {
                name: "edge 15: // eslint-disable-next-line typescript/no-explicit-any",
                path: "file.ts",
                code: "// eslint-disable-next-line typescript/no-explicit-any\nconst id = value as UserId;",
                expected: &[],
            },
            Case {
                name: "edge 16: // oxlint-disable-next-line anti-slop/require-safety-comme",
                path: "file.ts",
                code: "// oxlint-disable-next-line anti-slop/require-safety-comment-for-type-assertion -- 😀 reason\nconst id = value as UserId;",
                expected: &[(
                    0,
                    94,
                    "Don't disable `anti-slop/require-safety-comment-for-type-assertion` inline. Remove the cast, or add a file override with its reason in the lint config.",
                )],
            },
            Case { name: "edge 17: const a = 1; /* oxlint-disable-line */", path: "file.ts", code: "const a = 1; /* oxlint-disable-line */", expected: &[] },
            Case {
                name: "edge 18: // oxlint-disable-next-line typescript/consistent-type-ass",
                path: "file.ts",
                code: "// oxlint-disable-next-line typescript/consistent-type-assertions\nconst id = value as UserId;",
                expected: &[(
                    0,
                    66,
                    "Don't disable `typescript/consistent-type-assertions` inline. Remove the cast, or add a file override with its reason in the lint config.",
                )],
            },
        ],
    );
}

/// Edge cases recorded from the upstream plugin under oxlint 1.86, options `[{"additionalRules":["@typescript-eslint/no-type-assertion"]}]`.
#[test]
fn edges_options_1() {
    check(
        "anti-slop/no-disabled-type-assertion-rules",
        "[{\"additionalRules\":[\"@typescript-eslint/no-type-assertion\"]}]",
        &[
            Case {
                name: "edge 19: // oxlint-disable-next-line typescript/no-type-assertion",
                path: "file.ts",
                code: "// oxlint-disable-next-line typescript/no-type-assertion\nconst id = value as UserId;",
                expected: &[(
                    0,
                    56,
                    "Don't disable `typescript/no-type-assertion` inline. Remove the cast, or add a file override with its reason in the lint config.",
                )],
            },
            Case {
                name: "edge 20: // oxlint-disable-next-line @typescript-eslint/no-type-ass",
                path: "file.ts",
                code: "// oxlint-disable-next-line @typescript-eslint/no-type-assertion\nconst id = value as UserId;",
                expected: &[(
                    0,
                    64,
                    "Don't disable `typescript/no-type-assertion` inline. Remove the cast, or add a file override with its reason in the lint config.",
                )],
            },
        ],
    );
}

/// Edge cases recorded from the upstream plugin under oxlint 1.86, options `[{"additionalRules":[]}]`.
#[test]
fn edges_options_2() {
    check(
        "anti-slop/no-disabled-type-assertion-rules",
        "[{\"additionalRules\":[]}]",
        &[Case {
            name: "edge 21: // oxlint-disable-next-line typescript/consistent-type-ass",
            path: "file.ts",
            code: "// oxlint-disable-next-line typescript/consistent-type-assertions\nconst id = value as UserId;",
            expected: &[(
                0,
                65,
                "Don't disable `typescript/consistent-type-assertions` inline. Remove the cast, or add a file override with its reason in the lint config.",
            )],
        }],
    );
}

/// Written by hand: options outside upstream's schema are rejected.
#[test]
fn rejects_invalid_options() {
    let factory =
        super::super::RULES.iter().find(|(name, _)| *name == "anti-slop/no-disabled-type-assertion-rules").map(|(_, factory)| *factory).expect("registered");

    for options in [
        r#"[{"additionalRules":"a/b"}]"#,
        r#"[{"additionalRules":[""]}]"#,
        r#"[{"additionalRules":[1]}]"#,
        r#"[{"additionalRules":["a/b","a/b"]}]"#,
        r#"[{"rules":[]}]"#,
        "[[]]",
        "[{}, {}]",
    ] {
        let options: Vec<serde_json::Value> = serde_json::from_str(options).expect("JSON");

        assert!(factory(&options).is_err(), "accepts {options:?}");
    }
}
