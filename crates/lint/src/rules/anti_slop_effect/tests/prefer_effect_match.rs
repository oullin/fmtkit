//! `anti-slop-effect/prefer-effect-match`: expectations recorded from the upstream plugin
//! (github.com/k-mistele/anti-slop) under oxlint 1.86, one case per upstream
//! RuleTester case in `src/effect/rules/prefer-effect-match.test.ts`.

use super::super::super::anti_slop::tests::{Case, check};

/// src/effect/rules/prefer-effect-match.test.ts
#[test]
fn upstream() {
    check(
        "anti-slop-effect/prefer-effect-match",
        "[]",
        &[
            Case { name: "valid 0: kind === \"a\" ? first : fallback;", path: "file.ts", code: "kind === \"a\" ? first : fallback;", expected: &[] },
            Case {
                name: "valid 1: kind === \"a\" ? first : other === \"b\" ? second : fallback;",
                path: "file.ts",
                code: "kind === \"a\" ? first : other === \"b\" ? second : fallback;",
                expected: &[],
            },
            Case {
                name: "valid 2: condition ? first : otherCondition ? second : fallback;",
                path: "file.ts",
                code: "condition ? first : otherCondition ? second : fallback;",
                expected: &[],
            },
            Case {
                name: "invalid 3: kind === \"a\" ? first : kind === \"b\" ? second : fallback;",
                path: "file.ts",
                code: "kind === \"a\" ? first : kind === \"b\" ? second : fallback;",
                expected: &[(0, 55, "Use Match from Effect instead of a chained literal ternary.")],
            },
            Case {
                name: "invalid 4: `a` !== kind ? first : `b` === kind ? second : fallback;",
                path: "file.ts",
                code: "`a` !== kind ? first : `b` === kind ? second : fallback;",
                expected: &[(0, 55, "Use Match from Effect instead of a chained literal ternary.")],
            },
        ],
    );
}

/// Edge cases the upstream tests miss, recorded from the upstream plugin.
#[test]
fn edges() {
    check(
        "anti-slop-effect/prefer-effect-match",
        "[]",
        &[
            Case { name: "valid 0: kind === \"a\" ? first : fallback;", path: "file.ts", code: "kind === \"a\" ? first : fallback;", expected: &[] },
            Case {
                name: "valid 1: kind === \"a\" ? first : other === \"b\" ? second : fallback;",
                path: "file.ts",
                code: "kind === \"a\" ? first : other === \"b\" ? second : fallback;",
                expected: &[],
            },
            Case {
                name: "valid 2: kind === a ? first : kind === b ? second : fallback;",
                path: "file.ts",
                code: "kind === a ? first : kind === b ? second : fallback;",
                expected: &[],
            },
            Case {
                name: "valid 3: kind === `a${x}` ? first : kind === \"b\" ? second : fallback;",
                path: "file.ts",
                code: "kind === `a${x}` ? first : kind === \"b\" ? second : fallback;",
                expected: &[],
            },
            Case {
                name: "valid 4: kind > \"a\" ? first : kind > \"b\" ? second : fallback;",
                path: "file.ts",
                code: "kind > \"a\" ? first : kind > \"b\" ? second : fallback;",
                expected: &[],
            },
            Case {
                name: "invalid 5: (kind === \"a\") ? first : (kind === \"b\" ? second : fallback)",
                path: "file.ts",
                code: "(kind === \"a\") ? first : (kind === \"b\" ? second : fallback);",
                expected: &[(0, 59, "Use Match from Effect instead of a chained literal ternary.")],
            },
            Case {
                name: "invalid 6: kind == 1 ? first : kind != null ? second : kind === true ?",
                path: "file.ts",
                code: "kind == 1 ? first : kind != null ? second : kind === true ? third : fallback;",
                expected: &[(0, 76, "Use Match from Effect instead of a chained literal ternary.")],
            },
            Case {
                name: "invalid 7: value.kind === \"a\" ? first : value.kind === \"b\" ? second : ",
                path: "file.ts",
                code: "value.kind === \"a\" ? first : value.kind === \"b\" ? second : fallback;",
                expected: &[(0, 67, "Use Match from Effect instead of a chained literal ternary.")],
            },
            Case {
                name: "invalid 8: f(kind === \"a\" ? first : kind === \"b\" ? second : x === \"c\" ",
                path: "file.ts",
                code: "f(kind === \"a\" ? first : kind === \"b\" ? second : x === \"c\" ? y : z);",
                expected: &[],
            },
            Case {
                name: "invalid 9: kind === \"a\" ? (kind === \"b\" ? x : y) : kind === \"c\" ? seco",
                path: "file.ts",
                code: "kind === \"a\" ? (kind === \"b\" ? x : y) : kind === \"c\" ? second : fallback;",
                expected: &[(0, 72, "Use Match from Effect instead of a chained literal ternary.")],
            },
        ],
    );
}
