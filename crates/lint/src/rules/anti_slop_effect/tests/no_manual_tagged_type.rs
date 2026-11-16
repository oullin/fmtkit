//! `anti-slop-effect/no-manual-tagged-type`: expectations recorded from the upstream plugin
//! (github.com/k-mistele/anti-slop) under oxlint 1.86, one case per upstream
//! RuleTester case in `src/effect/rules/no-manual-tagged-type.test.ts`.

use super::super::super::anti_slop::tests::{Case, check};

/// src/effect/rules/no-manual-tagged-type.test.ts
#[test]
fn upstream() {
    check(
        "anti-slop-effect/no-manual-tagged-type",
        "[]",
        &[
            Case {
                name: "valid 0: const Ready = Schema.TaggedStruct(\"Ready\", { value: Schema.St",
                path: "value.ts",
                code: "const Ready = Schema.TaggedStruct(\"Ready\", { value: Schema.String });\ntype Ready = typeof Ready.Type;",
                expected: &[],
            },
            Case {
                name: "valid 1: type Event = Data.TaggedEnum<{ Ready: { readonly value: strin",
                path: "value.ts",
                code: "type Event = Data.TaggedEnum<{ Ready: { readonly value: string } }>;",
                expected: &[],
            },
            Case { name: "valid 2: type Value = { readonly tag: string };", path: "value.ts", code: "type Value = { readonly tag: string };", expected: &[] },
            Case { name: "valid 3: type ValueTag = Value[\"_tag\"];", path: "value.ts", code: "type ValueTag = Value[\"_tag\"];", expected: &[] },
            Case {
                name: "invalid 4: type Value = { readonly _tag: \"Ready\"; readonly payload: st",
                path: "value.ts",
                code: "type Value = { readonly _tag: \"Ready\"; readonly payload: string };",
                expected: &[(
                    15,
                    38,
                    "Do not declare `_tag` by hand. Define the value with Schema.TaggedStruct, Schema.TaggedClass, Schema.TaggedError, Schema.TaggedUnion, Data.TaggedClass, Data.TaggedError, or Data.taggedEnum, then infer the type from it, e.g. `export type X = typeof X.Type`.",
                )],
            },
            Case {
                name: "invalid 5: type Value = { readonly _tag: \"Ready\" } | { readonly _tag: ",
                path: "value.ts",
                code: "type Value = { readonly _tag: \"Ready\" } | { readonly _tag: \"Done\" };",
                expected: &[
                    (
                        15,
                        37,
                        "Do not declare `_tag` by hand. Define the value with Schema.TaggedStruct, Schema.TaggedClass, Schema.TaggedError, Schema.TaggedUnion, Data.TaggedClass, Data.TaggedError, or Data.taggedEnum, then infer the type from it, e.g. `export type X = typeof X.Type`.",
                    ),
                    (
                        44,
                        65,
                        "Do not declare `_tag` by hand. Define the value with Schema.TaggedStruct, Schema.TaggedClass, Schema.TaggedError, Schema.TaggedUnion, Data.TaggedClass, Data.TaggedError, or Data.taggedEnum, then infer the type from it, e.g. `export type X = typeof X.Type`.",
                    ),
                ],
            },
            Case {
                name: "invalid 6: interface Value { readonly \"_tag\": \"Ready\"; }",
                path: "value.ts",
                code: "interface Value { readonly \"_tag\": \"Ready\"; }",
                expected: &[(
                    18,
                    43,
                    "Do not declare `_tag` by hand. Define the value with Schema.TaggedStruct, Schema.TaggedClass, Schema.TaggedError, Schema.TaggedUnion, Data.TaggedClass, Data.TaggedError, or Data.taggedEnum, then infer the type from it, e.g. `export type X = typeof X.Type`.",
                )],
            },
            Case {
                name: "invalid 7: interface Authored { readonly _tag: string; readonly actor:",
                path: "value.ts",
                code: "interface Authored { readonly _tag: string; readonly actor: string }",
                expected: &[(
                    21,
                    43,
                    "Do not declare `_tag` by hand. Define the value with Schema.TaggedStruct, Schema.TaggedClass, Schema.TaggedError, Schema.TaggedUnion, Data.TaggedClass, Data.TaggedError, or Data.taggedEnum, then infer the type from it, e.g. `export type X = typeof X.Type`.",
                )],
            },
            Case {
                name: "invalid 8: const tagOf = <A extends { readonly _tag: string }>(value: ",
                path: "value.ts",
                code: "const tagOf = <A extends { readonly _tag: string }>(value: A) => value;",
                expected: &[(
                    27,
                    48,
                    "Do not declare `_tag` by hand. Define the value with Schema.TaggedStruct, Schema.TaggedClass, Schema.TaggedError, Schema.TaggedUnion, Data.TaggedClass, Data.TaggedError, or Data.taggedEnum, then infer the type from it, e.g. `export type X = typeof X.Type`.",
                )],
            },
            Case {
                name: "invalid 9: const handle = (event: { readonly _tag: string }) => event;",
                path: "value.ts",
                code: "const handle = (event: { readonly _tag: string }) => event;",
                expected: &[(
                    25,
                    46,
                    "Do not declare `_tag` by hand. Define the value with Schema.TaggedStruct, Schema.TaggedClass, Schema.TaggedError, Schema.TaggedUnion, Data.TaggedClass, Data.TaggedError, or Data.taggedEnum, then infer the type from it, e.g. `export type X = typeof X.Type`.",
                )],
            },
        ],
    );
}

/// Edge cases the upstream tests miss, recorded from the upstream plugin.
#[test]
fn edges() {
    check(
        "anti-slop-effect/no-manual-tagged-type",
        "[]",
        &[
            Case { name: "valid 0: type A = { tag: \"A\" };", path: "file.ts", code: "type A = { tag: \"A\" };", expected: &[] },
            Case { name: "valid 1: interface A { _tag(): void }", path: "file.ts", code: "interface A { _tag(): void }", expected: &[] },
            Case { name: "valid 2: type A = { [key: string]: \"A\" };", path: "file.ts", code: "type A = { [key: string]: \"A\" };", expected: &[] },
            Case {
                name: "invalid 3: type A = { readonly _tag: \"A\"; } | { \"_tag\": \"B\" };",
                path: "file.ts",
                code: "type A = { readonly _tag: \"A\"; } | { \"_tag\": \"B\" };",
                expected: &[
                    (
                        11,
                        30,
                        "Do not declare `_tag` by hand. Define the value with Schema.TaggedStruct, Schema.TaggedClass, Schema.TaggedError, Schema.TaggedUnion, Data.TaggedClass, Data.TaggedError, or Data.taggedEnum, then infer the type from it, e.g. `export type X = typeof X.Type`.",
                    ),
                    (
                        37,
                        48,
                        "Do not declare `_tag` by hand. Define the value with Schema.TaggedStruct, Schema.TaggedClass, Schema.TaggedError, Schema.TaggedUnion, Data.TaggedClass, Data.TaggedError, or Data.taggedEnum, then infer the type from it, e.g. `export type X = typeof X.Type`.",
                    ),
                ],
            },
            Case {
                name: "invalid 4: interface A { _tag?: \"A\", [\"_tag\"]: \"B\" }",
                path: "file.ts",
                code: "interface A { _tag?: \"A\", [\"_tag\"]: \"B\" }",
                expected: &[
                    (
                        14,
                        25,
                        "Do not declare `_tag` by hand. Define the value with Schema.TaggedStruct, Schema.TaggedClass, Schema.TaggedError, Schema.TaggedUnion, Data.TaggedClass, Data.TaggedError, or Data.taggedEnum, then infer the type from it, e.g. `export type X = typeof X.Type`.",
                    ),
                    (
                        26,
                        39,
                        "Do not declare `_tag` by hand. Define the value with Schema.TaggedStruct, Schema.TaggedClass, Schema.TaggedError, Schema.TaggedUnion, Data.TaggedClass, Data.TaggedError, or Data.taggedEnum, then infer the type from it, e.g. `export type X = typeof X.Type`.",
                    ),
                ],
            },
            Case {
                name: "invalid 5: declare function f(input: { _tag: \"A\" }): void;",
                path: "file.ts",
                code: "declare function f(input: { _tag: \"A\" }): void;",
                expected: &[(
                    28,
                    37,
                    "Do not declare `_tag` by hand. Define the value with Schema.TaggedStruct, Schema.TaggedClass, Schema.TaggedError, Schema.TaggedUnion, Data.TaggedClass, Data.TaggedError, or Data.taggedEnum, then infer the type from it, e.g. `export type X = typeof X.Type`.",
                )],
            },
        ],
    );
}
