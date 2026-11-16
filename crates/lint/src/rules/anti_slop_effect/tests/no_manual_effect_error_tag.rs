//! `anti-slop-effect/no-manual-effect-error-tag`: expectations recorded from the upstream plugin
//! (github.com/k-mistele/anti-slop) under oxlint 1.86, one case per upstream
//! RuleTester case in `src/effect/rules/no-manual-effect-error-tag.test.ts`.

use super::super::super::anti_slop::tests::{Case, check};

/// src/effect/rules/no-manual-effect-error-tag.test.ts
#[test]
fn upstream() {
    check(
        "anti-slop-effect/no-manual-effect-error-tag",
        "[]",
        &[
            Case { name: "valid 0: error._tag === \"NotFound\";", path: "file.ts", code: "error._tag === \"NotFound\";", expected: &[] },
            Case { name: "valid 1: Effect.catchTag(\"NotFound\", recover);", path: "file.ts", code: "Effect.catchTag(\"NotFound\", recover);", expected: &[] },
            Case {
                name: "valid 2: Effect.catchTag(\"Wrapper\", (error) => error.reason._tag === \"",
                path: "file.ts",
                code: "Effect.catchTag(\"Wrapper\", (error) => error.reason._tag === \"Timeout\" ? retry : fail);",
                expected: &[],
            },
            Case {
                name: "invalid 3: Effect.catch((error) => error._tag === \"NotFound\" ? recover",
                path: "file.ts",
                code: "Effect.catch((error) => error._tag === \"NotFound\" ? recover : fail);",
                expected: &[(
                    24,
                    49,
                    "Use Effect.catchTag or Effect.catchTags instead of manually discriminating a tagged error in a broad Effect catch handler.",
                )],
            },
            Case {
                name: "invalid 4: Effect.catchAll(function (error) { return error.reason._tag",
                path: "file.ts",
                code: "Effect.catchAll(function (error) { return error.reason._tag === \"Timeout\" ? retry : fail; });",
                expected: &[(
                    42,
                    73,
                    "Use Effect.catchReason or Effect.catchReasons instead of manually discriminating a tagged `reason` in a broad Effect catch handler.",
                )],
            },
            Case {
                name: "invalid 5: Effect.catchIf(predicate, (error) => { switch (error._tag) ",
                path: "file.ts",
                code: "Effect.catchIf(predicate, (error) => { switch (error._tag) { case \"NotFound\": return recover; } });",
                expected: &[(
                    39,
                    95,
                    "Use Effect.catchTag or Effect.catchTags instead of manually discriminating a tagged error in a broad Effect catch handler.",
                )],
            },
        ],
    );
}

/// Edge cases the upstream tests miss, recorded from the upstream plugin.
#[test]
fn edges() {
    check(
        "anti-slop-effect/no-manual-effect-error-tag",
        "[]",
        &[
            Case {
                name: "valid 0: E.catch((error) => error._tag === \"NotFound\" ? recover : fail",
                path: "file.ts",
                code: "E.catch((error) => error._tag === \"NotFound\" ? recover : fail);",
                expected: &[],
            },
            Case {
                name: "valid 1: Effect.catch((error) => () => error._tag === \"NotFound\");",
                path: "file.ts",
                code: "Effect.catch((error) => () => error._tag === \"NotFound\");",
                expected: &[],
            },
            Case {
                name: "valid 2: Effect.catch((error) => error._tag === other ? recover : fail",
                path: "file.ts",
                code: "Effect.catch((error) => error._tag === other ? recover : fail);",
                expected: &[],
            },
            Case {
                name: "valid 3: Effect.catchTag(\"A\", (error) => error._tag === \"NotFound\");",
                path: "file.ts",
                code: "Effect.catchTag(\"A\", (error) => error._tag === \"NotFound\");",
                expected: &[],
            },
            Case {
                name: "invalid 4: Effect.catch(((error) => (error)._tag === \"NotFound\" ? reco",
                path: "file.ts",
                code: "Effect.catch(((error) => (error)._tag === \"NotFound\" ? recover : fail));",
                expected: &[(
                    25,
                    52,
                    "Use Effect.catchTag or Effect.catchTags instead of manually discriminating a tagged error in a broad Effect catch handler.",
                )],
            },
            Case {
                name: "invalid 5: Effect.catch((error) => \"NotFound\" !== error[\"_tag\"]);",
                path: "file.ts",
                code: "Effect.catch((error) => \"NotFound\" !== error[\"_tag\"]);",
                expected: &[(
                    24,
                    52,
                    "Use Effect.catchTag or Effect.catchTags instead of manually discriminating a tagged error in a broad Effect catch handler.",
                )],
            },
            Case {
                name: "invalid 6: Effect.catchAll(function (error) { switch (error.reason._ta",
                path: "file.ts",
                code: "Effect.catchAll(function (error) { switch (error.reason._tag) { case \"A\": return a; } if (error._tag == \"B\") return b; });",
                expected: &[
                    (
                        35,
                        85,
                        "Use Effect.catchReason or Effect.catchReasons instead of manually discriminating a tagged `reason` in a broad Effect catch handler.",
                    ),
                    (90, 107, "Use Effect.catchTag or Effect.catchTags instead of manually discriminating a tagged error in a broad Effect catch handler."),
                ],
            },
            Case {
                name: "invalid 7: Effect.catch((error) => (error as Tagged)._tag === \"NotFoun",
                path: "file.ts",
                code: "Effect.catch((error) => (error as Tagged)._tag === \"NotFound\");",
                expected: &[(
                    24,
                    61,
                    "Use Effect.catchTag or Effect.catchTags instead of manually discriminating a tagged error in a broad Effect catch handler.",
                )],
            },
        ],
    );
}
