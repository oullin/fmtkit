//! `anti-slop-effect/prefer-tagged-error-handling`: expectations recorded from the upstream plugin
//! (github.com/k-mistele/anti-slop) under oxlint 1.86, one case per upstream
//! RuleTester case in `src/effect/rules/prefer-tagged-error-handling.test.ts`.

use super::super::super::anti_slop::tests::{Case, check};

/// src/effect/rules/prefer-tagged-error-handling.test.ts
#[test]
fn upstream() {
    check(
        "anti-slop-effect/prefer-tagged-error-handling",
        "[]",
        &[
            Case {
                name: "valid 0: const isNotFound = Predicate.isTagged(\"NotFound\");",
                path: "file.ts",
                code: "const isNotFound = Predicate.isTagged(\"NotFound\");",
                expected: &[],
            },
            Case {
                name: "valid 1: if (Predicate.isTagged(result, \"Ready\")) useReady(result);",
                path: "file.ts",
                code: "if (Predicate.isTagged(result, \"Ready\")) useReady(result);",
                expected: &[],
            },
            Case {
                name: "valid 2: Effect.catchTag(\"NotFound\", () => recover);",
                path: "file.ts",
                code: "Effect.catchTag(\"NotFound\", () => recover);",
                expected: &[],
            },
            Case {
                name: "valid 3: Effect.catchTag(\"Wrapper\", (error) => Predicate.isTagged(erro",
                path: "file.ts",
                code: "Effect.catchTag(\"Wrapper\", (error) => Predicate.isTagged(error.reason, \"Timeout\") ? retry : fail);",
                expected: &[],
            },
            Case {
                name: "valid 4: Effect.catch((error) => Effect.fail(error));",
                path: "file.ts",
                code: "Effect.catch((error) => Effect.fail(error));",
                expected: &[],
            },
            Case {
                name: "valid 5: Effect.catch((error) => items.filter((item) => Predicate.isTa",
                path: "file.ts",
                code: "Effect.catch((error) => items.filter((item) => Predicate.isTagged(item, \"Ready\")));",
                expected: &[],
            },
            Case {
                name: "invalid 6: Effect.catch((error) => Predicate.isTagged(error, \"NotFound",
                path: "file.ts",
                code: "Effect.catch((error) => Predicate.isTagged(error, \"NotFound\") ? recover : Effect.fail(error));",
                expected: &[(
                    24,
                    61,
                    "Use Effect.catchTag or Effect.catchTags instead of checking the error with `Predicate.isTagged` in a broad Effect catch handler.",
                )],
            },
            Case {
                name: "invalid 7: Effect.catchAll(function (error) { return Predicate.isTagge",
                path: "file.ts",
                code: "Effect.catchAll(function (error) { return Predicate.isTagged(\"NotFound\")(error) ? recover : fail; });",
                expected: &[(
                    42,
                    79,
                    "Use Effect.catchTag or Effect.catchTags instead of checking the error with `Predicate.isTagged` in a broad Effect catch handler.",
                )],
            },
            Case {
                name: "invalid 8: Effect.catchIf((error) => Predicate.isTagged(error, \"NotFou",
                path: "file.ts",
                code: "Effect.catchIf((error) => Predicate.isTagged(error, \"NotFound\"), () => recover);",
                expected: &[(
                    26,
                    63,
                    "Use Effect.catchTag or Effect.catchTags instead of checking the error with `Predicate.isTagged` in a broad Effect catch handler.",
                )],
            },
            Case {
                name: "invalid 9: Effect.catch((error) => Predicate.isTagged(error.reason, \"T",
                path: "file.ts",
                code: "Effect.catch((error) => Predicate.isTagged(error.reason, \"Timeout\") ? retry : fail);",
                expected: &[(
                    24,
                    67,
                    "Use Effect.catchReason or Effect.catchReasons instead of checking the error `reason` with `Predicate.isTagged` in a broad Effect catch handler.",
                )],
            },
        ],
    );
}

/// Edge cases the upstream tests miss, recorded from the upstream plugin.
#[test]
fn edges() {
    check(
        "anti-slop-effect/prefer-tagged-error-handling",
        "[]",
        &[
            Case {
                name: "valid 0: Predicate.isTagged(error, \"NotFound\");",
                path: "file.ts",
                code: "Predicate.isTagged(error, \"NotFound\");",
                expected: &[],
            },
            Case {
                name: "valid 1: Effect.catch((error) => P.isTagged(error, \"NotFound\") ? recov",
                path: "file.ts",
                code: "Effect.catch((error) => P.isTagged(error, \"NotFound\") ? recover : fail);",
                expected: &[],
            },
            Case {
                name: "valid 2: Effect.catch((error) => () => Predicate.isTagged(error, \"NotF",
                path: "file.ts",
                code: "Effect.catch((error) => () => Predicate.isTagged(error, \"NotFound\"));",
                expected: &[],
            },
            Case {
                name: "invalid 3: Effect.catch((error) => (Predicate.isTagged)((error), \"NotF",
                path: "file.ts",
                code: "Effect.catch((error) => (Predicate.isTagged)((error), \"NotFound\") ? recover : fail);",
                expected: &[(
                    24,
                    65,
                    "Use Effect.catchTag or Effect.catchTags instead of checking the error with `Predicate.isTagged` in a broad Effect catch handler.",
                )],
            },
            Case {
                name: "invalid 4: Effect.catchIf((error) => Predicate[\"isTagged\"](\"NotFound\")",
                path: "file.ts",
                code: "Effect.catchIf((error) => Predicate[\"isTagged\"](\"NotFound\")(error.reason), () => recover);",
                expected: &[],
            },
            Case {
                name: "invalid 5: Effect.catch((error) => Predicate.isTagged(error) ? a : Pre",
                path: "file.ts",
                code: "Effect.catch((error) => Predicate.isTagged(error) ? a : Predicate.isTagged(error.reason, \"B\") ? b : c);",
                expected: &[(
                    56,
                    93,
                    "Use Effect.catchReason or Effect.catchReasons instead of checking the error `reason` with `Predicate.isTagged` in a broad Effect catch handler.",
                )],
            },
        ],
    );
}
