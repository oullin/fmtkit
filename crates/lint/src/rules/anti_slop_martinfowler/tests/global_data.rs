//! `anti-slop-martinfowler/global-data`: expectations recorded from the upstream plugin
//! (github.com/k-mistele/anti-slop) under oxlint 1.86, one case per upstream
//! RuleTester case in `src/martinfowler/rules/global-data.test.ts`.

use super::super::super::anti_slop::tests::{Case, check};

/// src/martinfowler/rules/global-data.test.ts
#[test]
fn upstream() {
    check(
        "anti-slop-martinfowler/global-data",
        "[]",
        &[
            Case { name: "valid 0: export const limits = { max: 10 };", path: "file.ts", code: "export const limits = { max: 10 };", expected: &[] },
            Case {
                name: "valid 1: export const names = ['a', 'b']; console.log(names.length);",
                path: "file.ts",
                code: "export const names = ['a', 'b']; console.log(names.length);",
                expected: &[],
            },
            Case {
                name: "valid 2: export const cache = new Map(); const copy = new Map(cache); ",
                path: "file.ts",
                code: "export const cache = new Map(); const copy = new Map(cache); copy.set('a', 1);",
                expected: &[],
            },
            Case {
                name: "valid 3: export const registry = new Map(); const value = registry.get",
                path: "file.ts",
                code: "export const registry = new Map(); const value = registry.get('a');",
                expected: &[],
            },
            Case {
                name: "valid 4: let count = 0; export function bump() { count += 1; }",
                path: "file.ts",
                code: "let count = 0; export function bump() { count += 1; }",
                expected: &[],
            },
            Case {
                name: "valid 5: const seen = new Set(); export function mark(id) { seen.add(i",
                path: "file.ts",
                code: "const seen = new Set(); export function mark(id) { seen.add(id); }",
                expected: &[],
            },
            Case { name: "valid 6: export { state } from './state.ts';", path: "file.ts", code: "export { state } from './state.ts';", expected: &[] },
            Case {
                name: "valid 7: export const config = loadConfig(); config.port = 1;",
                path: "file.ts",
                code: "export const config = loadConfig(); config.port = 1;",
                expected: &[],
            },
            Case {
                name: "invalid 8: export let current = null;",
                path: "file.ts",
                code: "export let current = null;",
                expected: &[(
                    11,
                    18,
                    "Possibly Global Data: exported `current` is module-level state that is declared with `let`, so it can be reassigned, so any importer can read it while code anywhere changes it. Consider Encapsulate Variable: keep the state private and expose functions that own every read and write (in Effect, a `Ref` created in a service's layer and reached only through its operations).",
                )],
            },
            Case {
                name: "invalid 9: export let current = null; export function set(value) { cur",
                path: "file.ts",
                code: "export let current = null; export function set(value) { current = value; }",
                expected: &[(
                    11,
                    18,
                    "Possibly Global Data: exported `current` is module-level state that is declared with `let` and reassigned in this file, so any importer can read it while code anywhere changes it. Consider Encapsulate Variable: keep the state private and expose functions that own every read and write (in Effect, a `Ref` created in a service's layer and reached only through its operations).",
                )],
            },
            Case {
                name: "invalid 10: export const registry = new Map(); export function add(k, ",
                path: "file.ts",
                code: "export const registry = new Map(); export function add(k, v) { registry.set(k, v); }",
                expected: &[(
                    13,
                    21,
                    "Possibly Global Data: exported `registry` is module-level state that is mutated in this file, so any importer can read it while code anywhere changes it. Consider Encapsulate Variable: keep the state private and expose functions that own every read and write (in Effect, a `Ref` created in a service's layer and reached only through its operations).",
                )],
            },
            Case {
                name: "invalid 11: export const queue = []; queue.push(1);",
                path: "file.ts",
                code: "export const queue = []; queue.push(1);",
                expected: &[(
                    13,
                    18,
                    "Possibly Global Data: exported `queue` is module-level state that is mutated in this file, so any importer can read it while code anywhere changes it. Consider Encapsulate Variable: keep the state private and expose functions that own every read and write (in Effect, a `Ref` created in a service's layer and reached only through its operations).",
                )],
            },
            Case {
                name: "invalid 12: const state = { user: { name: '' } }; state.user.name = 'x",
                path: "file.ts",
                code: "const state = { user: { name: '' } }; state.user.name = 'x'; export { state };",
                expected: &[(
                    6,
                    11,
                    "Possibly Global Data: exported `state` is module-level state that is mutated in this file, so any importer can read it while code anywhere changes it. Consider Encapsulate Variable: keep the state private and expose functions that own every read and write (in Effect, a `Ref` created in a service's layer and reached only through its operations).",
                )],
            },
            Case {
                name: "invalid 13: const counters = {}; export default counters; counters.hit",
                path: "file.ts",
                code: "const counters = {}; export default counters; counters.hits++;",
                expected: &[(
                    6,
                    14,
                    "Possibly Global Data: exported `counters` is module-level state that is mutated in this file, so any importer can read it while code anywhere changes it. Consider Encapsulate Variable: keep the state private and expose functions that own every read and write (in Effect, a `Ref` created in a service's layer and reached only through its operations).",
                )],
            },
            Case {
                name: "invalid 14: export const settings = {}; Object.assign(settings, overri",
                path: "file.ts",
                code: "export const settings = {}; Object.assign(settings, overrides);",
                expected: &[(
                    13,
                    21,
                    "Possibly Global Data: exported `settings` is module-level state that is mutated in this file, so any importer can read it while code anywhere changes it. Consider Encapsulate Variable: keep the state private and expose functions that own every read and write (in Effect, a `Ref` created in a service's layer and reached only through its operations).",
                )],
            },
            Case {
                name: "invalid 15: export const flags = {} as Record<string, boolean>; delete",
                path: "file.ts",
                code: "export const flags = {} as Record<string, boolean>; delete flags.beta;",
                expected: &[(
                    13,
                    18,
                    "Possibly Global Data: exported `flags` is module-level state that is mutated in this file, so any importer can read it while code anywhere changes it. Consider Encapsulate Variable: keep the state private and expose functions that own every read and write (in Effect, a `Ref` created in a service's layer and reached only through its operations).",
                )],
            },
        ],
    );
}

/// Edge cases recorded from upstream under oxlint 1.86: src/martinfowler/rules/global-data.mfedges.test.ts
#[test]
fn edges() {
    check(
        "anti-slop-martinfowler/global-data",
        "[]",
        &[
            Case {
                name: "invalid 0: export let x: number = 1;",
                path: "file.ts",
                code: "export let x: number = 1;",
                expected: &[(
                    11,
                    20,
                    "Possibly Global Data: exported `x` is module-level state that is declared with `let`, so it can be reassigned, so any importer can read it while code anywhere changes it. Consider Encapsulate Variable: keep the state private and expose functions that own every read and write (in Effect, a `Ref` created in a service's layer and reached only through its operations).",
                )],
            },
            Case {
                name: "invalid 1: export var a = 1, b = 2; export function f() { b = 3; }",
                path: "file.ts",
                code: "export var a = 1, b = 2; export function f() { b = 3; }",
                expected: &[
                    (
                        11,
                        12,
                        "Possibly Global Data: exported `a` is module-level state that is declared with `var`, so it can be reassigned, so any importer can read it while code anywhere changes it. Consider Encapsulate Variable: keep the state private and expose functions that own every read and write (in Effect, a `Ref` created in a service's layer and reached only through its operations).",
                    ),
                    (
                        18,
                        19,
                        "Possibly Global Data: exported `b` is module-level state that is declared with `var` and reassigned in this file, so any importer can read it while code anywhere changes it. Consider Encapsulate Variable: keep the state private and expose functions that own every read and write (in Effect, a `Ref` created in a service's layer and reached only through its operations).",
                    ),
                ],
            },
            Case {
                name: "invalid 2: export const m = new Map(); m.get('a').push(1);",
                path: "file.ts",
                code: "export const m = new Map(); m.get('a').push(1);",
                expected: &[],
            },
            Case {
                name: "invalid 3: export const list = []; list.length = 0;",
                path: "file.ts",
                code: "export const list = []; list.length = 0;",
                expected: &[(
                    13,
                    17,
                    "Possibly Global Data: exported `list` is module-level state that is mutated in this file, so any importer can read it while code anywhere changes it. Consider Encapsulate Variable: keep the state private and expose functions that own every read and write (in Effect, a `Ref` created in a service's layer and reached only through its operations).",
                )],
            },
            Case {
                name: "invalid 4: export const o = {}; (o as any).x = 1;",
                path: "file.ts",
                code: "export const o = {}; (o as any).x = 1;",
                expected: &[(
                    13,
                    14,
                    "Possibly Global Data: exported `o` is module-level state that is mutated in this file, so any importer can read it while code anywhere changes it. Consider Encapsulate Variable: keep the state private and expose functions that own every read and write (in Effect, a `Ref` created in a service's layer and reached only through its operations).",
                )],
            },
            Case {
                name: "invalid 5: export const o = {}; o!.x = 1;",
                path: "file.ts",
                code: "export const o = {}; o!.x = 1;",
                expected: &[(
                    13,
                    14,
                    "Possibly Global Data: exported `o` is module-level state that is mutated in this file, so any importer can read it while code anywhere changes it. Consider Encapsulate Variable: keep the state private and expose functions that own every read and write (in Effect, a `Ref` created in a service's layer and reached only through its operations).",
                )],
            },
            Case {
                name: "invalid 6: export const o = {}; (o).x = 1;",
                path: "file.ts",
                code: "export const o = {}; (o).x = 1;",
                expected: &[(
                    13,
                    14,
                    "Possibly Global Data: exported `o` is module-level state that is mutated in this file, so any importer can read it while code anywhere changes it. Consider Encapsulate Variable: keep the state private and expose functions that own every read and write (in Effect, a `Ref` created in a service's layer and reached only through its operations).",
                )],
            },
            Case {
                name: "invalid 7: export const o = {}; o.a.b.push(1);",
                path: "file.ts",
                code: "export const o = {}; o.a.b.push(1);",
                expected: &[(
                    13,
                    14,
                    "Possibly Global Data: exported `o` is module-level state that is mutated in this file, so any importer can read it while code anywhere changes it. Consider Encapsulate Variable: keep the state private and expose functions that own every read and write (in Effect, a `Ref` created in a service's layer and reached only through its operations).",
                )],
            },
            Case {
                name: "invalid 8: export const o = {}; o?.a.push(1);",
                path: "file.ts",
                code: "export const o = {}; o?.a.push(1);",
                expected: &[(
                    13,
                    14,
                    "Possibly Global Data: exported `o` is module-level state that is mutated in this file, so any importer can read it while code anywhere changes it. Consider Encapsulate Variable: keep the state private and expose functions that own every read and write (in Effect, a `Ref` created in a service's layer and reached only through its operations).",
                )],
            },
            Case {
                name: "invalid 9: export const o = {}; o.a?.b.push(1);",
                path: "file.ts",
                code: "export const o = {}; o.a?.b.push(1);",
                expected: &[(
                    13,
                    14,
                    "Possibly Global Data: exported `o` is module-level state that is mutated in this file, so any importer can read it while code anywhere changes it. Consider Encapsulate Variable: keep the state private and expose functions that own every read and write (in Effect, a `Ref` created in a service's layer and reached only through its operations).",
                )],
            },
            Case {
                name: "invalid 10: export const s = new Set(); s.add(1);",
                path: "file.ts",
                code: "export const s = new Set(); s.add(1);",
                expected: &[(
                    13,
                    14,
                    "Possibly Global Data: exported `s` is module-level state that is mutated in this file, so any importer can read it while code anywhere changes it. Consider Encapsulate Variable: keep the state private and expose functions that own every read and write (in Effect, a `Ref` created in a service's layer and reached only through its operations).",
                )],
            },
            Case {
                name: "invalid 11: export const s = new WeakMap(); s.delete(k);",
                path: "file.ts",
                code: "export const s = new WeakMap(); s.delete(k);",
                expected: &[(
                    13,
                    14,
                    "Possibly Global Data: exported `s` is module-level state that is mutated in this file, so any importer can read it while code anywhere changes it. Consider Encapsulate Variable: keep the state private and expose functions that own every read and write (in Effect, a `Ref` created in a service's layer and reached only through its operations).",
                )],
            },
            Case { name: "invalid 12: export const s = new Foo(); s.add(1);", path: "file.ts", code: "export const s = new Foo(); s.add(1);", expected: &[] },
            Case {
                name: "invalid 13: export const s = new (Map)(); s.clear();",
                path: "file.ts",
                code: "export const s = new (Map)(); s.clear();",
                expected: &[(
                    13,
                    14,
                    "Possibly Global Data: exported `s` is module-level state that is mutated in this file, so any importer can read it while code anywhere changes it. Consider Encapsulate Variable: keep the state private and expose functions that own every read and write (in Effect, a `Ref` created in a service's layer and reached only through its operations).",
                )],
            },
            Case {
                name: "invalid 14: export const o = Object.freeze({}); o.x = 1;",
                path: "file.ts",
                code: "export const o = Object.freeze({}); o.x = 1;",
                expected: &[],
            },
            Case {
                name: "invalid 15: const o = {}; export { o as default }; o.x = 1;",
                path: "file.ts",
                code: "const o = {}; export { o as default }; o.x = 1;",
                expected: &[(
                    6,
                    7,
                    "Possibly Global Data: exported `o` is module-level state that is mutated in this file, so any importer can read it while code anywhere changes it. Consider Encapsulate Variable: keep the state private and expose functions that own every read and write (in Effect, a `Ref` created in a service's layer and reached only through its operations).",
                )],
            },
            Case {
                name: "invalid 16: const o = {}; export default o; Object.assign(o, x);",
                path: "file.ts",
                code: "const o = {}; export default o; Object.assign(o, x);",
                expected: &[(
                    6,
                    7,
                    "Possibly Global Data: exported `o` is module-level state that is mutated in this file, so any importer can read it while code anywhere changes it. Consider Encapsulate Variable: keep the state private and expose functions that own every read and write (in Effect, a `Ref` created in a service's layer and reached only through its operations).",
                )],
            },
            Case {
                name: "invalid 17: const o = {}; export default (o); o.x = 1;",
                path: "file.ts",
                code: "const o = {}; export default (o); o.x = 1;",
                expected: &[(
                    6,
                    7,
                    "Possibly Global Data: exported `o` is module-level state that is mutated in this file, so any importer can read it while code anywhere changes it. Consider Encapsulate Variable: keep the state private and expose functions that own every read and write (in Effect, a `Ref` created in a service's layer and reached only through its operations).",
                )],
            },
            Case {
                name: "invalid 18: export const o = {}; Object.assign({}, o);",
                path: "file.ts",
                code: "export const o = {}; Object.assign({}, o);",
                expected: &[],
            },
            Case {
                name: "invalid 19: export const o = {}; Object['assign'](o, x);",
                path: "file.ts",
                code: "export const o = {}; Object['assign'](o, x);",
                expected: &[],
            },
            Case {
                name: "invalid 20: export const o = {}; Object.assign((o as any), x);",
                path: "file.ts",
                code: "export const o = {}; Object.assign((o as any), x);",
                expected: &[(
                    13,
                    14,
                    "Possibly Global Data: exported `o` is module-level state that is mutated in this file, so any importer can read it while code anywhere changes it. Consider Encapsulate Variable: keep the state private and expose functions that own every read and write (in Effect, a `Ref` created in a service's layer and reached only through its operations).",
                )],
            },
            Case {
                name: "invalid 21: export const o = {}; function f(o) { o.x = 1; }",
                path: "file.ts",
                code: "export const o = {}; function f(o) { o.x = 1; }",
                expected: &[],
            },
            Case {
                name: "invalid 22: export const o = {} satisfies X; o.x++;",
                path: "file.ts",
                code: "export const o = {} satisfies X; o.x++;",
                expected: &[(
                    13,
                    14,
                    "Possibly Global Data: exported `o` is module-level state that is mutated in this file, so any importer can read it while code anywhere changes it. Consider Encapsulate Variable: keep the state private and expose functions that own every read and write (in Effect, a `Ref` created in a service's layer and reached only through its operations).",
                )],
            },
            Case {
                name: "invalid 23: export const o = {}; --o.x;",
                path: "file.ts",
                code: "export const o = {}; --o.x;",
                expected: &[(
                    13,
                    14,
                    "Possibly Global Data: exported `o` is module-level state that is mutated in this file, so any importer can read it while code anywhere changes it. Consider Encapsulate Variable: keep the state private and expose functions that own every read and write (in Effect, a `Ref` created in a service's layer and reached only through its operations).",
                )],
            },
            Case { name: "invalid 24: export const [a] = []; a.push(1);", path: "file.ts", code: "export const [a] = []; a.push(1);", expected: &[] },
            Case {
                name: "invalid 25: let x = 1; export { x };",
                path: "file.ts",
                code: "let x = 1; export { x };",
                expected: &[(
                    4,
                    5,
                    "Possibly Global Data: exported `x` is module-level state that is declared with `let`, so it can be reassigned, so any importer can read it while code anywhere changes it. Consider Encapsulate Variable: keep the state private and expose functions that own every read and write (in Effect, a `Ref` created in a service's layer and reached only through its operations).",
                )],
            },
            Case {
                name: "invalid 26: export const a = []; delete a[0];",
                path: "file.ts",
                code: "export const a = []; delete a[0];",
                expected: &[(
                    13,
                    14,
                    "Possibly Global Data: exported `a` is module-level state that is mutated in this file, so any importer can read it while code anywhere changes it. Consider Encapsulate Variable: keep the state private and expose functions that own every read and write (in Effect, a `Ref` created in a service's layer and reached only through its operations).",
                )],
            },
            Case {
                name: "invalid 27: export const a = []; a[0] = 1;",
                path: "file.ts",
                code: "export const a = []; a[0] = 1;",
                expected: &[(
                    13,
                    14,
                    "Possibly Global Data: exported `a` is module-level state that is mutated in this file, so any importer can read it while code anywhere changes it. Consider Encapsulate Variable: keep the state private and expose functions that own every read and write (in Effect, a `Ref` created in a service's layer and reached only through its operations).",
                )],
            },
            Case {
                name: "invalid 28: export const a = []; a.b.c = 1;",
                path: "file.ts",
                code: "export const a = []; a.b.c = 1;",
                expected: &[(
                    13,
                    14,
                    "Possibly Global Data: exported `a` is module-level state that is mutated in this file, so any importer can read it while code anywhere changes it. Consider Encapsulate Variable: keep the state private and expose functions that own every read and write (in Effect, a `Ref` created in a service's layer and reached only through its operations).",
                )],
            },
            Case {
                name: "invalid 29: export const a = []; a.sort().push(1);",
                path: "file.ts",
                code: "export const a = []; a.sort().push(1);",
                expected: &[(
                    13,
                    14,
                    "Possibly Global Data: exported `a` is module-level state that is mutated in this file, so any importer can read it while code anywhere changes it. Consider Encapsulate Variable: keep the state private and expose functions that own every read and write (in Effect, a `Ref` created in a service's layer and reached only through its operations).",
                )],
            },
            Case { name: "invalid 30: export const a = []; a.map(f).push(1);", path: "file.ts", code: "export const a = []; a.map(f).push(1);", expected: &[] },
            Case { name: "invalid 31: export const a = []; [a.x] = [1];", path: "file.ts", code: "export const a = []; [a.x] = [1];", expected: &[] },
            Case {
                name: "invalid 32: export const a = []; for (a.x of xs) {}",
                path: "file.ts",
                code: "export const a = []; for (a.x of xs) {}",
                expected: &[],
            },
            Case { name: "invalid 33: export const fn = () => {}; fn.x = 1;", path: "file.ts", code: "export const fn = () => {}; fn.x = 1;", expected: &[] },
            Case {
                name: "invalid 34: let x = 1; export type { x };",
                path: "file.ts",
                code: "let x = 1; export type { x };",
                expected: &[(
                    4,
                    5,
                    "Possibly Global Data: exported `x` is module-level state that is declared with `let`, so it can be reassigned, so any importer can read it while code anywhere changes it. Consider Encapsulate Variable: keep the state private and expose functions that own every read and write (in Effect, a `Ref` created in a service's layer and reached only through its operations).",
                )],
            },
            Case {
                name: "invalid 35: const o = {}; export { o, o as p }; o.x = 1;",
                path: "file.ts",
                code: "const o = {}; export { o, o as p }; o.x = 1;",
                expected: &[(
                    6,
                    7,
                    "Possibly Global Data: exported `o` is module-level state that is mutated in this file, so any importer can read it while code anywhere changes it. Consider Encapsulate Variable: keep the state private and expose functions that own every read and write (in Effect, a `Ref` created in a service's layer and reached only through its operations).",
                )],
            },
            Case {
                name: "invalid 36: const o = {}; const o2 = []; export { o2 }; o.x = 1;",
                path: "file.ts",
                code: "const o = {}; const o2 = []; export { o2 }; o.x = 1;",
                expected: &[],
            },
            Case {
                name: "invalid 37: export const o = {}; o.x += 1;",
                path: "file.ts",
                code: "export const o = {}; o.x += 1;",
                expected: &[(
                    13,
                    14,
                    "Possibly Global Data: exported `o` is module-level state that is mutated in this file, so any importer can read it while code anywhere changes it. Consider Encapsulate Variable: keep the state private and expose functions that own every read and write (in Effect, a `Ref` created in a service's layer and reached only through its operations).",
                )],
            },
            Case {
                name: "invalid 38: export const o = {}; o.x.y ??= 1;",
                path: "file.ts",
                code: "export const o = {}; o.x.y ??= 1;",
                expected: &[(
                    13,
                    14,
                    "Possibly Global Data: exported `o` is module-level state that is mutated in this file, so any importer can read it while code anywhere changes it. Consider Encapsulate Variable: keep the state private and expose functions that own every read and write (in Effect, a `Ref` created in a service's layer and reached only through its operations).",
                )],
            },
            Case {
                name: "invalid 39: export const o = {}; o['k'] = 1;",
                path: "file.ts",
                code: "export const o = {}; o['k'] = 1;",
                expected: &[(
                    13,
                    14,
                    "Possibly Global Data: exported `o` is module-level state that is mutated in this file, so any importer can read it while code anywhere changes it. Consider Encapsulate Variable: keep the state private and expose functions that own every read and write (in Effect, a `Ref` created in a service's layer and reached only through its operations).",
                )],
            },
            Case {
                name: "invalid 40: export declare let d: number;",
                path: "file.ts",
                code: "export declare let d: number;",
                expected: &[(
                    19,
                    28,
                    "Possibly Global Data: exported `d` is module-level state that is declared with `let`, so it can be reassigned, so any importer can read it while code anywhere changes it. Consider Encapsulate Variable: keep the state private and expose functions that own every read and write (in Effect, a `Ref` created in a service's layer and reached only through its operations).",
                )],
            },
            Case { name: "invalid 41: export const o = []; o.push;", path: "file.ts", code: "export const o = []; o.push;", expected: &[] },
            Case {
                name: "invalid 42: export const o = []; f(o.push(1));",
                path: "file.ts",
                code: "export const o = []; f(o.push(1));",
                expected: &[(
                    13,
                    14,
                    "Possibly Global Data: exported `o` is module-level state that is mutated in this file, so any importer can read it while code anywhere changes it. Consider Encapsulate Variable: keep the state private and expose functions that own every read and write (in Effect, a `Ref` created in a service's layer and reached only through its operations).",
                )],
            },
            Case {
                name: "invalid 43: export const o = [] as const; o.reverse();",
                path: "file.ts",
                code: "export const o = [] as const; o.reverse();",
                expected: &[(
                    13,
                    14,
                    "Possibly Global Data: exported `o` is module-level state that is mutated in this file, so any importer can read it while code anywhere changes it. Consider Encapsulate Variable: keep the state private and expose functions that own every read and write (in Effect, a `Ref` created in a service's layer and reached only through its operations).",
                )],
            },
            Case {
                name: "invalid 44: export const o = <X[]>[]; o.fill(0);",
                path: "file.ts",
                code: "export const o = <X[]>[]; o.fill(0);",
                expected: &[(
                    13,
                    14,
                    "Possibly Global Data: exported `o` is module-level state that is mutated in this file, so any importer can read it while code anywhere changes it. Consider Encapsulate Variable: keep the state private and expose functions that own every read and write (in Effect, a `Ref` created in a service's layer and reached only through its operations).",
                )],
            },
            Case { name: "invalid 45: export let x = 1;", path: "src/__tests__/a.ts", code: "export let x = 1;", expected: &[] },
        ],
    );
}

/// Edge cases recorded from upstream under oxlint 1.86: src/martinfowler/rules/global-data.mfedges.test.ts, options `[{"ignoreTests":false}]`
#[test]
fn edges_options_1() {
    check(
        "anti-slop-martinfowler/global-data",
        "[{\"ignoreTests\":false}]",
        &[Case {
            name: "invalid 46: export let x = 1;",
            path: "src/__tests__/a.ts",
            code: "export let x = 1;",
            expected: &[(
                11,
                12,
                "Possibly Global Data: exported `x` is module-level state that is declared with `let`, so it can be reassigned, so any importer can read it while code anywhere changes it. Consider Encapsulate Variable: keep the state private and expose functions that own every read and write (in Effect, a `Ref` created in a service's layer and reached only through its operations).",
            )],
        }],
    );
}
