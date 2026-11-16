//! `anti-slop-martinfowler/mutable-data`: expectations recorded from the upstream plugin
//! (github.com/k-mistele/anti-slop) under oxlint 1.86, one case per upstream
//! RuleTester case in `src/martinfowler/rules/mutable-data.test.ts`.

use super::super::super::anti_slop::tests::{Case, check};

/// src/martinfowler/rules/mutable-data.test.ts
#[test]
fn upstream() {
    check(
        "anti-slop-martinfowler/mutable-data",
        "[]",
        &[
            Case {
                name: "valid 0: const count = 0; export function read() { return count; }",
                path: "file.ts",
                code: "const count = 0; export function read() { return count; }",
                expected: &[],
            },
            Case {
                name: "valid 1: let mode = 'a'; if (process.env.B) mode = 'b'; export const r",
                path: "file.ts",
                code: "let mode = 'a'; if (process.env.B) mode = 'b'; export const read = () => mode;",
                expected: &[],
            },
            Case {
                name: "valid 2: export function sum(xs) { let total = 0; for (const x of xs) ",
                path: "file.ts",
                code: "export function sum(xs) { let total = 0; for (const x of xs) total += x; return total; }",
                expected: &[],
            },
            Case {
                name: "valid 3: export let current = 0; export function bump() { current += 1",
                path: "file.ts",
                code: "export let current = 0; export function bump() { current += 1; }",
                expected: &[],
            },
            Case {
                name: "valid 4: const seen = new Set(); export function mark(id) { seen.add(i",
                path: "file.ts",
                code: "const seen = new Set(); export function mark(id) { seen.add(id); }",
                expected: &[],
            },
            Case {
                name: "invalid 5: let count = 0; export function bump() { count += 1; }",
                path: "file.ts",
                code: "let count = 0; export function bump() { count += 1; }",
                expected: &[(
                    4,
                    9,
                    "Possibly Mutable Data: module-level `count` is reassigned inside 1 function, so its value depends on which functions ran before and in what order. Consider Encapsulate Variable so one owner controls every update, or Replace Derived Variable with Query if it can be computed from other data (in Effect, a `Ref` owned by a service, or a value derived where it is needed).",
                )],
            },
            Case {
                name: "invalid 6: let cached; export const get = () => { cached ??= load(); r",
                path: "file.ts",
                code: "let cached; export const get = () => { cached ??= load(); return cached; }; export function reset() { cached = undefined; }",
                expected: &[(
                    4,
                    10,
                    "Possibly Mutable Data: module-level `cached` is reassigned inside 2 functions, so its value depends on which functions ran before and in what order. Consider Encapsulate Variable so one owner controls every update, or Replace Derived Variable with Query if it can be computed from other data (in Effect, a `Ref` owned by a service, or a value derived where it is needed).",
                )],
            },
            Case {
                name: "invalid 7: var listeners = []; export function on(fn) { listeners = [.",
                path: "file.ts",
                code: "var listeners = []; export function on(fn) { listeners = [...listeners, fn]; }",
                expected: &[(
                    4,
                    13,
                    "Possibly Mutable Data: module-level `listeners` is reassigned inside 1 function, so its value depends on which functions ran before and in what order. Consider Encapsulate Variable so one owner controls every update, or Replace Derived Variable with Query if it can be computed from other data (in Effect, a `Ref` owned by a service, or a value derived where it is needed).",
                )],
            },
            Case {
                name: "invalid 8: let id = 0; export const next = () => id++;",
                path: "file.ts",
                code: "let id = 0; export const next = () => id++;",
                expected: &[(
                    4,
                    6,
                    "Possibly Mutable Data: module-level `id` is reassigned inside 1 function, so its value depends on which functions ran before and in what order. Consider Encapsulate Variable so one owner controls every update, or Replace Derived Variable with Query if it can be computed from other data (in Effect, a `Ref` owned by a service, or a value derived where it is needed).",
                )],
            },
            Case {
                name: "invalid 9: let a = 0, b = 0; export function swap() { [a, b] = [b, a];",
                path: "file.ts",
                code: "let a = 0, b = 0; export function swap() { [a, b] = [b, a]; }",
                expected: &[
                    (
                        4,
                        5,
                        "Possibly Mutable Data: module-level `a` is reassigned inside 1 function, so its value depends on which functions ran before and in what order. Consider Encapsulate Variable so one owner controls every update, or Replace Derived Variable with Query if it can be computed from other data (in Effect, a `Ref` owned by a service, or a value derived where it is needed).",
                    ),
                    (
                        11,
                        12,
                        "Possibly Mutable Data: module-level `b` is reassigned inside 1 function, so its value depends on which functions ran before and in what order. Consider Encapsulate Variable so one owner controls every update, or Replace Derived Variable with Query if it can be computed from other data (in Effect, a `Ref` owned by a service, or a value derived where it is needed).",
                    ),
                ],
            },
        ],
    );
}

/// Edge cases recorded from upstream under oxlint 1.86: src/martinfowler/rules/mutable-data.mfedges.test.ts
#[test]
fn edges() {
    check(
        "anti-slop-martinfowler/mutable-data",
        "[]",
        &[
            Case {
                name: "invalid 0: let x: number = 0; export function f() { x = 1; }",
                path: "file.ts",
                code: "let x: number = 0; export function f() { x = 1; }",
                expected: &[(
                    4,
                    13,
                    "Possibly Mutable Data: module-level `x` is reassigned inside 1 function, so its value depends on which functions ran before and in what order. Consider Encapsulate Variable so one owner controls every update, or Replace Derived Variable with Query if it can be computed from other data (in Effect, a `Ref` owned by a service, or a value derived where it is needed).",
                )],
            },
            Case {
                name: "invalid 1: let { a, b: [c] } = o; function f() { a = 1; c = 2; }",
                path: "file.ts",
                code: "let { a, b: [c] } = o; function f() { a = 1; c = 2; }",
                expected: &[
                    (
                        6,
                        7,
                        "Possibly Mutable Data: module-level `a` is reassigned inside 1 function, so its value depends on which functions ran before and in what order. Consider Encapsulate Variable so one owner controls every update, or Replace Derived Variable with Query if it can be computed from other data (in Effect, a `Ref` owned by a service, or a value derived where it is needed).",
                    ),
                    (
                        13,
                        14,
                        "Possibly Mutable Data: module-level `c` is reassigned inside 1 function, so its value depends on which functions ran before and in what order. Consider Encapsulate Variable so one owner controls every update, or Replace Derived Variable with Query if it can be computed from other data (in Effect, a `Ref` owned by a service, or a value derived where it is needed).",
                    ),
                ],
            },
            Case {
                name: "invalid 2: var x = 1; var x = 2; function f() { x = 3; }",
                path: "file.ts",
                code: "var x = 1; var x = 2; function f() { x = 3; }",
                expected: &[
                    (
                        4,
                        5,
                        "Possibly Mutable Data: module-level `x` is reassigned inside 1 function, so its value depends on which functions ran before and in what order. Consider Encapsulate Variable so one owner controls every update, or Replace Derived Variable with Query if it can be computed from other data (in Effect, a `Ref` owned by a service, or a value derived where it is needed).",
                    ),
                    (
                        4,
                        5,
                        "Possibly Mutable Data: module-level `x` is reassigned inside 1 function, so its value depends on which functions ran before and in what order. Consider Encapsulate Variable so one owner controls every update, or Replace Derived Variable with Query if it can be computed from other data (in Effect, a `Ref` owned by a service, or a value derived where it is needed).",
                    ),
                ],
            },
            Case {
                name: "invalid 3: let x = 0; const f = () => { x++; }; class A { m() { x = 1;",
                path: "file.ts",
                code: "let x = 0; const f = () => { x++; }; class A { m() { x = 1; } }",
                expected: &[(
                    4,
                    5,
                    "Possibly Mutable Data: module-level `x` is reassigned inside 2 functions, so its value depends on which functions ran before and in what order. Consider Encapsulate Variable so one owner controls every update, or Replace Derived Variable with Query if it can be computed from other data (in Effect, a `Ref` owned by a service, or a value derived where it is needed).",
                )],
            },
            Case { name: "invalid 4: let x = 0; x = 1;", path: "file.ts", code: "let x = 0; x = 1;", expected: &[] },
            Case {
                name: "invalid 5: let x; function f() { for (x of xs) {} }",
                path: "file.ts",
                code: "let x; function f() { for (x of xs) {} }",
                expected: &[(
                    4,
                    5,
                    "Possibly Mutable Data: module-level `x` is reassigned inside 1 function, so its value depends on which functions ran before and in what order. Consider Encapsulate Variable so one owner controls every update, or Replace Derived Variable with Query if it can be computed from other data (in Effect, a `Ref` owned by a service, or a value derived where it is needed).",
                )],
            },
            Case {
                name: "invalid 6: let x; function f() { ({ x } = o); }",
                path: "file.ts",
                code: "let x; function f() { ({ x } = o); }",
                expected: &[(
                    4,
                    5,
                    "Possibly Mutable Data: module-level `x` is reassigned inside 1 function, so its value depends on which functions ran before and in what order. Consider Encapsulate Variable so one owner controls every update, or Replace Derived Variable with Query if it can be computed from other data (in Effect, a `Ref` owned by a service, or a value derived where it is needed).",
                )],
            },
            Case {
                name: "invalid 7: let x = 0; function f() { let x = 1; x = 2; }",
                path: "file.ts",
                code: "let x = 0; function f() { let x = 1; x = 2; }",
                expected: &[],
            },
            Case {
                name: "invalid 8: let x = 0; function f() { x = 1; x = 2; } function g() { co",
                path: "file.ts",
                code: "let x = 0; function f() { x = 1; x = 2; } function g() { const h = () => { x--; }; }",
                expected: &[(
                    4,
                    5,
                    "Possibly Mutable Data: module-level `x` is reassigned inside 2 functions, so its value depends on which functions ran before and in what order. Consider Encapsulate Variable so one owner controls every update, or Replace Derived Variable with Query if it can be computed from other data (in Effect, a `Ref` owned by a service, or a value derived where it is needed).",
                )],
            },
            Case { name: "invalid 9: let x = 0; function f() { return x; }", path: "file.ts", code: "let x = 0; function f() { return x; }", expected: &[] },
            Case { name: "invalid 10: let x = {}; function f() { x.y = 1; }", path: "file.ts", code: "let x = {}; function f() { x.y = 1; }", expected: &[] },
            Case { name: "invalid 11: const x = 0; function f() { x = 1; }", path: "file.ts", code: "const x = 0; function f() { x = 1; }", expected: &[] },
            Case {
                name: "invalid 12: let x = 0; class A { static { x = 1; } }",
                path: "file.ts",
                code: "let x = 0; class A { static { x = 1; } }",
                expected: &[],
            },
            Case { name: "invalid 13: let x = 0; class A { p = (x = 1); }", path: "file.ts", code: "let x = 0; class A { p = (x = 1); }", expected: &[] },
            Case {
                name: "invalid 14: let x = 0; function f(a = (x = 1)) {}",
                path: "file.ts",
                code: "let x = 0; function f(a = (x = 1)) {}",
                expected: &[(
                    4,
                    5,
                    "Possibly Mutable Data: module-level `x` is reassigned inside 1 function, so its value depends on which functions ran before and in what order. Consider Encapsulate Variable so one owner controls every update, or Replace Derived Variable with Query if it can be computed from other data (in Effect, a `Ref` owned by a service, or a value derived where it is needed).",
                )],
            },
            Case {
                name: "invalid 15: let x: number; function f() { (x as any) = 1; }",
                path: "file.ts",
                code: "let x: number; function f() { (x as any) = 1; }",
                expected: &[(
                    4,
                    13,
                    "Possibly Mutable Data: module-level `x` is reassigned inside 1 function, so its value depends on which functions ran before and in what order. Consider Encapsulate Variable so one owner controls every update, or Replace Derived Variable with Query if it can be computed from other data (in Effect, a `Ref` owned by a service, or a value derived where it is needed).",
                )],
            },
            Case {
                name: "invalid 16: let x: number; function f() { x! = 1; }",
                path: "file.ts",
                code: "let x: number; function f() { x! = 1; }",
                expected: &[(
                    4,
                    13,
                    "Possibly Mutable Data: module-level `x` is reassigned inside 1 function, so its value depends on which functions ran before and in what order. Consider Encapsulate Variable so one owner controls every update, or Replace Derived Variable with Query if it can be computed from other data (in Effect, a `Ref` owned by a service, or a value derived where it is needed).",
                )],
            },
            Case {
                name: "invalid 17: let x = 0; function f() { x &&= 1; x ||= 2; }",
                path: "file.ts",
                code: "let x = 0; function f() { x &&= 1; x ||= 2; }",
                expected: &[(
                    4,
                    5,
                    "Possibly Mutable Data: module-level `x` is reassigned inside 1 function, so its value depends on which functions ran before and in what order. Consider Encapsulate Variable so one owner controls every update, or Replace Derived Variable with Query if it can be computed from other data (in Effect, a `Ref` owned by a service, or a value derived where it is needed).",
                )],
            },
            Case {
                name: "invalid 18: let x = 0; function f() { for (x in o) {} }",
                path: "file.ts",
                code: "let x = 0; function f() { for (x in o) {} }",
                expected: &[(
                    4,
                    5,
                    "Possibly Mutable Data: module-level `x` is reassigned inside 1 function, so its value depends on which functions ran before and in what order. Consider Encapsulate Variable so one owner controls every update, or Replace Derived Variable with Query if it can be computed from other data (in Effect, a `Ref` owned by a service, or a value derived where it is needed).",
                )],
            },
            Case {
                name: "invalid 19: declare let x: number; function f() { x = 1; }",
                path: "file.ts",
                code: "declare let x: number; function f() { x = 1; }",
                expected: &[(
                    12,
                    21,
                    "Possibly Mutable Data: module-level `x` is reassigned inside 1 function, so its value depends on which functions ran before and in what order. Consider Encapsulate Variable so one owner controls every update, or Replace Derived Variable with Query if it can be computed from other data (in Effect, a `Ref` owned by a service, or a value derived where it is needed).",
                )],
            },
            Case { name: "invalid 20: let x = 0; function f() { x = 1; }", path: "src/a.spec.ts", code: "let x = 0; function f() { x = 1; }", expected: &[] },
        ],
    );
}

/// Edge cases recorded from upstream under oxlint 1.86: src/martinfowler/rules/mutable-data.mfedges.test.ts, options `[{"ignoreTests":false}]`
#[test]
fn edges_options_1() {
    check(
        "anti-slop-martinfowler/mutable-data",
        "[{\"ignoreTests\":false}]",
        &[Case {
            name: "invalid 21: let x = 0; function f() { x = 1; }",
            path: "src/a.spec.ts",
            code: "let x = 0; function f() { x = 1; }",
            expected: &[(
                4,
                5,
                "Possibly Mutable Data: module-level `x` is reassigned inside 1 function, so its value depends on which functions ran before and in what order. Consider Encapsulate Variable so one owner controls every update, or Replace Derived Variable with Query if it can be computed from other data (in Effect, a `Ref` owned by a service, or a value derived where it is needed).",
            )],
        }],
    );
}
