//! `anti-slop-martinfowler/long-parameter-list`: expectations recorded from the upstream plugin
//! (github.com/k-mistele/anti-slop) under oxlint 1.86, one case per upstream
//! RuleTester case in `src/martinfowler/rules/long-parameter-list.test.ts`.

use super::super::super::anti_slop::tests::{Case, check};

/// src/martinfowler/rules/long-parameter-list.test.ts
#[test]
fn upstream() {
    check(
        "anti-slop-martinfowler/long-parameter-list",
        "[]",
        &[
            Case {
                name: "valid 0: function send(to, subject, body, options) {}",
                path: "file.ts",
                code: "function send(to, subject, body, options) {}",
                expected: &[],
            },
            Case {
                name: "valid 1: function log(level, message, context, tags, ...rest) {}",
                path: "file.ts",
                code: "function log(level, message, context, tags, ...rest) {}",
                expected: &[],
            },
            Case {
                name: "valid 2: function create({ a, b, c, d, e, f }) {}",
                path: "file.ts",
                code: "function create({ a, b, c, d, e, f }) {}",
                expected: &[],
            },
            Case {
                name: "valid 3: function bind(this: Window, a: A, b: B, c: C, d: D) {}",
                path: "file.ts",
                code: "function bind(this: Window, a: A, b: B, c: C, d: D) {}",
                expected: &[],
            },
            Case {
                name: "valid 4: declare function external(a, b, c, d, e): void;",
                path: "file.ts",
                code: "declare function external(a, b, c, d, e): void;",
                expected: &[],
            },
            Case {
                name: "valid 5: interface Api { call(a: A, b: B, c: C, d: D, e: E): void }",
                path: "file.ts",
                code: "interface Api { call(a: A, b: B, c: C, d: D, e: E): void }",
                expected: &[],
            },
            Case {
                name: "invalid 7: function send(to, cc, subject, body, attachments) {}",
                path: "file.ts",
                code: "function send(to, cc, subject, body, attachments) {}",
                expected: &[(
                    9,
                    13,
                    "Possibly Long Parameter List: `send` takes 5 positional parameters (limit 4), so every call site depends on argument order and adding one more breaks them all. Consider Introduce Parameter Object: group related parameters into one named input (in Effect, a `Schema.Struct` input type), or Preserve Whole Object when several come from the same value.",
                )],
            },
            Case {
                name: "invalid 8: const build = (a, b = 1, [c], d, e) => a;",
                path: "file.ts",
                code: "const build = (a, b = 1, [c], d, e) => a;",
                expected: &[(
                    6,
                    11,
                    "Possibly Long Parameter List: `build` takes 5 positional parameters (limit 4), so every call site depends on argument order and adding one more breaks them all. Consider Introduce Parameter Object: group related parameters into one named input (in Effect, a `Schema.Struct` input type), or Preserve Whole Object when several come from the same value.",
                )],
            },
            Case {
                name: "invalid 9: class Store { constructor(a, b, c, d, e) {} }",
                path: "file.ts",
                code: "class Store { constructor(a, b, c, d, e) {} }",
                expected: &[(
                    14,
                    25,
                    "Possibly Long Parameter List: `constructor` takes 5 positional parameters (limit 4), so every call site depends on argument order and adding one more breaks them all. Consider Introduce Parameter Object: group related parameters into one named input (in Effect, a `Schema.Struct` input type), or Preserve Whole Object when several come from the same value.",
                )],
            },
        ],
    );
}

/// src/martinfowler/rules/long-parameter-list.test.ts, options `[{"max":2}]`
#[test]
fn upstream_options_1() {
    check(
        "anti-slop-martinfowler/long-parameter-list",
        "[{\"max\":2}]",
        &[
            Case { name: "valid 6: function pair(a, b) {}", path: "file.ts", code: "function pair(a, b) {}", expected: &[] },
            Case {
                name: "invalid 10: items.map(function (a, b, c) {});",
                path: "file.ts",
                code: "items.map(function (a, b, c) {});",
                expected: &[(
                    10,
                    31,
                    "Possibly Long Parameter List: This function takes 3 positional parameters (limit 2), so every call site depends on argument order and adding one more breaks them all. Consider Introduce Parameter Object: group related parameters into one named input (in Effect, a `Schema.Struct` input type), or Preserve Whole Object when several come from the same value.",
                )],
            },
        ],
    );
}

/// Edge cases recorded from upstream under oxlint 1.86: src/martinfowler/rules/long-parameter-list.mfedges.test.ts
#[test]
fn edges() {
    check(
        "anti-slop-martinfowler/long-parameter-list",
        "[]",
        &[
            Case {
                name: "invalid 0: const run: Fn = (a, b, c, d, e) => a;",
                path: "file.ts",
                code: "const run: Fn = (a, b, c, d, e) => a;",
                expected: &[(
                    6,
                    13,
                    "Possibly Long Parameter List: `run` takes 5 positional parameters (limit 4), so every call site depends on argument order and adding one more breaks them all. Consider Introduce Parameter Object: group related parameters into one named input (in Effect, a `Schema.Struct` input type), or Preserve Whole Object when several come from the same value.",
                )],
            },
            Case {
                name: "invalid 1: class A { m(a, b, c, d, e) {} }",
                path: "file.ts",
                code: "class A { m(a, b, c, d, e) {} }",
                expected: &[(
                    10,
                    11,
                    "Possibly Long Parameter List: `m` takes 5 positional parameters (limit 4), so every call site depends on argument order and adding one more breaks them all. Consider Introduce Parameter Object: group related parameters into one named input (in Effect, a `Schema.Struct` input type), or Preserve Whole Object when several come from the same value.",
                )],
            },
            Case {
                name: "invalid 2: class A { #p(a, b, c, d, e) {} }",
                path: "file.ts",
                code: "class A { #p(a, b, c, d, e) {} }",
                expected: &[(
                    10,
                    12,
                    "Possibly Long Parameter List: `#p` takes 5 positional parameters (limit 4), so every call site depends on argument order and adding one more breaks them all. Consider Introduce Parameter Object: group related parameters into one named input (in Effect, a `Schema.Struct` input type), or Preserve Whole Object when several come from the same value.",
                )],
            },
            Case {
                name: "invalid 3: const o = { 'str key'(a, b, c, d, e) {} };",
                path: "file.ts",
                code: "const o = { 'str key'(a, b, c, d, e) {} };",
                expected: &[(
                    12,
                    21,
                    "Possibly Long Parameter List: `str key` takes 5 positional parameters (limit 4), so every call site depends on argument order and adding one more breaks them all. Consider Introduce Parameter Object: group related parameters into one named input (in Effect, a `Schema.Struct` input type), or Preserve Whole Object when several come from the same value.",
                )],
            },
            Case {
                name: "invalid 4: const o = { [k](a, b, c, d, e) {} };",
                path: "file.ts",
                code: "const o = { [k](a, b, c, d, e) {} };",
                expected: &[(
                    13,
                    14,
                    "Possibly Long Parameter List: This function takes 5 positional parameters (limit 4), so every call site depends on argument order and adding one more breaks them all. Consider Introduce Parameter Object: group related parameters into one named input (in Effect, a `Schema.Struct` input type), or Preserve Whole Object when several come from the same value.",
                )],
            },
            Case {
                name: "invalid 5: const o = { 1(a, b, c, d, e) {} };",
                path: "file.ts",
                code: "const o = { 1(a, b, c, d, e) {} };",
                expected: &[(
                    12,
                    13,
                    "Possibly Long Parameter List: This function takes 5 positional parameters (limit 4), so every call site depends on argument order and adding one more breaks them all. Consider Introduce Parameter Object: group related parameters into one named input (in Effect, a `Schema.Struct` input type), or Preserve Whole Object when several come from the same value.",
                )],
            },
            Case {
                name: "invalid 6: x.y = function (a, b, c, d, e) {};",
                path: "file.ts",
                code: "x.y = function (a, b, c, d, e) {};",
                expected: &[(
                    6,
                    33,
                    "Possibly Long Parameter List: `x.y` takes 5 positional parameters (limit 4), so every call site depends on argument order and adding one more breaks them all. Consider Introduce Parameter Object: group related parameters into one named input (in Effect, a `Schema.Struct` input type), or Preserve Whole Object when several come from the same value.",
                )],
            },
            Case {
                name: "invalid 7: x.y ??= function (a, b, c, d, e) {};",
                path: "file.ts",
                code: "x.y ??= function (a, b, c, d, e) {};",
                expected: &[(
                    8,
                    35,
                    "Possibly Long Parameter List: `x.y` takes 5 positional parameters (limit 4), so every call site depends on argument order and adding one more breaks them all. Consider Introduce Parameter Object: group related parameters into one named input (in Effect, a `Schema.Struct` input type), or Preserve Whole Object when several come from the same value.",
                )],
            },
            Case {
                name: "invalid 8: export default function (a, b, c, d, e) {}",
                path: "file.ts",
                code: "export default function (a, b, c, d, e) {}",
                expected: &[(
                    15,
                    42,
                    "Possibly Long Parameter List: This function takes 5 positional parameters (limit 4), so every call site depends on argument order and adding one more breaks them all. Consider Introduce Parameter Object: group related parameters into one named input (in Effect, a `Schema.Struct` input type), or Preserve Whole Object when several come from the same value.",
                )],
            },
            Case {
                name: "invalid 9: const f = ((a, b, c, d, e) => a) as Fn;",
                path: "file.ts",
                code: "const f = ((a, b, c, d, e) => a) as Fn;",
                expected: &[(
                    6,
                    7,
                    "Possibly Long Parameter List: `f` takes 5 positional parameters (limit 4), so every call site depends on argument order and adding one more breaks them all. Consider Introduce Parameter Object: group related parameters into one named input (in Effect, a `Schema.Struct` input type), or Preserve Whole Object when several come from the same value.",
                )],
            },
            Case {
                name: "invalid 10: const f = ((a, b, c, d, e) => a);",
                path: "file.ts",
                code: "const f = ((a, b, c, d, e) => a);",
                expected: &[(
                    6,
                    7,
                    "Possibly Long Parameter List: `f` takes 5 positional parameters (limit 4), so every call site depends on argument order and adding one more breaks them all. Consider Introduce Parameter Object: group related parameters into one named input (in Effect, a `Schema.Struct` input type), or Preserve Whole Object when several come from the same value.",
                )],
            },
            Case {
                name: "invalid 11: class A { constructor(private a, public b, c, d, e) {} }",
                path: "file.ts",
                code: "class A { constructor(private a, public b, c, d, e) {} }",
                expected: &[(
                    10,
                    21,
                    "Possibly Long Parameter List: `constructor` takes 5 positional parameters (limit 4), so every call site depends on argument order and adding one more breaks them all. Consider Introduce Parameter Object: group related parameters into one named input (in Effect, a `Schema.Struct` input type), or Preserve Whole Object when several come from the same value.",
                )],
            },
            Case {
                name: "invalid 12: function f(this: X, a, b, c, d, e) {}",
                path: "file.ts",
                code: "function f(this: X, a, b, c, d, e) {}",
                expected: &[(
                    9,
                    10,
                    "Possibly Long Parameter List: `f` takes 5 positional parameters (limit 4), so every call site depends on argument order and adding one more breaks them all. Consider Introduce Parameter Object: group related parameters into one named input (in Effect, a `Schema.Struct` input type), or Preserve Whole Object when several come from the same value.",
                )],
            },
            Case {
                name: "invalid 13: abstract class A { abstract m(a, b, c, d, e): void; }",
                path: "file.ts",
                code: "abstract class A { abstract m(a, b, c, d, e): void; }",
                expected: &[],
            },
            Case {
                name: "invalid 14: function f(a, b, c, d, e): void;",
                path: "file.ts",
                code: "function f(a, b, c, d, e): void;\nfunction f(...args) {}",
                expected: &[],
            },
            Case {
                name: "invalid 15: const s = Effect.fn(\"users.make\")(function* (a, b, c, d, e",
                path: "file.ts",
                code: "const s = Effect.fn(\"users.make\")(function* (a, b, c, d, e) {});",
                expected: &[(
                    34,
                    62,
                    "Possibly Long Parameter List: `users.make` takes 5 positional parameters (limit 4), so every call site depends on argument order and adding one more breaks them all. Consider Introduce Parameter Object: group related parameters into one named input (in Effect, a `Schema.Struct` input type), or Preserve Whole Object when several come from the same value.",
                )],
            },
            Case {
                name: "invalid 16: const s = Effect.gen(function* (a, b, c, d, e) {});",
                path: "file.ts",
                code: "const s = Effect.gen(function* (a, b, c, d, e) {});",
                expected: &[(
                    21,
                    49,
                    "Possibly Long Parameter List: `s` takes 5 positional parameters (limit 4), so every call site depends on argument order and adding one more breaks them all. Consider Introduce Parameter Object: group related parameters into one named input (in Effect, a `Schema.Struct` input type), or Preserve Whole Object when several come from the same value.",
                )],
            },
            Case {
                name: "invalid 17: const s = Effect.fn(function* (a, b, c, d, e) {});",
                path: "file.ts",
                code: "const s = Effect.fn(function* (a, b, c, d, e) {});",
                expected: &[(
                    20,
                    48,
                    "Possibly Long Parameter List: `s` takes 5 positional parameters (limit 4), so every call site depends on argument order and adding one more breaks them all. Consider Introduce Parameter Object: group related parameters into one named input (in Effect, a `Schema.Struct` input type), or Preserve Whole Object when several come from the same value.",
                )],
            },
            Case {
                name: "invalid 18: const s = foo(\"lbl\")(function (a, b, c, d, e) {});",
                path: "file.ts",
                code: "const s = foo(\"lbl\")(function (a, b, c, d, e) {});",
                expected: &[(
                    21,
                    48,
                    "Possibly Long Parameter List: `lbl` takes 5 positional parameters (limit 4), so every call site depends on argument order and adding one more breaks them all. Consider Introduce Parameter Object: group related parameters into one named input (in Effect, a `Schema.Struct` input type), or Preserve Whole Object when several come from the same value.",
                )],
            },
            Case {
                name: "invalid 19: const s = foo(`tpl`)(function (a, b, c, d, e) {});",
                path: "file.ts",
                code: "const s = foo(`tpl`)(function (a, b, c, d, e) {});",
                expected: &[(
                    21,
                    48,
                    "Possibly Long Parameter List: `tpl` takes 5 positional parameters (limit 4), so every call site depends on argument order and adding one more breaks them all. Consider Introduce Parameter Object: group related parameters into one named input (in Effect, a `Schema.Struct` input type), or Preserve Whole Object when several come from the same value.",
                )],
            },
            Case {
                name: "invalid 20: const s = wrap(function (a, b, c, d, e) {});",
                path: "file.ts",
                code: "const s = wrap(function (a, b, c, d, e) {});",
                expected: &[(
                    15,
                    42,
                    "Possibly Long Parameter List: This function takes 5 positional parameters (limit 4), so every call site depends on argument order and adding one more breaks them all. Consider Introduce Parameter Object: group related parameters into one named input (in Effect, a `Schema.Struct` input type), or Preserve Whole Object when several come from the same value.",
                )],
            },
            Case {
                name: "invalid 21: const s = pipe(Effect.gen(function* (a, b, c, d, e) {}));",
                path: "file.ts",
                code: "const s = pipe(Effect.gen(function* (a, b, c, d, e) {}));",
                expected: &[(
                    26,
                    54,
                    "Possibly Long Parameter List: `s` takes 5 positional parameters (limit 4), so every call site depends on argument order and adding one more breaks them all. Consider Introduce Parameter Object: group related parameters into one named input (in Effect, a `Schema.Struct` input type), or Preserve Whole Object when several come from the same value.",
                )],
            },
            Case {
                name: "invalid 22: const s = Effect.fn(function* (a, b, c, d, e) {}).pipe(x);",
                path: "file.ts",
                code: "const s = Effect.fn(function* (a, b, c, d, e) {}).pipe(x);",
                expected: &[(
                    20,
                    48,
                    "Possibly Long Parameter List: This function takes 5 positional parameters (limit 4), so every call site depends on argument order and adding one more breaks them all. Consider Introduce Parameter Object: group related parameters into one named input (in Effect, a `Schema.Struct` input type), or Preserve Whole Object when several come from the same value.",
                )],
            },
            Case {
                name: "invalid 23: class A { p = (a, b, c, d, e) => a; }",
                path: "file.ts",
                code: "class A { p = (a, b, c, d, e) => a; }",
                expected: &[(
                    10,
                    11,
                    "Possibly Long Parameter List: `p` takes 5 positional parameters (limit 4), so every call site depends on argument order and adding one more breaks them all. Consider Introduce Parameter Object: group related parameters into one named input (in Effect, a `Schema.Struct` input type), or Preserve Whole Object when several come from the same value.",
                )],
            },
            Case {
                name: "invalid 24: class A { accessor p = (a, b, c, d, e) => a; }",
                path: "file.ts",
                code: "class A { accessor p = (a, b, c, d, e) => a; }",
                expected: &[(
                    23,
                    43,
                    "Possibly Long Parameter List: This function takes 5 positional parameters (limit 4), so every call site depends on argument order and adding one more breaks them all. Consider Introduce Parameter Object: group related parameters into one named input (in Effect, a `Schema.Struct` input type), or Preserve Whole Object when several come from the same value.",
                )],
            },
            Case {
                name: "invalid 25: const { a = (p, q, r, s, t) => p } = o;",
                path: "file.ts",
                code: "const { a = (p, q, r, s, t) => p } = o;",
                expected: &[(
                    12,
                    32,
                    "Possibly Long Parameter List: This function takes 5 positional parameters (limit 4), so every call site depends on argument order and adding one more breaks them all. Consider Introduce Parameter Object: group related parameters into one named input (in Effect, a `Schema.Struct` input type), or Preserve Whole Object when several come from the same value.",
                )],
            },
            Case {
                name: "invalid 26: const x = function named(a, b, c, d, e) {};",
                path: "file.ts",
                code: "const x = function named(a, b, c, d, e) {};",
                expected: &[(
                    19,
                    24,
                    "Possibly Long Parameter List: `named` takes 5 positional parameters (limit 4), so every call site depends on argument order and adding one more breaks them all. Consider Introduce Parameter Object: group related parameters into one named input (in Effect, a `Schema.Struct` input type), or Preserve Whole Object when several come from the same value.",
                )],
            },
            Case {
                name: "invalid 27: const f = async (a, b, c, d, e) => a;",
                path: "file.ts",
                code: "const f = async (a, b, c, d, e) => a;",
                expected: &[(
                    6,
                    7,
                    "Possibly Long Parameter List: `f` takes 5 positional parameters (limit 4), so every call site depends on argument order and adding one more breaks them all. Consider Introduce Parameter Object: group related parameters into one named input (in Effect, a `Schema.Struct` input type), or Preserve Whole Object when several come from the same value.",
                )],
            },
            Case {
                name: "invalid 28: const o = { get v() { return 1; }, set v(x) {}, m: (a, b, ",
                path: "file.ts",
                code: "const o = { get v() { return 1; }, set v(x) {}, m: (a, b, c, d, e) => a };",
                expected: &[(
                    48,
                    49,
                    "Possibly Long Parameter List: `m` takes 5 positional parameters (limit 4), so every call site depends on argument order and adding one more breaks them all. Consider Introduce Parameter Object: group related parameters into one named input (in Effect, a `Schema.Struct` input type), or Preserve Whole Object when several come from the same value.",
                )],
            },
            Case {
                name: "invalid 29: let g; g = (a, b, c, d, e) => a;",
                path: "file.ts",
                code: "let g; g = (a, b, c, d, e) => a;",
                expected: &[(
                    11,
                    31,
                    "Possibly Long Parameter List: `g` takes 5 positional parameters (limit 4), so every call site depends on argument order and adding one more breaks them all. Consider Introduce Parameter Object: group related parameters into one named input (in Effect, a `Schema.Struct` input type), or Preserve Whole Object when several come from the same value.",
                )],
            },
            Case {
                name: "invalid 30: const [h] = [(a, b, c, d, e) => a];",
                path: "file.ts",
                code: "const [h] = [(a, b, c, d, e) => a];",
                expected: &[(
                    13,
                    33,
                    "Possibly Long Parameter List: This function takes 5 positional parameters (limit 4), so every call site depends on argument order and adding one more breaks them all. Consider Introduce Parameter Object: group related parameters into one named input (in Effect, a `Schema.Struct` input type), or Preserve Whole Object when several come from the same value.",
                )],
            },
            Case {
                name: "invalid 31: new Foo(function (a, b, c, d, e) {});",
                path: "file.ts",
                code: "new Foo(function (a, b, c, d, e) {});",
                expected: &[(
                    8,
                    35,
                    "Possibly Long Parameter List: This function takes 5 positional parameters (limit 4), so every call site depends on argument order and adding one more breaks them all. Consider Introduce Parameter Object: group related parameters into one named input (in Effect, a `Schema.Struct` input type), or Preserve Whole Object when several come from the same value.",
                )],
            },
            Case {
                name: "invalid 32: function f(a = 1, b, { c }, [d], e: number) {}",
                path: "file.ts",
                code: "function f(a = 1, b, { c }, [d], e: number) {}",
                expected: &[(
                    9,
                    10,
                    "Possibly Long Parameter List: `f` takes 5 positional parameters (limit 4), so every call site depends on argument order and adding one more breaks them all. Consider Introduce Parameter Object: group related parameters into one named input (in Effect, a `Schema.Struct` input type), or Preserve Whole Object when several come from the same value.",
                )],
            },
            Case { name: "invalid 33: function f(a, b, c, d, ...e) {}", path: "file.ts", code: "function f(a, b, c, d, ...e) {}", expected: &[] },
            Case { name: "invalid 35: function f(a, b, c, d, e) {}", path: "src/a.test.ts", code: "function f(a, b, c, d, e) {}", expected: &[] },
            Case { name: "invalid 36: function f(a, b, c, d, e) {}", path: "src/a.spec.jsx", code: "function f(a, b, c, d, e) {}", expected: &[] },
            Case { name: "invalid 37: function f(a, b, c, d, e) {}", path: "test/a.ts", code: "function f(a, b, c, d, e) {}", expected: &[] },
            Case { name: "invalid 38: function f(a, b, c, d, e) {}", path: "src/__mocks__/a.ts", code: "function f(a, b, c, d, e) {}", expected: &[] },
            Case {
                name: "invalid 39: function f(a, b, c, d, e) {}",
                path: "src/test.ts",
                code: "function f(a, b, c, d, e) {}",
                expected: &[(
                    9,
                    10,
                    "Possibly Long Parameter List: `f` takes 5 positional parameters (limit 4), so every call site depends on argument order and adding one more breaks them all. Consider Introduce Parameter Object: group related parameters into one named input (in Effect, a `Schema.Struct` input type), or Preserve Whole Object when several come from the same value.",
                )],
            },
            Case {
                name: "invalid 40: function f(a, b, c, d, e) {}",
                path: "src/contest/a.ts",
                code: "function f(a, b, c, d, e) {}",
                expected: &[(
                    9,
                    10,
                    "Possibly Long Parameter List: `f` takes 5 positional parameters (limit 4), so every call site depends on argument order and adding one more breaks them all. Consider Introduce Parameter Object: group related parameters into one named input (in Effect, a `Schema.Struct` input type), or Preserve Whole Object when several come from the same value.",
                )],
            },
        ],
    );
}

/// Edge cases recorded from upstream under oxlint 1.86: src/martinfowler/rules/long-parameter-list.mfedges.test.ts, options `[{"max":0}]`
#[test]
fn edges_options_1() {
    check(
        "anti-slop-martinfowler/long-parameter-list",
        "[{\"max\":0}]",
        &[Case {
            name: "invalid 34: function f(a) {}",
            path: "file.ts",
            code: "function f(a) {}",
            expected: &[(
                9,
                10,
                "Possibly Long Parameter List: `f` takes 1 positional parameters (limit 0), so every call site depends on argument order and adding one more breaks them all. Consider Introduce Parameter Object: group related parameters into one named input (in Effect, a `Schema.Struct` input type), or Preserve Whole Object when several come from the same value.",
            )],
        }],
    );
}

/// Edge cases recorded from upstream under oxlint 1.86: src/martinfowler/rules/long-parameter-list.mfedges.test.ts, options `[{"ignoreTests":false}]`
#[test]
fn edges_options_2() {
    check(
        "anti-slop-martinfowler/long-parameter-list",
        "[{\"ignoreTests\":false}]",
        &[Case {
            name: "invalid 41: function f(a, b, c, d, e) {}",
            path: "src/a.test.ts",
            code: "function f(a, b, c, d, e) {}",
            expected: &[(
                9,
                10,
                "Possibly Long Parameter List: `f` takes 5 positional parameters (limit 4), so every call site depends on argument order and adding one more breaks them all. Consider Introduce Parameter Object: group related parameters into one named input (in Effect, a `Schema.Struct` input type), or Preserve Whole Object when several come from the same value.",
            )],
        }],
    );
}
