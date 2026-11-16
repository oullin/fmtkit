//! `anti-slop/no-conditional-spread`: expectations recorded from the upstream plugin
//! (github.com/k-mistele/anti-slop) under oxlint 1.86, one case per upstream
//! RuleTester case in `src/rules/no-conditional-spread.test.ts`.

use super::{Case, check};

/// src/rules/no-conditional-spread.test.ts
#[test]
fn upstream() {
    check(
        "anti-slop/no-conditional-spread",
        "[]",
        &[
            Case { name: "valid 0: const value = { ...base };", path: "file.tsx", code: "const value = { ...base };", expected: &[] },
            Case { name: "valid 1: const value = [...items];", path: "file.tsx", code: "const value = [...items];", expected: &[] },
            Case { name: "valid 2: fn(...args);", path: "file.tsx", code: "fn(...args);", expected: &[] },
            Case {
                name: "invalid 3: const value = { ...(condition ? left : right) };",
                path: "file.tsx",
                code: "const value = { ...(condition ? left : right) };",
                expected: &[(16, 45, "Conditional spreads obscure which values or properties are present. Build the value in separate statements instead.")],
            },
            Case {
                name: "invalid 4: const value = { ...(!condition ? undefined : fields) };",
                path: "file.tsx",
                code: "const value = { ...(!condition ? undefined : fields) };",
                expected: &[(16, 52, "Conditional spreads obscure which values or properties are present. Build the value in separate statements instead.")],
            },
            Case {
                name: "invalid 5: const value = { ...(condition && fields) };",
                path: "file.tsx",
                code: "const value = { ...(condition && fields) };",
                expected: &[(16, 40, "Conditional spreads obscure which values or properties are present. Build the value in separate statements instead.")],
            },
            Case {
                name: "invalid 6: const value = { ...(maybeValue ?? {}) };",
                path: "file.tsx",
                code: "const value = { ...(maybeValue ?? {}) };",
                expected: &[(16, 37, "Conditional spreads obscure which values or properties are present. Build the value in separate statements instead.")],
            },
            Case {
                name: "invalid 7: const value = { ...(maybeValue || {}) };",
                path: "file.tsx",
                code: "const value = { ...(maybeValue || {}) };",
                expected: &[(16, 37, "Conditional spreads obscure which values or properties are present. Build the value in separate statements instead.")],
            },
            Case {
                name: "invalid 8: const value = [...(condition ? items : [])];",
                path: "file.tsx",
                code: "const value = [...(condition ? items : [])];",
                expected: &[(15, 42, "Conditional spreads obscure which values or properties are present. Build the value in separate statements instead.")],
            },
            Case {
                name: "invalid 9: const value = [...(maybeItems ?? [])];",
                path: "file.tsx",
                code: "const value = [...(maybeItems ?? [])];",
                expected: &[(15, 36, "Conditional spreads obscure which values or properties are present. Build the value in separate statements instead.")],
            },
            Case {
                name: "invalid 10: fn(...(condition ? args : []));",
                path: "file.tsx",
                code: "fn(...(condition ? args : []));",
                expected: &[(3, 29, "Conditional spreads obscure which values or properties are present. Build the value in separate statements instead.")],
            },
            Case {
                name: "invalid 11: const value = { ...((condition && fields)) };",
                path: "file.tsx",
                code: "const value = { ...((condition && fields)) };",
                expected: &[(16, 42, "Conditional spreads obscure which values or properties are present. Build the value in separate statements instead.")],
            },
            Case {
                name: "invalid 12: const element = <Component {...(condition ? props : {})} /",
                path: "file.tsx",
                code: "const element = <Component {...(condition ? props : {})} />;",
                expected: &[(27, 56, "Conditional spreads obscure which values or properties are present. Build the value in separate statements instead.")],
            },
            Case {
                name: "invalid 13: const element = <Component {...(condition && props)} />;",
                path: "file.tsx",
                code: "const element = <Component {...(condition && props)} />;",
                expected: &[(27, 52, "Conditional spreads obscure which values or properties are present. Build the value in separate statements instead.")],
            },
            Case {
                name: "invalid 14: const element = <Component {...(maybeProps ?? {})} />;",
                path: "file.tsx",
                code: "const element = <Component {...(maybeProps ?? {})} />;",
                expected: &[(27, 50, "Conditional spreads obscure which values or properties are present. Build the value in separate statements instead.")],
            },
        ],
    );
}

/// Edge cases recorded from the upstream plugin under oxlint 1.86.
#[test]
fn edges() {
    check(
        "anti-slop/no-conditional-spread",
        "[]",
        &[
            Case {
                name: "edge 0: const value = { ...(a ? b : c as T) };",
                path: "file.tsx",
                code: "const value = { ...(a ? b : c as T) };",
                expected: &[(16, 35, "Conditional spreads obscure which values or properties are present. Build the value in separate statements instead.")],
            },
            Case {
                name: "edge 1: const value = { ...((a ? b : c) as T) };",
                path: "file.tsx",
                code: "const value = { ...((a ? b : c) as T) };",
                expected: &[],
            },
            Case {
                name: "edge 2: const value = { ...a ? b : c };",
                path: "file.tsx",
                code: "const value = { ...a ? b : c };",
                expected: &[(16, 28, "Conditional spreads obscure which values or properties are present. Build the value in separate statements instead.")],
            },
            Case {
                name: "edge 3: const value = new Foo(...(a ? b : c));",
                path: "file.tsx",
                code: "const value = new Foo(...(a ? b : c));",
                expected: &[(22, 36, "Conditional spreads obscure which values or properties are present. Build the value in separate statements instead.")],
            },
            Case { name: "edge 4: const { ...rest } = value;", path: "file.tsx", code: "const { ...rest } = value;", expected: &[] },
            Case { name: "edge 5: const value = { ...!a };", path: "file.tsx", code: "const value = { ...!a };", expected: &[] },
            Case { name: "edge 6: const value = { ...(a, b) };", path: "file.tsx", code: "const value = { ...(a, b) };", expected: &[] },
            Case { name: "edge 7: const value = { ...(a = b ?? c) };", path: "file.tsx", code: "const value = { ...(a = b ?? c) };", expected: &[] },
            Case { name: "edge 8: const value = { ...(a ?? b)! };", path: "file.tsx", code: "const value = { ...(a ?? b)! };", expected: &[] },
            Case {
                name: "edge 9: const element = <div {...(a || b)} />;",
                path: "file.tsx",
                code: "const element = <div {...(a || b)} />;",
                expected: &[(21, 34, "Conditional spreads obscure which values or properties are present. Build the value in separate statements instead.")],
            },
            Case { name: "edge 10: const element = <div {...props} />;", path: "file.tsx", code: "const element = <div {...props} />;", expected: &[] },
            Case {
                name: "edge 11: const value = [...(a ? b : c)];",
                path: "file.ts",
                code: "const value = [...(a ? b : c)];",
                expected: &[(15, 29, "Conditional spreads obscure which values or properties are present. Build the value in separate statements instead.")],
            },
            Case { name: "edge 12: const value = { ...f(a ? b : c) };", path: "file.tsx", code: "const value = { ...f(a ? b : c) };", expected: &[] },
        ],
    );
}
