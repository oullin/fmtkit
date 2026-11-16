//! `anti-slop-effect/no-manual-tagged-construction`: expectations recorded from the upstream plugin
//! (github.com/k-mistele/anti-slop) under oxlint 1.86, one case per upstream
//! RuleTester case in `src/effect/rules/no-manual-tagged-construction.test.ts`.

use super::super::super::anti_slop::tests::{Case, check};

/// src/effect/rules/no-manual-tagged-construction.test.ts
#[test]
fn upstream() {
    check(
        "anti-slop-effect/no-manual-tagged-construction",
        "[]",
        &[
            Case {
                name: "valid 0: Match.value(value).pipe(Match.tag(\"Ready\", handleReady));",
                path: "file.ts",
                code: "Match.value(value).pipe(Match.tag(\"Ready\", handleReady));",
                expected: &[],
            },
            Case { name: "valid 1: Ready.make({ value });", path: "file.ts", code: "Ready.make({ value });", expected: &[] },
            Case { name: "valid 2: new NotFound({ id });", path: "file.ts", code: "new NotFound({ id });", expected: &[] },
            Case {
                name: "invalid 3: const value = { _tag: \"Ready\", payload };",
                path: "file.ts",
                code: "const value = { _tag: \"Ready\", payload };",
                expected: &[(16, 29, "Use the existing Schema tagged `.make` or tagged class/error constructor instead of writing a literal `_tag` object.")],
            },
            Case {
                name: "invalid 4: const value = { [\"_tag\"]: \"Ready\" };",
                path: "file.ts",
                code: "const value = { [\"_tag\"]: \"Ready\" };",
                expected: &[(16, 33, "Use the existing Schema tagged `.make` or tagged class/error constructor instead of writing a literal `_tag` object.")],
            },
            Case {
                name: "invalid 5: const value = { _tag: tag, payload };",
                path: "file.ts",
                code: "const value = { _tag: tag, payload };",
                expected: &[(16, 25, "Use the existing Schema tagged `.make` or tagged class/error constructor instead of writing a literal `_tag` object.")],
            },
            Case {
                name: "invalid 6: const value = { _tag: \"Ready\" as const, payload };",
                path: "value.ts",
                code: "const value = { _tag: \"Ready\" as const, payload };",
                expected: &[(16, 38, "Use the existing Schema tagged `.make` or tagged class/error constructor instead of writing a literal `_tag` object.")],
            },
            Case {
                name: "invalid 7: Match.when({ _tag: \"Ready\" }, handleReady);",
                path: "file.ts",
                code: "Match.when({ _tag: \"Ready\" }, handleReady);",
                expected: &[(13, 26, "Use the existing Schema tagged `.make` or tagged class/error constructor instead of writing a literal `_tag` object.")],
            },
            Case {
                name: "invalid 8: Match.not({ \"_tag\": \"Pending\" });",
                path: "file.ts",
                code: "Match.not({ \"_tag\": \"Pending\" });",
                expected: &[(12, 29, "Use the existing Schema tagged `.make` or tagged class/error constructor instead of writing a literal `_tag` object.")],
            },
            Case {
                name: "invalid 9: class Value { readonly _tag = \"Ready\"; }",
                path: "value.ts",
                code: "class Value { readonly _tag = \"Ready\"; }",
                expected: &[(
                    14,
                    38,
                    "Declare tagged classes with Schema.TaggedClass, Schema.TaggedError, Data.TaggedClass, or Data.TaggedError instead of writing a literal `_tag` property.",
                )],
            },
        ],
    );
}

/// Edge cases the upstream tests miss, recorded from the upstream plugin.
#[test]
fn edges() {
    check(
        "anti-slop-effect/no-manual-tagged-construction",
        "[]",
        &[
            Case { name: "valid 0: const value = { [tag]: \"A\" };", path: "file.ts", code: "const value = { [tag]: \"A\" };", expected: &[] },
            Case { name: "valid 1: const { _tag } = value;", path: "file.ts", code: "const { _tag } = value;", expected: &[] },
            Case { name: "valid 2: class A { _tag() {} }", path: "file.ts", code: "class A { _tag() {} }", expected: &[] },
            Case { name: "valid 3: class A { accessor _tag = \"A\"; }", path: "file.ts", code: "class A { accessor _tag = \"A\"; }", expected: &[] },
            Case {
                name: "invalid 4: const value = { _tag: \"A\", other: { \"_tag\": \"B\" } };",
                path: "file.ts",
                code: "const value = { _tag: \"A\", other: { \"_tag\": \"B\" } };",
                expected: &[
                    (16, 25, "Use the existing Schema tagged `.make` or tagged class/error constructor instead of writing a literal `_tag` object."),
                    (36, 47, "Use the existing Schema tagged `.make` or tagged class/error constructor instead of writing a literal `_tag` object."),
                ],
            },
            Case {
                name: "invalid 5: const value = { [\"_tag\"]: \"A\" };",
                path: "file.ts",
                code: "const value = { [\"_tag\"]: \"A\" };",
                expected: &[(16, 29, "Use the existing Schema tagged `.make` or tagged class/error constructor instead of writing a literal `_tag` object.")],
            },
            Case {
                name: "invalid 6: class A { readonly _tag = \"A\" as const; static \"_tag\" = \"B\"",
                path: "file.ts",
                code: "class A { readonly _tag = \"A\" as const; static \"_tag\" = \"B\" }",
                expected: &[
                    (
                        10,
                        39,
                        "Declare tagged classes with Schema.TaggedClass, Schema.TaggedError, Data.TaggedClass, or Data.TaggedError instead of writing a literal `_tag` property.",
                    ),
                    (
                        40,
                        59,
                        "Declare tagged classes with Schema.TaggedClass, Schema.TaggedError, Data.TaggedClass, or Data.TaggedError instead of writing a literal `_tag` property.",
                    ),
                ],
            },
            Case {
                name: "invalid 7: const value = { _tag, _tag: \"B\" };",
                path: "file.ts",
                code: "const value = { _tag, _tag: \"B\" };",
                expected: &[(16, 20, "Use the existing Schema tagged `.make` or tagged class/error constructor instead of writing a literal `_tag` object.")],
            },
        ],
    );
}
