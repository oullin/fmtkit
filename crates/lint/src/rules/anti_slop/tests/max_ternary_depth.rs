//! `anti-slop/max-ternary-depth`: expectations recorded from the upstream plugin
//! (github.com/k-mistele/anti-slop) under oxlint 1.86, one case per upstream
//! RuleTester case in `src/rules/max-ternary-depth.test.ts`.

use super::{Case, check};

/// src/rules/max-ternary-depth.test.ts
#[test]
fn upstream() {
    check(
        "anti-slop/max-ternary-depth",
        "[]",
        &[
            Case { name: "valid 0: const value = condition ? left : right;", path: "file.ts", code: "const value = condition ? left : right;", expected: &[] },
            Case {
                name: "valid 1: const value = first ? one : second ? two : fallback;",
                path: "file.ts",
                code: "const value = first ? one : second ? two : fallback;",
                expected: &[],
            },
            Case {
                name: "invalid 3: const value = first ? one : second ? two : third ? three : ",
                path: "file.ts",
                code: "const value = first ? one : second ? two : third ? three : fallback;",
                expected: &[(14, 67, "This ternary has a depth of 3; the configured maximum is 2. Use a lookup table, switch, or named decision function.")],
            },
            Case {
                name: "invalid 4: const value = first ? (second ? (third ? three : other) : t",
                path: "file.ts",
                code: "const value = first ? (second ? (third ? three : other) : two) : fallback;",
                expected: &[(14, 73, "This ternary has a depth of 3; the configured maximum is 2. Use a lookup table, switch, or named decision function.")],
            },
            Case {
                name: "invalid 5: const value = first ? ((second ? two : third ? three : fall",
                path: "file.ts",
                code: "const value = first ? ((second ? two : third ? three : fallback) as string) : other;",
                expected: &[(14, 83, "This ternary has a depth of 3; the configured maximum is 2. Use a lookup table, switch, or named decision function.")],
            },
            Case {
                name: "invalid 6: const value = first ? (second ? two : third ? three : fallb",
                path: "file.ts",
                code: "const value = first ? (second ? two : third ? three : fallback)! : other;",
                expected: &[(14, 72, "This ternary has a depth of 3; the configured maximum is 2. Use a lookup table, switch, or named decision function.")],
            },
        ],
    );
}

/// src/rules/max-ternary-depth.test.ts, options `[{"maxDepth":3}]`
#[test]
fn upstream_options_1() {
    check(
        "anti-slop/max-ternary-depth",
        "[{\"maxDepth\":3}]",
        &[Case {
            name: "valid 2: const value = first ? one : second ? two : third ? three : fa",
            path: "file.ts",
            code: "const value = first ? one : second ? two : third ? three : fallback;",
            expected: &[],
        }],
    );
}

/// src/rules/max-ternary-depth.test.ts, options `[{"maxDepth":1}]`
#[test]
fn upstream_options_2() {
    check(
        "anti-slop/max-ternary-depth",
        "[{\"maxDepth\":1}]",
        &[Case {
            name: "invalid 7: const value = first ? one : second ? two : fallback;",
            path: "file.ts",
            code: "const value = first ? one : second ? two : fallback;",
            expected: &[(14, 51, "This ternary has a depth of 2; the configured maximum is 1. Use a lookup table, switch, or named decision function.")],
        }],
    );
}

/// Edge cases recorded from the upstream plugin under oxlint 1.86.
#[test]
fn edges() {
    check(
        "anti-slop/max-ternary-depth",
        "[]",
        &[
            Case {
                name: "edge 0: const value = (a ? b ? c : d : e) ? f : g ? h : i;",
                path: "file.ts",
                code: "const value = (a ? b ? c : d : e) ? f : g ? h : i;",
                expected: &[(14, 49, "This ternary has a depth of 3; the configured maximum is 2. Use a lookup table, switch, or named decision function.")],
            },
            Case {
                name: "edge 1: const value = a ? (b ? c : d) as T : e ? f : g;",
                path: "file.ts",
                code: "const value = a ? (b ? c : d) as T : e ? f : g;",
                expected: &[],
            },
            Case {
                name: "edge 2: const value = a ? f(b ? c : d ? e : g) : h;",
                path: "file.ts",
                code: "const value = a ? f(b ? c : d ? e : g) : h;",
                expected: &[],
            },
            Case {
                name: "edge 3: const value = a ? f(b ? c : d ? e : g ? i : j) : h;",
                path: "file.ts",
                code: "const value = a ? f(b ? c : d ? e : g ? i : j) : h;",
                expected: &[(20, 45, "This ternary has a depth of 3; the configured maximum is 2. Use a lookup table, switch, or named decision function.")],
            },
            Case {
                name: "edge 4: const value = (a ? b : c ? d : e ? f : g)!;",
                path: "file.ts",
                code: "const value = (a ? b : c ? d : e ? f : g)!;",
                expected: &[(15, 40, "This ternary has a depth of 3; the configured maximum is 2. Use a lookup table, switch, or named decision function.")],
            },
            Case {
                name: "edge 5: const value = a ? <T>(b ? c : d ? e : f) : g;",
                path: "file.ts",
                code: "const value = a ? <T>(b ? c : d ? e : f) : g;",
                expected: &[(14, 44, "This ternary has a depth of 3; the configured maximum is 2. Use a lookup table, switch, or named decision function.")],
            },
            Case {
                name: "edge 6: const value = a ? (b ? c : d ? e : f) satisfies T : g;",
                path: "file.ts",
                code: "const value = a ? (b ? c : d ? e : f) satisfies T : g;",
                expected: &[(14, 53, "This ternary has a depth of 3; the configured maximum is 2. Use a lookup table, switch, or named decision function.")],
            },
            Case {
                name: "edge 7: const value = a ? b : (c ? d : e ? f : g)!;",
                path: "file.ts",
                code: "const value = a ? b : (c ? d : e ? f : g)!;",
                expected: &[(14, 42, "This ternary has a depth of 3; the configured maximum is 2. Use a lookup table, switch, or named decision function.")],
            },
            Case { name: "edge 8: const value = a ? b : c;", path: "file.ts", code: "const value = a ? b : c;", expected: &[] },
            Case {
                name: "edge 9: function f() { return a ? (() => b ? c : d ? e : g) : h; }",
                path: "file.ts",
                code: "function f() { return a ? (() => b ? c : d ? e : g) : h; }",
                expected: &[],
            },
        ],
    );
}

/// Edge cases recorded from the upstream plugin under oxlint 1.86, options `[{"maxDepth":1}]`.
#[test]
fn edges_options_1() {
    check(
        "anti-slop/max-ternary-depth",
        "[{\"maxDepth\":1}]",
        &[Case { name: "edge 10: const value = a ? b : c;", path: "file.ts", code: "const value = a ? b : c;", expected: &[] }],
    );
}

/// Edge cases recorded from the upstream plugin under oxlint 1.86, options `[{"maxDepth":3}]`.
#[test]
fn edges_options_2() {
    check(
        "anti-slop/max-ternary-depth",
        "[{\"maxDepth\":3}]",
        &[Case {
            name: "edge 11: const value = a ? b : c ? d : e ? f : g ? h : i;",
            path: "file.ts",
            code: "const value = a ? b : c ? d : e ? f : g ? h : i;",
            expected: &[(14, 47, "This ternary has a depth of 4; the configured maximum is 3. Use a lookup table, switch, or named decision function.")],
        }],
    );
}

/// Edge cases recorded from the upstream plugin under oxlint 1.86, options `[{}]`.
#[test]
fn edges_options_3() {
    check(
        "anti-slop/max-ternary-depth",
        "[{}]",
        &[Case {
            name: "edge 12: const value = a ? b : c ? d : e ? f : g;",
            path: "file.ts",
            code: "const value = a ? b : c ? d : e ? f : g;",
            expected: &[(14, 39, "This ternary has a depth of 3; the configured maximum is 2. Use a lookup table, switch, or named decision function.")],
        }],
    );
}

/// Written by hand: options outside upstream's schema are rejected.
#[test]
fn rejects_invalid_options() {
    let factory = super::super::RULES.iter().find(|(name, _)| *name == "anti-slop/max-ternary-depth").map(|(_, factory)| *factory).expect("registered");

    for options in [r#"[{"maxDepth":0}]"#, r#"[{"maxDepth":1.5}]"#, r#"[{"maxDepth":"2"}]"#, r#"[{"depth":2}]"#, "[2]", "[{}, {}]"] {
        let options: Vec<serde_json::Value> = serde_json::from_str(options).expect("JSON");

        assert!(factory(&options).is_err(), "accepts {options:?}");
    }

    assert!(factory(&[serde_json::json!({ "maxDepth": 3.0 })]).is_ok(), "rejects an integral float");
}
