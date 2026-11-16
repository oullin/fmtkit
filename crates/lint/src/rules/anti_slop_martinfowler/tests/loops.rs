//! `anti-slop-martinfowler/loops`: expectations recorded from the upstream plugin
//! (github.com/k-mistele/anti-slop) under oxlint 1.86, one case per upstream
//! RuleTester case in `src/martinfowler/rules/loops.test.ts`.

use super::super::super::anti_slop::tests::{Case, check};

/// src/martinfowler/rules/loops.test.ts
#[test]
fn upstream() {
    check(
        "anti-slop-martinfowler/loops",
        "[]",
        &[
            Case {
                name: "valid 0: for (const user of users) { await notify(user); }",
                path: "file.ts",
                code: "for (const user of users) { await notify(user); }",
                expected: &[],
            },
            Case {
                name: "valid 1: const run = Effect.gen(function* () { for (const id of ids) {",
                path: "file.ts",
                code: "const run = Effect.gen(function* () { for (const id of ids) { yield* remove(id); } });",
                expected: &[],
            },
            Case {
                name: "valid 2: for (const row of rows) { const cells = []; cells.push(row.a)",
                path: "file.ts",
                code: "for (const row of rows) { const cells = []; cells.push(row.a); emit(cells); }",
                expected: &[],
            },
            Case {
                name: "valid 3: const out = []; for (const x of xs) { if (x.done) break; out.",
                path: "file.ts",
                code: "const out = []; for (const x of xs) { if (x.done) break; out.push(x); }",
                expected: &[],
            },
            Case {
                name: "valid 4: function f(xs) { const out = []; for (const x of xs) { if (!x",
                path: "file.ts",
                code: "function f(xs) { const out = []; for (const x of xs) { if (!x) return null; out.push(x); } return out; }",
                expected: &[],
            },
            Case {
                name: "valid 5: const queue = [root]; for (const node of queue) { queue.push(",
                path: "file.ts",
                code: "const queue = [root]; for (const node of queue) { queue.push(...node.children); }",
                expected: &[],
            },
            Case {
                name: "valid 6: const pages = []; let cursor; do { const page = await load(cu",
                path: "file.ts",
                code: "const pages = []; let cursor; do { const page = await load(cursor); pages.push(page); cursor = page.next; } while (cursor);",
                expected: &[],
            },
            Case {
                name: "valid 7: const parts = []; while (match = re.exec(text)) parts.push(ma",
                path: "file.ts",
                code: "const parts = []; while (match = re.exec(text)) parts.push(match[1]);",
                expected: &[],
            },
            Case {
                name: "valid 8: let total = 0; for (const x of xs) total += x.amount;",
                path: "file.ts",
                code: "let total = 0; for (const x of xs) total += x.amount;",
                expected: &[],
            },
            Case {
                name: "valid 9: const state = { done: false }; for (const x of xs) state.done",
                path: "file.ts",
                code: "const state = { done: false }; for (const x of xs) state.done = x.last;",
                expected: &[],
            },
            Case {
                name: "valid 10: const frames = []; const state = { afterCr: false }; for (co",
                path: "file.ts",
                code: "const frames = []; const state = { afterCr: false }; for (const byte of chunk) { if (byte === 13) state.afterCr = true; else frames.push(byte); }",
                expected: &[],
            },
            Case {
                name: "valid 11: const out = []; let depth = 0; for (const token of tokens) {",
                path: "file.ts",
                code: "const out = []; let depth = 0; for (const token of tokens) { depth += token.delta; out.push(depth); }",
                expected: &[],
            },
            Case {
                name: "valid 12: const saved = new Map(); for (const m of methods) { saved.se",
                path: "file.ts",
                code: "const saved = new Map(); for (const m of methods) { saved.set(m, console[m]); console[m] = noop; }",
                expected: &[],
            },
            Case {
                name: "valid 13: const out = new Uint8Array(n); for (let y = 0; y < h; y++) f",
                path: "file.ts",
                code: "const out = new Uint8Array(n); for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) out[y * w + x] = src[x * h + y];",
                expected: &[],
            },
            Case {
                name: "valid 14: const out = []; for (const x of xs) out.push(x);",
                path: "src/a.test.ts",
                code: "const out = []; for (const x of xs) out.push(x);",
                expected: &[],
            },
            Case {
                name: "valid 15: const out = []; for (const x of xs) { for (const y of x) { i",
                path: "file.ts",
                code: "const out = []; for (const x of xs) { for (const y of x) { if (y) break; } }",
                expected: &[],
            },
            Case {
                name: "invalid 16: const emails = []; for (const user of users) { if (user.ac",
                path: "file.ts",
                code: "const emails = []; for (const user of users) { if (user.active) emails.push(user.email); }",
                expected: &[(
                    19,
                    90,
                    "Possibly Loops: this loop builds `emails` by calling `.push` on each iteration, so the transformation is hidden inside control flow and mutation. Consider Replace Loop with Pipeline: express it as `map`/`filter`/`flatMap`/`groupBy` over the source (Effect's `Arr` and `Record` modules, or `Object.fromEntries`), or `Effect.forEach`/`Stream` when each iteration runs an effect.",
                )],
            },
            Case {
                name: "invalid 17: const byId = {}; for (const item of items) { const copy = ",
                path: "file.ts",
                code: "const byId = {}; for (const item of items) { const copy = { ...item }; copy.seen = true; byId[item.id] = copy; }",
                expected: &[(
                    17,
                    112,
                    "Possibly Loops: this loop builds `byId` by assigning computed keys on each iteration, so the transformation is hidden inside control flow and mutation. Consider Replace Loop with Pipeline: express it as `map`/`filter`/`flatMap`/`groupBy` over the source (Effect's `Arr` and `Record` modules, or `Object.fromEntries`), or `Effect.forEach`/`Stream` when each iteration runs an effect.",
                )],
            },
            Case {
                name: "invalid 18: const index = new Map(); for (let i = 0; i < rows.length; ",
                path: "file.ts",
                code: "const index = new Map(); for (let i = 0; i < rows.length; i++) index.set(rows[i].id, i);",
                expected: &[(
                    25,
                    88,
                    "Possibly Loops: this loop builds `index` by calling `.set` on each iteration, so the transformation is hidden inside control flow and mutation. Consider Replace Loop with Pipeline: express it as `map`/`filter`/`flatMap`/`groupBy` over the source (Effect's `Arr` and `Record` modules, or `Object.fromEntries`), or `Effect.forEach`/`Stream` when each iteration runs an effect.",
                )],
            },
            Case {
                name: "invalid 19: let all = []; for (const group of groups) { all = [...all,",
                path: "file.ts",
                code: "let all = []; for (const group of groups) { all = [...all, ...group.items]; }",
                expected: &[(
                    14,
                    77,
                    "Possibly Loops: this loop builds `all` by copying it with a spread on each iteration, so the transformation is hidden inside control flow and mutation. Consider Replace Loop with Pipeline: express it as `map`/`filter`/`flatMap`/`groupBy` over the source (Effect's `Arr` and `Record` modules, or `Object.fromEntries`), or `Effect.forEach`/`Stream` when each iteration runs an effect.",
                )],
            },
            Case {
                name: "invalid 20: let all = []; for (const key in record) all = all.concat(r",
                path: "file.ts",
                code: "let all = []; for (const key in record) all = all.concat(record[key]);",
                expected: &[(
                    14,
                    70,
                    "Possibly Loops: this loop builds `all` by calling `.concat` on each iteration, so the transformation is hidden inside control flow and mutation. Consider Replace Loop with Pipeline: express it as `map`/`filter`/`flatMap`/`groupBy` over the source (Effect's `Arr` and `Record` modules, or `Object.fromEntries`), or `Effect.forEach`/`Stream` when each iteration runs an effect.",
                )],
            },
            Case {
                name: "invalid 21: const results = []; for (const id of ids) { const user = y",
                path: "file.ts",
                code: "const results = []; for (const id of ids) { const user = yield* load(id); results.push(user); }",
                expected: &[(
                    20,
                    95,
                    "Possibly Loops: this loop builds `results` by calling `.push` on each iteration, so the transformation is hidden inside control flow and mutation. Consider Replace Loop with Pipeline: express it as `map`/`filter`/`flatMap`/`groupBy` over the source (Effect's `Arr` and `Record` modules, or `Object.fromEntries`), or `Effect.forEach`/`Stream` when each iteration runs an effect.",
                )],
            },
            Case {
                name: "invalid 22: const pairs = []; for (const a of xs) { for (const b of ys",
                path: "file.ts",
                code: "const pairs = []; for (const a of xs) { for (const b of ys) { pairs.push([a, b]); } }",
                expected: &[(
                    18,
                    85,
                    "Possibly Loops: this loop builds `pairs` by calling `.push` on each iteration, so the transformation is hidden inside control flow and mutation. Consider Replace Loop with Pipeline: express it as `map`/`filter`/`flatMap`/`groupBy` over the source (Effect's `Arr` and `Record` modules, or `Object.fromEntries`), or `Effect.forEach`/`Stream` when each iteration runs an effect.",
                )],
            },
            Case {
                name: "invalid 23: const out = []; for (const x of xs) { switch (x.kind) { ca",
                path: "file.ts",
                code: "const out = []; for (const x of xs) { switch (x.kind) { case 'a': out.push(x); break; } }",
                expected: &[(
                    16,
                    89,
                    "Possibly Loops: this loop builds `out` by calling `.push` on each iteration, so the transformation is hidden inside control flow and mutation. Consider Replace Loop with Pipeline: express it as `map`/`filter`/`flatMap`/`groupBy` over the source (Effect's `Arr` and `Record` modules, or `Object.fromEntries`), or `Effect.forEach`/`Stream` when each iteration runs an effect.",
                )],
            },
        ],
    );
}

/// src/martinfowler/rules/loops.test.ts, options `[{"includeWhile":true}]`
#[test]
fn upstream_options_1() {
    check(
        "anti-slop-martinfowler/loops",
        "[{\"includeWhile\":true}]",
        &[Case {
            name: "invalid 24: const pages = []; while (hasMore()) pages.push(next());",
            path: "file.ts",
            code: "const pages = []; while (hasMore()) pages.push(next());",
            expected: &[(
                18,
                55,
                "Possibly Loops: this loop builds `pages` by calling `.push` on each iteration, so the transformation is hidden inside control flow and mutation. Consider Replace Loop with Pipeline: express it as `map`/`filter`/`flatMap`/`groupBy` over the source (Effect's `Arr` and `Record` modules, or `Object.fromEntries`), or `Effect.forEach`/`Stream` when each iteration runs an effect.",
            )],
        }],
    );
}

/// Edge cases recorded from upstream under oxlint 1.86: src/martinfowler/rules/loops.mfedges.test.ts
#[test]
fn edges() {
    check(
        "anti-slop-martinfowler/loops",
        "[]",
        &[
            Case {
                name: "invalid 0: const out = []; outer: for (const a of xs) { for (const b o",
                path: "file.ts",
                code: "const out = []; outer: for (const a of xs) { for (const b of a) { if (b) break outer; out.push(b); } }",
                expected: &[],
            },
            Case {
                name: "invalid 1: const out = []; outer: for (const a of xs) { for (const b o",
                path: "file.ts",
                code: "const out = []; outer: for (const a of xs) { for (const b of a) { if (b) continue outer; out.push(b); } }",
                expected: &[(
                    23,
                    105,
                    "Possibly Loops: this loop builds `out` by calling `.push` on each iteration, so the transformation is hidden inside control flow and mutation. Consider Replace Loop with Pipeline: express it as `map`/`filter`/`flatMap`/`groupBy` over the source (Effect's `Arr` and `Record` modules, or `Object.fromEntries`), or `Effect.forEach`/`Stream` when each iteration runs an effect.",
                )],
            },
            Case {
                name: "invalid 2: const out = []; for (const a of xs) { inner: { if (a) break",
                path: "file.ts",
                code: "const out = []; for (const a of xs) { inner: { if (a) break inner; out.push(a); } }",
                expected: &[],
            },
            Case {
                name: "invalid 3: const out = []; for (const a of xs) { if (!a) continue; out",
                path: "file.ts",
                code: "const out = []; for (const a of xs) { if (!a) continue; out.push(a); }",
                expected: &[(
                    16,
                    70,
                    "Possibly Loops: this loop builds `out` by calling `.push` on each iteration, so the transformation is hidden inside control flow and mutation. Consider Replace Loop with Pipeline: express it as `map`/`filter`/`flatMap`/`groupBy` over the source (Effect's `Arr` and `Record` modules, or `Object.fromEntries`), or `Effect.forEach`/`Stream` when each iteration runs an effect.",
                )],
            },
            Case {
                name: "invalid 4: const out = []; for (const a of xs) { xs.forEach(() => { re",
                path: "file.ts",
                code: "const out = []; for (const a of xs) { xs.forEach(() => { return; }); out.push(a); }",
                expected: &[(
                    16,
                    83,
                    "Possibly Loops: this loop builds `out` by calling `.push` on each iteration, so the transformation is hidden inside control flow and mutation. Consider Replace Loop with Pipeline: express it as `map`/`filter`/`flatMap`/`groupBy` over the source (Effect's `Arr` and `Record` modules, or `Object.fromEntries`), or `Effect.forEach`/`Stream` when each iteration runs an effect.",
                )],
            },
            Case {
                name: "invalid 5: const out = []; for (const a of xs) { function g() { return",
                path: "file.ts",
                code: "const out = []; for (const a of xs) { function g() { return 1; } out.push(a); }",
                expected: &[(
                    16,
                    79,
                    "Possibly Loops: this loop builds `out` by calling `.push` on each iteration, so the transformation is hidden inside control flow and mutation. Consider Replace Loop with Pipeline: express it as `map`/`filter`/`flatMap`/`groupBy` over the source (Effect's `Arr` and `Record` modules, or `Object.fromEntries`), or `Effect.forEach`/`Stream` when each iteration runs an effect.",
                )],
            },
            Case {
                name: "invalid 6: const out = []; let count = 0; for (const x of xs) { out.pu",
                path: "file.ts",
                code: "const out = []; let count = 0; for (const x of xs) { out.push(x); count++; }",
                expected: &[],
            },
            Case {
                name: "invalid 7: const out = {}; for (const x of xs) out[x.id] = x;",
                path: "file.ts",
                code: "const out = {}; for (const x of xs) out[x.id] = x;",
                expected: &[(
                    16,
                    50,
                    "Possibly Loops: this loop builds `out` by assigning computed keys on each iteration, so the transformation is hidden inside control flow and mutation. Consider Replace Loop with Pipeline: express it as `map`/`filter`/`flatMap`/`groupBy` over the source (Effect's `Arr` and `Record` modules, or `Object.fromEntries`), or `Effect.forEach`/`Stream` when each iteration runs an effect.",
                )],
            },
            Case {
                name: "invalid 8: const out = []; for (let i = 0; i < n; i++) out[i] = x;",
                path: "file.ts",
                code: "const out = []; for (let i = 0; i < n; i++) out[i] = x;",
                expected: &[],
            },
            Case {
                name: "invalid 9: const out = []; for (let i = 0; i < n; i++) out.push(i);",
                path: "file.ts",
                code: "const out = []; for (let i = 0; i < n; i++) out.push(i);",
                expected: &[(
                    16,
                    56,
                    "Possibly Loops: this loop builds `out` by calling `.push` on each iteration, so the transformation is hidden inside control flow and mutation. Consider Replace Loop with Pipeline: express it as `map`/`filter`/`flatMap`/`groupBy` over the source (Effect's `Arr` and `Record` modules, or `Object.fromEntries`), or `Effect.forEach`/`Stream` when each iteration runs an effect.",
                )],
            },
            Case {
                name: "invalid 10: const m = new Map(); for (const k in o) { m.set(k, o[k]); ",
                path: "file.ts",
                code: "const m = new Map(); for (const k in o) { m.set(k, o[k]); }",
                expected: &[(
                    21,
                    59,
                    "Possibly Loops: this loop builds `m` by calling `.set` on each iteration, so the transformation is hidden inside control flow and mutation. Consider Replace Loop with Pipeline: express it as `map`/`filter`/`flatMap`/`groupBy` over the source (Effect's `Arr` and `Record` modules, or `Object.fromEntries`), or `Effect.forEach`/`Stream` when each iteration runs an effect.",
                )],
            },
            Case {
                name: "invalid 11: for (const x of xs) { var acc = []; acc.push(x); }",
                path: "file.ts",
                code: "for (const x of xs) { var acc = []; acc.push(x); }",
                expected: &[],
            },
            Case {
                name: "invalid 12: const out = []; for (const x of xs) (out as any[]).push(x)",
                path: "file.ts",
                code: "const out = []; for (const x of xs) (out as any[]).push(x);",
                expected: &[(
                    16,
                    59,
                    "Possibly Loops: this loop builds `out` by calling `.push` on each iteration, so the transformation is hidden inside control flow and mutation. Consider Replace Loop with Pipeline: express it as `map`/`filter`/`flatMap`/`groupBy` over the source (Effect's `Arr` and `Record` modules, or `Object.fromEntries`), or `Effect.forEach`/`Stream` when each iteration runs an effect.",
                )],
            },
            Case {
                name: "invalid 13: const out = []; for (const x of xs) out?.push(x);",
                path: "file.ts",
                code: "const out = []; for (const x of xs) out?.push(x);",
                expected: &[(
                    16,
                    49,
                    "Possibly Loops: this loop builds `out` by calling `.push` on each iteration, so the transformation is hidden inside control flow and mutation. Consider Replace Loop with Pipeline: express it as `map`/`filter`/`flatMap`/`groupBy` over the source (Effect's `Arr` and `Record` modules, or `Object.fromEntries`), or `Effect.forEach`/`Stream` when each iteration runs an effect.",
                )],
            },
            Case {
                name: "invalid 14: const out = []; for (const x of xs) out!.push(x);",
                path: "file.ts",
                code: "const out = []; for (const x of xs) out!.push(x);",
                expected: &[(
                    16,
                    49,
                    "Possibly Loops: this loop builds `out` by calling `.push` on each iteration, so the transformation is hidden inside control flow and mutation. Consider Replace Loop with Pipeline: express it as `map`/`filter`/`flatMap`/`groupBy` over the source (Effect's `Arr` and `Record` modules, or `Object.fromEntries`), or `Effect.forEach`/`Stream` when each iteration runs an effect.",
                )],
            },
            Case {
                name: "invalid 15: let out = []; for (const x of xs) out = (out).concat(x);",
                path: "file.ts",
                code: "let out = []; for (const x of xs) out = (out).concat(x);",
                expected: &[(
                    14,
                    56,
                    "Possibly Loops: this loop builds `out` by calling `.concat` on each iteration, so the transformation is hidden inside control flow and mutation. Consider Replace Loop with Pipeline: express it as `map`/`filter`/`flatMap`/`groupBy` over the source (Effect's `Arr` and `Record` modules, or `Object.fromEntries`), or `Effect.forEach`/`Stream` when each iteration runs an effect.",
                )],
            },
            Case {
                name: "invalid 16: let out = []; for (const x of xs) out = out.concat(x) as a",
                path: "file.ts",
                code: "let out = []; for (const x of xs) out = out.concat(x) as any;",
                expected: &[(
                    14,
                    61,
                    "Possibly Loops: this loop builds `out` by calling `.concat` on each iteration, so the transformation is hidden inside control flow and mutation. Consider Replace Loop with Pipeline: express it as `map`/`filter`/`flatMap`/`groupBy` over the source (Effect's `Arr` and `Record` modules, or `Object.fromEntries`), or `Effect.forEach`/`Stream` when each iteration runs an effect.",
                )],
            },
            Case {
                name: "invalid 17: let obj = {}; for (const x of xs) obj = { ...obj, [x]: 1 }",
                path: "file.ts",
                code: "let obj = {}; for (const x of xs) obj = { ...obj, [x]: 1 };",
                expected: &[(
                    14,
                    59,
                    "Possibly Loops: this loop builds `obj` by copying it with a spread on each iteration, so the transformation is hidden inside control flow and mutation. Consider Replace Loop with Pipeline: express it as `map`/`filter`/`flatMap`/`groupBy` over the source (Effect's `Arr` and `Record` modules, or `Object.fromEntries`), or `Effect.forEach`/`Stream` when each iteration runs an effect.",
                )],
            },
            Case {
                name: "invalid 18: let obj = {}; for (const x of xs) obj = { ...(obj as any),",
                path: "file.ts",
                code: "let obj = {}; for (const x of xs) obj = { ...(obj as any), [x]: 1 };",
                expected: &[(
                    14,
                    68,
                    "Possibly Loops: this loop builds `obj` by copying it with a spread on each iteration, so the transformation is hidden inside control flow and mutation. Consider Replace Loop with Pipeline: express it as `map`/`filter`/`flatMap`/`groupBy` over the source (Effect's `Arr` and `Record` modules, or `Object.fromEntries`), or `Effect.forEach`/`Stream` when each iteration runs an effect.",
                )],
            },
            Case {
                name: "invalid 19: let obj = {}; for (const x of xs) obj += x;",
                path: "file.ts",
                code: "let obj = {}; for (const x of xs) obj += x;",
                expected: &[],
            },
            Case {
                name: "invalid 20: const out = []; for (const x of out.items) out.push(x);",
                path: "file.ts",
                code: "const out = []; for (const x of out.items) out.push(x);",
                expected: &[],
            },
            Case {
                name: "invalid 21: const out = []; for (const x of data.out) out.push(x);",
                path: "file.ts",
                code: "const out = []; for (const x of data.out) out.push(x);",
                expected: &[],
            },
            Case {
                name: "invalid 22: const out = []; for (const a of xs) { while (c) { out.push",
                path: "file.ts",
                code: "const out = []; for (const a of xs) { while (c) { out.push(a); } }",
                expected: &[(
                    16,
                    66,
                    "Possibly Loops: this loop builds `out` by calling `.push` on each iteration, so the transformation is hidden inside control flow and mutation. Consider Replace Loop with Pipeline: express it as `map`/`filter`/`flatMap`/`groupBy` over the source (Effect's `Arr` and `Record` modules, or `Object.fromEntries`), or `Effect.forEach`/`Stream` when each iteration runs an effect.",
                )],
            },
            Case {
                name: "invalid 23: const out = []; for (const a of xs) { for (const b of a) {",
                path: "file.ts",
                code: "const out = []; for (const a of xs) { for (const b of a) { out.push(b); } }",
                expected: &[(
                    16,
                    75,
                    "Possibly Loops: this loop builds `out` by calling `.push` on each iteration, so the transformation is hidden inside control flow and mutation. Consider Replace Loop with Pipeline: express it as `map`/`filter`/`flatMap`/`groupBy` over the source (Effect's `Arr` and `Record` modules, or `Object.fromEntries`), or `Effect.forEach`/`Stream` when each iteration runs an effect.",
                )],
            },
            Case {
                name: "invalid 24: const out = []; for (const a of xs) { if (a) return; for (",
                path: "file.ts",
                code: "const out = []; for (const a of xs) { if (a) return; for (const b of a) { out.push(b); } }",
                expected: &[(
                    53,
                    88,
                    "Possibly Loops: this loop builds `out` by calling `.push` on each iteration, so the transformation is hidden inside control flow and mutation. Consider Replace Loop with Pipeline: express it as `map`/`filter`/`flatMap`/`groupBy` over the source (Effect's `Arr` and `Record` modules, or `Object.fromEntries`), or `Effect.forEach`/`Stream` when each iteration runs an effect.",
                )],
            },
            Case {
                name: "invalid 25: const a = [], b = []; for (const x of xs) { a.push(x); b.p",
                path: "file.ts",
                code: "const a = [], b = []; for (const x of xs) { a.push(x); b.push(x); }",
                expected: &[(
                    22,
                    67,
                    "Possibly Loops: this loop builds `b` by calling `.push` on each iteration, so the transformation is hidden inside control flow and mutation. Consider Replace Loop with Pipeline: express it as `map`/`filter`/`flatMap`/`groupBy` over the source (Effect's `Arr` and `Record` modules, or `Object.fromEntries`), or `Effect.forEach`/`Stream` when each iteration runs an effect.",
                )],
            },
            Case {
                name: "invalid 26: const a = []; let b = []; for (const x of xs) { a.push(x);",
                path: "file.ts",
                code: "const a = []; let b = []; for (const x of xs) { a.push(x); b = [...b, x]; }",
                expected: &[(
                    26,
                    75,
                    "Possibly Loops: this loop builds `b` by copying it with a spread on each iteration, so the transformation is hidden inside control flow and mutation. Consider Replace Loop with Pipeline: express it as `map`/`filter`/`flatMap`/`groupBy` over the source (Effect's `Arr` and `Record` modules, or `Object.fromEntries`), or `Effect.forEach`/`Stream` when each iteration runs an effect.",
                )],
            },
            Case {
                name: "invalid 27: const a = []; let b = []; for (const x of xs) { b = [...b,",
                path: "file.ts",
                code: "const a = []; let b = []; for (const x of xs) { b = [...b, x]; a.push(x); }",
                expected: &[],
            },
            Case {
                name: "invalid 28: let all = []; for (const g of gs) { all.push(g); all = all",
                path: "file.ts",
                code: "let all = []; for (const g of gs) { all.push(g); all = all; }",
                expected: &[(
                    14,
                    61,
                    "Possibly Loops: this loop builds `all` by calling `.push` on each iteration, so the transformation is hidden inside control flow and mutation. Consider Replace Loop with Pipeline: express it as `map`/`filter`/`flatMap`/`groupBy` over the source (Effect's `Arr` and `Record` modules, or `Object.fromEntries`), or `Effect.forEach`/`Stream` when each iteration runs an effect.",
                )],
            },
            Case { name: "invalid 29: for (const x of xs) out.push(x);", path: "file.ts", code: "for (const x of xs) out.push(x);", expected: &[] },
            Case {
                name: "invalid 30: function f(out) { for (const x of xs) out.push(x); }",
                path: "file.ts",
                code: "function f(out) { for (const x of xs) out.push(x); }",
                expected: &[(
                    18,
                    50,
                    "Possibly Loops: this loop builds `out` by calling `.push` on each iteration, so the transformation is hidden inside control flow and mutation. Consider Replace Loop with Pipeline: express it as `map`/`filter`/`flatMap`/`groupBy` over the source (Effect's `Arr` and `Record` modules, or `Object.fromEntries`), or `Effect.forEach`/`Stream` when each iteration runs an effect.",
                )],
            },
            Case {
                name: "invalid 31: function f() { const out = []; for (const x of xs) out.pus",
                path: "file.ts",
                code: "function f() { const out = []; for (const x of xs) out.push(x); return out; }",
                expected: &[(
                    31,
                    63,
                    "Possibly Loops: this loop builds `out` by calling `.push` on each iteration, so the transformation is hidden inside control flow and mutation. Consider Replace Loop with Pipeline: express it as `map`/`filter`/`flatMap`/`groupBy` over the source (Effect's `Arr` and `Record` modules, or `Object.fromEntries`), or `Effect.forEach`/`Stream` when each iteration runs an effect.",
                )],
            },
            Case {
                name: "invalid 32: const out = []; for (const x of xs) { const out = []; out.",
                path: "file.ts",
                code: "const out = []; for (const x of xs) { const out = []; out.push(x); }",
                expected: &[],
            },
            Case {
                name: "invalid 33: const out = []; for (const x of xs) out.add(x).add(x);",
                path: "file.ts",
                code: "const out = []; for (const x of xs) out.add(x).add(x);",
                expected: &[(
                    16,
                    54,
                    "Possibly Loops: this loop builds `out` by calling `.add` on each iteration, so the transformation is hidden inside control flow and mutation. Consider Replace Loop with Pipeline: express it as `map`/`filter`/`flatMap`/`groupBy` over the source (Effect's `Arr` and `Record` modules, or `Object.fromEntries`), or `Effect.forEach`/`Stream` when each iteration runs an effect.",
                )],
            },
            Case {
                name: "invalid 34: const out = []; for (const x of xs) out.items.push(x);",
                path: "file.ts",
                code: "const out = []; for (const x of xs) out.items.push(x);",
                expected: &[],
            },
            Case {
                name: "invalid 35: const out = []; for (const x of xs) this.out.push(x);",
                path: "file.ts",
                code: "const out = []; for (const x of xs) this.out.push(x);",
                expected: &[],
            },
            Case {
                name: "invalid 36: const out = []; for (const x of xs) out[x] += 1;",
                path: "file.ts",
                code: "const out = []; for (const x of xs) out[x] += 1;",
                expected: &[(
                    16,
                    48,
                    "Possibly Loops: this loop builds `out` by assigning computed keys on each iteration, so the transformation is hidden inside control flow and mutation. Consider Replace Loop with Pipeline: express it as `map`/`filter`/`flatMap`/`groupBy` over the source (Effect's `Arr` and `Record` modules, or `Object.fromEntries`), or `Effect.forEach`/`Stream` when each iteration runs an effect.",
                )],
            },
            Case {
                name: "invalid 37: const out = []; for (const x of xs) { switch (x) { case 1:",
                path: "file.ts",
                code: "const out = []; for (const x of xs) { switch (x) { case 1: continue; } out.push(x); }",
                expected: &[(
                    16,
                    85,
                    "Possibly Loops: this loop builds `out` by calling `.push` on each iteration, so the transformation is hidden inside control flow and mutation. Consider Replace Loop with Pipeline: express it as `map`/`filter`/`flatMap`/`groupBy` over the source (Effect's `Arr` and `Record` modules, or `Object.fromEntries`), or `Effect.forEach`/`Stream` when each iteration runs an effect.",
                )],
            },
            Case {
                name: "invalid 38: const out = []; for (const x of xs) { try { out.push(x); }",
                path: "file.ts",
                code: "const out = []; for (const x of xs) { try { out.push(x); } catch { break; } }",
                expected: &[],
            },
            Case {
                name: "invalid 39: const out = []; for (const x of xs) { do { break; } while ",
                path: "file.ts",
                code: "const out = []; for (const x of xs) { do { break; } while (x); out.push(x); }",
                expected: &[(
                    16,
                    77,
                    "Possibly Loops: this loop builds `out` by calling `.push` on each iteration, so the transformation is hidden inside control flow and mutation. Consider Replace Loop with Pipeline: express it as `map`/`filter`/`flatMap`/`groupBy` over the source (Effect's `Arr` and `Record` modules, or `Object.fromEntries`), or `Effect.forEach`/`Stream` when each iteration runs an effect.",
                )],
            },
            Case {
                name: "invalid 40: const out = []; for (;;) out.push(1);",
                path: "file.ts",
                code: "const out = []; for (;;) out.push(1);",
                expected: &[(
                    16,
                    37,
                    "Possibly Loops: this loop builds `out` by calling `.push` on each iteration, so the transformation is hidden inside control flow and mutation. Consider Replace Loop with Pipeline: express it as `map`/`filter`/`flatMap`/`groupBy` over the source (Effect's `Arr` and `Record` modules, or `Object.fromEntries`), or `Effect.forEach`/`Stream` when each iteration runs an effect.",
                )],
            },
            Case {
                name: "invalid 41: const out = []; let i = 0; for (; i < n; ) { out.push(i); ",
                path: "file.ts",
                code: "const out = []; let i = 0; for (; i < n; ) { out.push(i); }",
                expected: &[(
                    27,
                    59,
                    "Possibly Loops: this loop builds `out` by calling `.push` on each iteration, so the transformation is hidden inside control flow and mutation. Consider Replace Loop with Pipeline: express it as `map`/`filter`/`flatMap`/`groupBy` over the source (Effect's `Arr` and `Record` modules, or `Object.fromEntries`), or `Effect.forEach`/`Stream` when each iteration runs an effect.",
                )],
            },
            Case {
                name: "invalid 42: const out = []; for (const x of xs) { out.push(x); xs.leng",
                path: "file.ts",
                code: "const out = []; for (const x of xs) { out.push(x); xs.length = 0; }",
                expected: &[],
            },
            Case {
                name: "invalid 43: const out = []; for (const x of xs) { out.push(x); x.seen ",
                path: "file.ts",
                code: "const out = []; for (const x of xs) { out.push(x); x.seen = true; }",
                expected: &[(
                    16,
                    67,
                    "Possibly Loops: this loop builds `out` by calling `.push` on each iteration, so the transformation is hidden inside control flow and mutation. Consider Replace Loop with Pipeline: express it as `map`/`filter`/`flatMap`/`groupBy` over the source (Effect's `Arr` and `Record` modules, or `Object.fromEntries`), or `Effect.forEach`/`Stream` when each iteration runs an effect.",
                )],
            },
            Case {
                name: "invalid 44: const out = []; for (const x of xs) { out.push(x); out.len",
                path: "file.ts",
                code: "const out = []; for (const x of xs) { out.push(x); out.length = 0; }",
                expected: &[(
                    16,
                    68,
                    "Possibly Loops: this loop builds `out` by calling `.push` on each iteration, so the transformation is hidden inside control flow and mutation. Consider Replace Loop with Pipeline: express it as `map`/`filter`/`flatMap`/`groupBy` over the source (Effect's `Arr` and `Record` modules, or `Object.fromEntries`), or `Effect.forEach`/`Stream` when each iteration runs an effect.",
                )],
            },
            Case {
                name: "invalid 45: const out = []; for (const x of xs) out.push(() => { other",
                path: "file.ts",
                code: "const out = []; for (const x of xs) out.push(() => { other = 1; });",
                expected: &[],
            },
            Case {
                name: "invalid 46: const out = []; for (const x of xs) { label: out.push(x); ",
                path: "file.ts",
                code: "const out = []; for (const x of xs) { label: out.push(x); }",
                expected: &[(
                    16,
                    59,
                    "Possibly Loops: this loop builds `out` by calling `.push` on each iteration, so the transformation is hidden inside control flow and mutation. Consider Replace Loop with Pipeline: express it as `map`/`filter`/`flatMap`/`groupBy` over the source (Effect's `Arr` and `Record` modules, or `Object.fromEntries`), or `Effect.forEach`/`Stream` when each iteration runs an effect.",
                )],
            },
            Case {
                name: "invalid 47: const out = []; for (const [k, v] of Object.entries(o)) ou",
                path: "file.ts",
                code: "const out = []; for (const [k, v] of Object.entries(o)) out.push(k + v);",
                expected: &[(
                    16,
                    72,
                    "Possibly Loops: this loop builds `out` by calling `.push` on each iteration, so the transformation is hidden inside control flow and mutation. Consider Replace Loop with Pipeline: express it as `map`/`filter`/`flatMap`/`groupBy` over the source (Effect's `Arr` and `Record` modules, or `Object.fromEntries`), or `Effect.forEach`/`Stream` when each iteration runs an effect.",
                )],
            },
            Case {
                name: "invalid 51: const out = []; for (const x of xs) out.push(x);",
                path: "src/a.spec.mts",
                code: "const out = []; for (const x of xs) out.push(x);",
                expected: &[],
            },
        ],
    );
}

/// Edge cases recorded from upstream under oxlint 1.86: src/martinfowler/rules/loops.mfedges.test.ts, options `[{"includeWhile":true}]`
#[test]
fn edges_options_1() {
    check(
        "anti-slop-martinfowler/loops",
        "[{\"includeWhile\":true}]",
        &[
            Case {
                name: "invalid 48: const out = []; while (c) { out.push(1); for (const x of x",
                path: "file.ts",
                code: "const out = []; while (c) { out.push(1); for (const x of xs) out.push(x); }",
                expected: &[(
                    16,
                    75,
                    "Possibly Loops: this loop builds `out` by calling `.push` on each iteration, so the transformation is hidden inside control flow and mutation. Consider Replace Loop with Pipeline: express it as `map`/`filter`/`flatMap`/`groupBy` over the source (Effect's `Arr` and `Record` modules, or `Object.fromEntries`), or `Effect.forEach`/`Stream` when each iteration runs an effect.",
                )],
            },
            Case {
                name: "invalid 49: const out = []; do { out.push(1); } while (c);",
                path: "file.ts",
                code: "const out = []; do { out.push(1); } while (c);",
                expected: &[(
                    16,
                    46,
                    "Possibly Loops: this loop builds `out` by calling `.push` on each iteration, so the transformation is hidden inside control flow and mutation. Consider Replace Loop with Pipeline: express it as `map`/`filter`/`flatMap`/`groupBy` over the source (Effect's `Arr` and `Record` modules, or `Object.fromEntries`), or `Effect.forEach`/`Stream` when each iteration runs an effect.",
                )],
            },
            Case {
                name: "invalid 50: const out = []; while (c) { if (d) break; out.push(1); }",
                path: "file.ts",
                code: "const out = []; while (c) { if (d) break; out.push(1); }",
                expected: &[],
            },
        ],
    );
}

/// Edge cases recorded from upstream under oxlint 1.86: src/martinfowler/rules/loops.mfedges.test.ts, options `[{"ignoreTests":false}]`
#[test]
fn edges_options_2() {
    check(
        "anti-slop-martinfowler/loops",
        "[{\"ignoreTests\":false}]",
        &[Case {
            name: "invalid 52: const out = []; for (const x of xs) out.push(x);",
            path: "src/a.spec.mts",
            code: "const out = []; for (const x of xs) out.push(x);",
            expected: &[(
                16,
                48,
                "Possibly Loops: this loop builds `out` by calling `.push` on each iteration, so the transformation is hidden inside control flow and mutation. Consider Replace Loop with Pipeline: express it as `map`/`filter`/`flatMap`/`groupBy` over the source (Effect's `Arr` and `Record` modules, or `Object.fromEntries`), or `Effect.forEach`/`Stream` when each iteration runs an effect.",
            )],
        }],
    );
}
