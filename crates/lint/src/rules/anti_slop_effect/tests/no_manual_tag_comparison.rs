//! `anti-slop-effect/no-manual-tag-comparison`: expectations recorded from the upstream plugin
//! (github.com/k-mistele/anti-slop) under oxlint 1.86, one case per upstream
//! RuleTester case in `src/effect/rules/no-manual-tag-comparison.test.ts`.

use super::super::super::anti_slop::tests::{Case, check};

/// src/effect/rules/no-manual-tag-comparison.test.ts
#[test]
fn upstream() {
    check(
        "anti-slop-effect/no-manual-tag-comparison",
        "[]",
        &[
            Case { name: "valid 0: Predicate.isTagged(\"Ready\")(value);", path: "file.ts", code: "Predicate.isTagged(\"Ready\")(value);", expected: &[] },
            Case {
                name: "valid 1: Match.value(value).pipe(Match.tag(\"Ready\", handleReady));",
                path: "file.ts",
                code: "Match.value(value).pipe(Match.tag(\"Ready\", handleReady));",
                expected: &[],
            },
            Case {
                name: "valid 2: if (value.status === \"Ready\") handleReady(value);",
                path: "file.ts",
                code: "if (value.status === \"Ready\") handleReady(value);",
                expected: &[],
            },
            Case {
                name: "valid 3: Effect.catch((error) => error._tag === \"NotFound\" ? recover :",
                path: "file.ts",
                code: "Effect.catch((error) => error._tag === \"NotFound\" ? recover : fail);",
                expected: &[],
            },
            Case {
                name: "invalid 4: value._tag === \"Ready\";",
                path: "file.ts",
                code: "value._tag === \"Ready\";",
                expected: &[(0, 22, "Use Match.tag/Match.tags for tagged-value branching, or Predicate.isTagged for a simple reusable predicate.")],
            },
            Case {
                name: "invalid 5: \"Ready\" !== value[\"_tag\"];",
                path: "file.ts",
                code: "\"Ready\" !== value[\"_tag\"];",
                expected: &[(0, 25, "Use Match.tag/Match.tags for tagged-value branching, or Predicate.isTagged for a simple reusable predicate.")],
            },
            Case {
                name: "invalid 6: switch (value._tag) { case \"Ready\": handleReady(value); }",
                path: "file.ts",
                code: "switch (value._tag) { case \"Ready\": handleReady(value); }",
                expected: &[(
                    0,
                    57,
                    "Use Match.value(value).pipe(Match.tag/Match.tags/Match.tagsExhaustive) or the tagged enum `$match` helper instead of switching on `_tag`.",
                )],
            },
        ],
    );
}

/// Edge cases the upstream tests miss, recorded from the upstream plugin.
#[test]
fn edges() {
    check(
        "anti-slop-effect/no-manual-tag-comparison",
        "[]",
        &[
            Case { name: "valid 0: value._tag === other;", path: "file.ts", code: "value._tag === other;", expected: &[] },
            Case { name: "valid 1: value.tag === \"A\";", path: "file.ts", code: "value.tag === \"A\";", expected: &[] },
            Case {
                name: "valid 2: Effect.catch((error) => error._tag === \"NotFound\");",
                path: "file.ts",
                code: "Effect.catch((error) => error._tag === \"NotFound\");",
                expected: &[],
            },
            Case {
                name: "invalid 3: (value)._tag === \"A\";",
                path: "file.ts",
                code: "(value)._tag === \"A\";",
                expected: &[(0, 20, "Use Match.tag/Match.tags for tagged-value branching, or Predicate.isTagged for a simple reusable predicate.")],
            },
            Case {
                name: "invalid 4: \"A\" != value[\"_tag\"];",
                path: "file.ts",
                code: "\"A\" != value[\"_tag\"];",
                expected: &[(0, 20, "Use Match.tag/Match.tags for tagged-value branching, or Predicate.isTagged for a simple reusable predicate.")],
            },
            Case {
                name: "invalid 5: switch ((value as Tagged)._tag) { case \"A\": break; }",
                path: "file.ts",
                code: "switch ((value as Tagged)._tag) { case \"A\": break; }",
                expected: &[(
                    0,
                    52,
                    "Use Match.value(value).pipe(Match.tag/Match.tags/Match.tagsExhaustive) or the tagged enum `$match` helper instead of switching on `_tag`.",
                )],
            },
            Case {
                name: "invalid 6: E.catch((error) => error._tag === \"NotFound\");",
                path: "file.ts",
                code: "E.catch((error) => error._tag === \"NotFound\");",
                expected: &[(19, 44, "Use Match.tag/Match.tags for tagged-value branching, or Predicate.isTagged for a simple reusable predicate.")],
            },
            Case {
                name: "invalid 7: Effect.catchTag(\"A\", (error) => error.reason._tag === \"Time",
                path: "file.ts",
                code: "Effect.catchTag(\"A\", (error) => error.reason._tag === \"Timeout\");",
                expected: &[(32, 63, "Use Match.tag/Match.tags for tagged-value branching, or Predicate.isTagged for a simple reusable predicate.")],
            },
            Case {
                name: "invalid 8: Effect.catch((error) => () => error._tag === \"NotFound\");",
                path: "file.ts",
                code: "Effect.catch((error) => () => error._tag === \"NotFound\");",
                expected: &[(30, 55, "Use Match.tag/Match.tags for tagged-value branching, or Predicate.isTagged for a simple reusable predicate.")],
            },
        ],
    );
}
