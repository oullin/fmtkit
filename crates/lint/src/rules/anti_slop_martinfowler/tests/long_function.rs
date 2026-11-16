//! `anti-slop-martinfowler/long-function`: expectations recorded from the upstream plugin
//! (github.com/k-mistele/anti-slop) under oxlint 1.86, one case per upstream
//! RuleTester case in `src/martinfowler/rules/long-function.test.ts`.

use super::super::super::anti_slop::tests::{Case, check};

/// src/martinfowler/rules/long-function.test.ts
#[test]
fn upstream() {
    check(
        "anti-slop-martinfowler/long-function",
        "[]",
        &[
            Case {
                name: "valid 0: function short() {",
                path: "file.ts",
                code: "function short() {\nstep0();\nstep1();\nstep2();\nstep3();\nstep4();\nstep5();\nstep6();\nstep7();\nstep8();\nstep9();\nstep10();\nstep11();\nstep12();\nstep13();\nstep14();\nstep15();\nstep16();\nstep17();\nstep18();\nstep19();\nstep20();\nstep21();\nstep22();\nstep23();\nstep24();\nstep25();\nstep26();\nstep27();\nstep28();\nstep29();\n}",
                expected: &[],
            },
            Case {
                name: "valid 1: function outer() {",
                path: "file.ts",
                code: "function outer() {\n  const inner = () => {\nstep0();\nstep1();\nstep2();\nstep3();\nstep4();\nstep5();\nstep6();\nstep7();\nstep8();\nstep9();\nstep10();\nstep11();\nstep12();\nstep13();\nstep14();\nstep15();\nstep16();\nstep17();\nstep18();\nstep19();\nstep20();\nstep21();\nstep22();\nstep23();\nstep24();\n  };\n  const other = () => {\nstep0();\nstep1();\nstep2();\nstep3();\nstep4();\nstep5();\nstep6();\nstep7();\nstep8();\nstep9();\nstep10();\nstep11();\nstep12();\nstep13();\nstep14();\nstep15();\nstep16();\nstep17();\nstep18();\nstep19();\nstep20();\nstep21();\nstep22();\nstep23();\nstep24();\n  };\n  return [inner, other];\n}",
                expected: &[],
            },
            Case {
                name: "valid 2: const pick = (value) => value.a + value.b;",
                path: "file.ts",
                code: "const pick = (value) => value.a + value.b;",
                expected: &[],
            },
            Case {
                name: "invalid 4: function long() {",
                path: "file.ts",
                code: "function long() {\nstep0();\nstep1();\nstep2();\nstep3();\nstep4();\nstep5();\nstep6();\nstep7();\nstep8();\nstep9();\nstep10();\nstep11();\nstep12();\nstep13();\nstep14();\nstep15();\nstep16();\nstep17();\nstep18();\nstep19();\nstep20();\nstep21();\nstep22();\nstep23();\nstep24();\nstep25();\nstep26();\nstep27();\nstep28();\nstep29();\nstep30();\n}",
                expected: &[(
                    9,
                    13,
                    "Possibly Long Function: `long` has 31 statements over 33 lines of its own (limits 30 statements, 80 lines), so a reader must hold every step in mind at once. Consider Extract Function: move each step into a function named for what it does (in Effect, a named `Effect.fn(\"domain.operation\")` step that this function yields).",
                )],
            },
        ],
    );
}

/// src/martinfowler/rules/long-function.test.ts, options `[{"maxStatements":10}]`
#[test]
fn upstream_options_1() {
    check(
        "anti-slop-martinfowler/long-function",
        "[{\"maxStatements\":10}]",
        &[
            Case {
                name: "valid 3: function short() {",
                path: "file.ts",
                code: "function short() {\nstep0();\nstep1();\nstep2();\nstep3();\nstep4();\nstep5();\nstep6();\nstep7();\nstep8();\nstep9();\n}",
                expected: &[],
            },
            Case {
                name: "invalid 5: export const make = Effect.fn(\"users.make\")(function* () {",
                path: "file.ts",
                code: "export const make = Effect.fn(\"users.make\")(function* () {\nstep0();\nstep1();\nstep2();\nstep3();\nstep4();\nstep5();\nstep6();\nstep7();\nstep8();\nstep9();\nstep10();\nstep11();\n});",
                expected: &[(
                    44,
                    170,
                    "Possibly Long Function: `users.make` has 12 statements over 14 lines of its own (limits 10 statements, 80 lines), so a reader must hold every step in mind at once. Consider Extract Function: move each step into a function named for what it does (in Effect, a named `Effect.fn(\"domain.operation\")` step that this function yields).",
                )],
            },
        ],
    );
}

/// src/martinfowler/rules/long-function.test.ts, options `[{"maxLines":8}]`
#[test]
fn upstream_options_2() {
    check(
        "anti-slop-martinfowler/long-function",
        "[{\"maxLines\":8}]",
        &[Case {
            name: "invalid 6: const spread = () => {",
            path: "file.ts",
            code: "const spread = () => {\n  a();\n\n\n\n\n\n\n\n\n\n\n  b();\n};",
            expected: &[(
                6,
                12,
                "Possibly Long Function: `spread` has 2 statements over 14 lines of its own (limits 30 statements, 8 lines), so a reader must hold every step in mind at once. Consider Extract Function: move each step into a function named for what it does (in Effect, a named `Effect.fn(\"domain.operation\")` step that this function yields).",
            )],
        }],
    );
}

/// src/martinfowler/rules/long-function.test.ts, options `[{"maxStatements":3}]`
#[test]
fn upstream_options_3() {
    check(
        "anti-slop-martinfowler/long-function",
        "[{\"maxStatements\":3}]",
        &[Case {
            name: "invalid 7: items.forEach(function () {",
            path: "file.ts",
            code: "items.forEach(function () {\nstep0();\nstep1();\nstep2();\nstep3();\n});",
            expected: &[(
                14,
                65,
                "Possibly Long Function: This function has 4 statements over 6 lines of its own (limits 3 statements, 80 lines), so a reader must hold every step in mind at once. Consider Extract Function: move each step into a function named for what it does (in Effect, a named `Effect.fn(\"domain.operation\")` step that this function yields).",
            )],
        }],
    );
}

/// Edge cases recorded from upstream under oxlint 1.86: src/martinfowler/rules/long-function.mfedges.test.ts, options `[{"maxStatements":1,"maxLines":2}]`
#[test]
fn edges() {
    check(
        "anti-slop-martinfowler/long-function",
        "[{\"maxStatements\":1,\"maxLines\":2}]",
        &[
            Case {
                name: "invalid 0: function f() {\r",
                path: "file.ts",
                code: "function f() {\r\na();\r\n}",
                expected: &[(
                    9,
                    10,
                    "Possibly Long Function: `f` has 1 statements over 3 lines of its own (limits 1 statements, 2 lines), so a reader must hold every step in mind at once. Consider Extract Function: move each step into a function named for what it does (in Effect, a named `Effect.fn(\"domain.operation\")` step that this function yields).",
                )],
            },
            Case {
                name: "invalid 1: function f() {\ra();\r}",
                path: "file.ts",
                code: "function f() {\ra();\r}",
                expected: &[(
                    9,
                    10,
                    "Possibly Long Function: `f` has 1 statements over 3 lines of its own (limits 1 statements, 2 lines), so a reader must hold every step in mind at once. Consider Extract Function: move each step into a function named for what it does (in Effect, a named `Effect.fn(\"domain.operation\")` step that this function yields).",
                )],
            },
            Case {
                name: "invalid 2: function f() { a(); }",
                path: "file.ts",
                code: "function f() { a(); }",
                expected: &[(
                    9,
                    10,
                    "Possibly Long Function: `f` has 1 statements over 3 lines of its own (limits 1 statements, 2 lines), so a reader must hold every step in mind at once. Consider Extract Function: move each step into a function named for what it does (in Effect, a named `Effect.fn(\"domain.operation\")` step that this function yields).",
                )],
            },
            Case { name: "invalid 3: function f() { const s = ' '; }", path: "file.ts", code: "function f() { const s = ' '; }", expected: &[] },
            Case {
                name: "invalid 4: function f() { 'use strict'; a(); }",
                path: "file.ts",
                code: "function f() { 'use strict'; a(); }",
                expected: &[(
                    9,
                    10,
                    "Possibly Long Function: `f` has 2 statements over 1 lines of its own (limits 1 statements, 2 lines), so a reader must hold every step in mind at once. Consider Extract Function: move each step into a function named for what it does (in Effect, a named `Effect.fn(\"domain.operation\")` step that this function yields).",
                )],
            },
            Case {
                name: "invalid 5: function f() { for (let i = 0; i < 1; i++) {} }",
                path: "file.ts",
                code: "function f() { for (let i = 0; i < 1; i++) {} }",
                expected: &[(
                    9,
                    10,
                    "Possibly Long Function: `f` has 2 statements over 1 lines of its own (limits 1 statements, 2 lines), so a reader must hold every step in mind at once. Consider Extract Function: move each step into a function named for what it does (in Effect, a named `Effect.fn(\"domain.operation\")` step that this function yields).",
                )],
            },
            Case {
                name: "invalid 6: function f() { for (const x of xs) ; }",
                path: "file.ts",
                code: "function f() { for (const x of xs) ; }",
                expected: &[(
                    9,
                    10,
                    "Possibly Long Function: `f` has 2 statements over 1 lines of its own (limits 1 statements, 2 lines), so a reader must hold every step in mind at once. Consider Extract Function: move each step into a function named for what it does (in Effect, a named `Effect.fn(\"domain.operation\")` step that this function yields).",
                )],
            },
            Case {
                name: "invalid 7: function f() { class A { m() { a(); b(); } } }",
                path: "file.ts",
                code: "function f() { class A { m() { a(); b(); } } }",
                expected: &[(
                    25,
                    26,
                    "Possibly Long Function: `m` has 2 statements over 1 lines of its own (limits 1 statements, 2 lines), so a reader must hold every step in mind at once. Consider Extract Function: move each step into a function named for what it does (in Effect, a named `Effect.fn(\"domain.operation\")` step that this function yields).",
                )],
            },
            Case {
                name: "invalid 11: function f() {",
                path: "file.ts",
                code: "function f() {\n  function g() {\n  }\n  a();\n}",
                expected: &[(
                    9,
                    10,
                    "Possibly Long Function: `f` has 2 statements over 4 lines of its own (limits 1 statements, 2 lines), so a reader must hold every step in mind at once. Consider Extract Function: move each step into a function named for what it does (in Effect, a named `Effect.fn(\"domain.operation\")` step that this function yields).",
                )],
            },
            Case {
                name: "invalid 12: function f() { label: for (;;) { break label; } }",
                path: "file.ts",
                code: "function f() { label: for (;;) { break label; } }",
                expected: &[(
                    9,
                    10,
                    "Possibly Long Function: `f` has 3 statements over 1 lines of its own (limits 1 statements, 2 lines), so a reader must hold every step in mind at once. Consider Extract Function: move each step into a function named for what it does (in Effect, a named `Effect.fn(\"domain.operation\")` step that this function yields).",
                )],
            },
            Case {
                name: "invalid 13: function f() { try { a(); } catch { b(); } finally { c(); ",
                path: "file.ts",
                code: "function f() { try { a(); } catch { b(); } finally { c(); } }",
                expected: &[(
                    9,
                    10,
                    "Possibly Long Function: `f` has 4 statements over 1 lines of its own (limits 1 statements, 2 lines), so a reader must hold every step in mind at once. Consider Extract Function: move each step into a function named for what it does (in Effect, a named `Effect.fn(\"domain.operation\")` step that this function yields).",
                )],
            },
            Case {
                name: "invalid 14: function f() { switch (x) { case 1: a(); break; default: }",
                path: "file.ts",
                code: "function f() { switch (x) { case 1: a(); break; default: } }",
                expected: &[(
                    9,
                    10,
                    "Possibly Long Function: `f` has 3 statements over 1 lines of its own (limits 1 statements, 2 lines), so a reader must hold every step in mind at once. Consider Extract Function: move each step into a function named for what it does (in Effect, a named `Effect.fn(\"domain.operation\")` step that this function yields).",
                )],
            },
            Case {
                name: "invalid 15: function f() { if (a) b(); else { c(); } ;;; }",
                path: "file.ts",
                code: "function f() { if (a) b(); else { c(); } ;;; }",
                expected: &[(
                    9,
                    10,
                    "Possibly Long Function: `f` has 3 statements over 1 lines of its own (limits 1 statements, 2 lines), so a reader must hold every step in mind at once. Consider Extract Function: move each step into a function named for what it does (in Effect, a named `Effect.fn(\"domain.operation\")` step that this function yields).",
                )],
            },
            Case {
                name: "invalid 16: function f() { do a(); while (x); while (y) b(); debugger;",
                path: "file.ts",
                code: "function f() { do a(); while (x); while (y) b(); debugger; throw e; }",
                expected: &[(
                    9,
                    10,
                    "Possibly Long Function: `f` has 6 statements over 1 lines of its own (limits 1 statements, 2 lines), so a reader must hold every step in mind at once. Consider Extract Function: move each step into a function named for what it does (in Effect, a named `Effect.fn(\"domain.operation\")` step that this function yields).",
                )],
            },
            Case {
                name: "invalid 17: function f() { type T = 1; interface I {} enum E {} }",
                path: "file.ts",
                code: "function f() { type T = 1; interface I {} enum E {} }",
                expected: &[],
            },
            Case { name: "invalid 18: function f() { return { [a()]: 1 }; }", path: "file.ts", code: "function f() { return { [a()]: 1 }; }", expected: &[] },
            Case {
                name: "invalid 19: class A { m() { a(); b(); } }",
                path: "file.ts",
                code: "class A { m() { a(); b(); } }",
                expected: &[(
                    10,
                    11,
                    "Possibly Long Function: `m` has 2 statements over 1 lines of its own (limits 1 statements, 2 lines), so a reader must hold every step in mind at once. Consider Extract Function: move each step into a function named for what it does (in Effect, a named `Effect.fn(\"domain.operation\")` step that this function yields).",
                )],
            },
            Case { name: "invalid 20: class A { static { a(); b(); } }", path: "file.ts", code: "class A { static { a(); b(); } }", expected: &[] },
            Case {
                name: "invalid 21: const o = { m() { a(); b(); } };",
                path: "file.ts",
                code: "const o = { m() { a(); b(); } };",
                expected: &[(
                    12,
                    13,
                    "Possibly Long Function: `m` has 2 statements over 1 lines of its own (limits 1 statements, 2 lines), so a reader must hold every step in mind at once. Consider Extract Function: move each step into a function named for what it does (in Effect, a named `Effect.fn(\"domain.operation\")` step that this function yields).",
                )],
            },
            Case {
                name: "invalid 22: function* gen() { yield a(); yield b(); }",
                path: "file.ts",
                code: "function* gen() { yield a(); yield b(); }",
                expected: &[(
                    10,
                    13,
                    "Possibly Long Function: `gen` has 2 statements over 1 lines of its own (limits 1 statements, 2 lines), so a reader must hold every step in mind at once. Consider Extract Function: move each step into a function named for what it does (in Effect, a named `Effect.fn(\"domain.operation\")` step that this function yields).",
                )],
            },
            Case {
                name: "invalid 23: export default function () { a(); b(); }",
                path: "file.ts",
                code: "export default function () { a(); b(); }",
                expected: &[(
                    15,
                    40,
                    "Possibly Long Function: This function has 2 statements over 1 lines of its own (limits 1 statements, 2 lines), so a reader must hold every step in mind at once. Consider Extract Function: move each step into a function named for what it does (in Effect, a named `Effect.fn(\"domain.operation\")` step that this function yields).",
                )],
            },
            Case {
                name: "invalid 24: export default () => { a(); b(); };",
                path: "file.ts",
                code: "export default () => { a(); b(); };",
                expected: &[(
                    15,
                    34,
                    "Possibly Long Function: This function has 2 statements over 1 lines of its own (limits 1 statements, 2 lines), so a reader must hold every step in mind at once. Consider Extract Function: move each step into a function named for what it does (in Effect, a named `Effect.fn(\"domain.operation\")` step that this function yields).",
                )],
            },
            Case { name: "invalid 25: const f = () => a() + b();", path: "file.ts", code: "const f = () => a() + b();", expected: &[] },
            Case {
                name: "invalid 26: function f() {",
                path: "file.ts",
                code: "function f() {\n// one\n// two\n\n/* three */\n}",
                expected: &[(
                    9,
                    10,
                    "Possibly Long Function: `f` has 0 statements over 6 lines of its own (limits 1 statements, 2 lines), so a reader must hold every step in mind at once. Consider Extract Function: move each step into a function named for what it does (in Effect, a named `Effect.fn(\"domain.operation\")` step that this function yields).",
                )],
            },
            Case { name: "invalid 27: function f() { a(); b(); }", path: "src/__tests__/f.ts", code: "function f() { a(); b(); }", expected: &[] },
        ],
    );
}

/// Edge cases recorded from upstream under oxlint 1.86: src/martinfowler/rules/long-function.mfedges.test.ts, options `[{"maxLines":4}]`
#[test]
fn edges_options_1() {
    check(
        "anti-slop-martinfowler/long-function",
        "[{\"maxLines\":4}]",
        &[Case {
            name: "invalid 8: function f() {",
            path: "file.ts",
            code: "function f() {\n  class A {\n    m() {\n      a();\n    }\n  }\n}",
            expected: &[(
                9,
                10,
                "Possibly Long Function: `f` has 1 statements over 5 lines of its own (limits 30 statements, 4 lines), so a reader must hold every step in mind at once. Consider Extract Function: move each step into a function named for what it does (in Effect, a named `Effect.fn(\"domain.operation\")` step that this function yields).",
            )],
        }],
    );
}

/// Edge cases recorded from upstream under oxlint 1.86: src/martinfowler/rules/long-function.mfedges.test.ts, options `[{"maxLines":2}]`
#[test]
fn edges_options_2() {
    check(
        "anti-slop-martinfowler/long-function",
        "[{\"maxLines\":2}]",
        &[
            Case {
                name: "invalid 9: function f() {",
                path: "file.ts",
                code: "function f() {\n  const g = () =>\n    1;\n}",
                expected: &[(
                    9,
                    10,
                    "Possibly Long Function: `f` has 1 statements over 3 lines of its own (limits 30 statements, 2 lines), so a reader must hold every step in mind at once. Consider Extract Function: move each step into a function named for what it does (in Effect, a named `Effect.fn(\"domain.operation\")` step that this function yields).",
                )],
            },
            Case {
                name: "invalid 10: function f() {",
                path: "file.ts",
                code: "function f() {\n  const o = {\n    m() {\n      a();\n    },\n    n: function () {\n    },\n  };\n}",
                expected: &[
                    (
                        9,
                        10,
                        "Possibly Long Function: `f` has 1 statements over 6 lines of its own (limits 30 statements, 2 lines), so a reader must hold every step in mind at once. Consider Extract Function: move each step into a function named for what it does (in Effect, a named `Effect.fn(\"domain.operation\")` step that this function yields).",
                    ),
                    (
                        33,
                        34,
                        "Possibly Long Function: `m` has 1 statements over 3 lines of its own (limits 30 statements, 2 lines), so a reader must hold every step in mind at once. Consider Extract Function: move each step into a function named for what it does (in Effect, a named `Effect.fn(\"domain.operation\")` step that this function yields).",
                    ),
                ],
            },
        ],
    );
}

/// Edge cases recorded from upstream under oxlint 1.86: src/martinfowler/rules/long-function.mfedges.test.ts, options `[{"ignoreTests":false,"maxStatements":1}]`
#[test]
fn edges_options_3() {
    check(
        "anti-slop-martinfowler/long-function",
        "[{\"ignoreTests\":false,\"maxStatements\":1}]",
        &[Case {
            name: "invalid 28: function f() { a(); b(); }",
            path: "src/__tests__/f.ts",
            code: "function f() { a(); b(); }",
            expected: &[(
                9,
                10,
                "Possibly Long Function: `f` has 2 statements over 1 lines of its own (limits 1 statements, 80 lines), so a reader must hold every step in mind at once. Consider Extract Function: move each step into a function named for what it does (in Effect, a named `Effect.fn(\"domain.operation\")` step that this function yields).",
            )],
        }],
    );
}
