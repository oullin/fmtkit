//! `anti-slop/no-reduce-accumulator-copy`: expectations recorded from the upstream plugin
//! (github.com/k-mistele/anti-slop) under oxlint 1.86, one case per upstream
//! RuleTester case in `src/rules/no-reduce-accumulator-copy.test.ts`.

use super::{Case, check};

/// src/rules/no-reduce-accumulator-copy.test.ts
#[test]
fn upstream() {
    check(
        "anti-slop/no-reduce-accumulator-copy",
        "[]",
        &[
            Case {
                name: "valid 0: items.reduce((acc, item) => { acc.push(item); return acc; }, ",
                path: "file.ts",
                code: "items.reduce((acc, item) => { acc.push(item); return acc; }, []);",
                expected: &[],
            },
            Case {
                name: "valid 1: items.reduce((acc, item) => Object.assign(acc, item), {});",
                path: "file.ts",
                code: "items.reduce((acc, item) => Object.assign(acc, item), {});",
                expected: &[],
            },
            Case {
                name: "valid 2: items.reduce((acc, item) => Object.assign(acc, acc, item), {}",
                path: "file.ts",
                code: "items.reduce((acc, item) => Object.assign(acc, acc, item), {});",
                expected: &[],
            },
            Case {
                name: "valid 3: items.reduce((acc, item) => { acc[item.id] = { ...item }; ret",
                path: "file.ts",
                code: "items.reduce((acc, item) => { acc[item.id] = { ...item }; return acc; }, {});",
                expected: &[],
            },
            Case {
                name: "valid 4: items.reduce((acc, item) => { acc.push(Object.assign({}, item",
                path: "file.ts",
                code: "items.reduce((acc, item) => { acc.push(Object.assign({}, item)); return acc; }, []);",
                expected: &[],
            },
            Case {
                name: "valid 5: items.reduce((acc, item) => { acc.push(item.slice()); return ",
                path: "file.ts",
                code: "items.reduce((acc, item) => { acc.push(item.slice()); return acc; }, []);",
                expected: &[],
            },
            Case {
                name: "valid 6: items.reduce((acc, item) => acc.concat(item), '');",
                path: "file.ts",
                code: "items.reduce((acc, item) => acc.concat(item), '');",
                expected: &[],
            },
            Case {
                name: "valid 7: items.reduce((acc, item) => acc.concat(item), customCollectio",
                path: "file.ts",
                code: "items.reduce((acc, item) => acc.concat(item), customCollection);",
                expected: &[],
            },
            Case {
                name: "valid 8: function copy(acc) { return Object.assign({}, acc); }",
                path: "file.ts",
                code: "function copy(acc) { return Object.assign({}, acc); }",
                expected: &[],
            },
            Case {
                name: "valid 9: items.map((acc, item) => Object.assign({}, acc));",
                path: "file.ts",
                code: "items.map((acc, item) => Object.assign({}, acc));",
                expected: &[],
            },
            Case {
                name: "valid 10: items.reduce((acc, item) => { function copy(acc) { return Ob",
                path: "file.ts",
                code: "items.reduce((acc, item) => { function copy(acc) { return Object.assign({}, acc); } return acc; }, {});",
                expected: &[],
            },
            Case {
                name: "valid 11: items.reduce((acc, item) => { const snapshot = () => Object.",
                path: "file.ts",
                code: "items.reduce((acc, item) => { const snapshot = () => Object.assign({}, acc); return acc; }, {});",
                expected: &[],
            },
            Case {
                name: "valid 12: items.reduce((acc, item) => { { const acc = {}; Object.assig",
                path: "file.ts",
                code: "items.reduce((acc, item) => { { const acc = {}; Object.assign({}, acc); } return acc; }, {});",
                expected: &[],
            },
            Case {
                name: "valid 13: const Object = custom; items.reduce((acc, item) => Object.as",
                path: "file.ts",
                code: "const Object = custom; items.reduce((acc, item) => Object.assign({}, acc), {});",
                expected: &[],
            },
            Case {
                name: "valid 14: function run(Object) { return items.reduce((acc, item) => Ob",
                path: "file.ts",
                code: "function run(Object) { return items.reduce((acc, item) => Object.assign({}, acc), {}); }",
                expected: &[],
            },
            Case {
                name: "valid 15: const Array = custom; items.reduce((acc, item) => Array.from",
                path: "file.ts",
                code: "const Array = custom; items.reduce((acc, item) => Array.from(acc), []);",
                expected: &[],
            },
            Case {
                name: "valid 16: items.reduce((acc, item) => { let alias = acc; alias = item;",
                path: "file.ts",
                code: "items.reduce((acc, item) => { let alias = acc; alias = item; return Object.assign({}, alias); }, {});",
                expected: &[],
            },
            Case {
                name: "valid 17: items.reduce((acc, item) => [...acc, item], []);",
                path: "file.ts",
                code: "items.reduce((acc, item) => [...acc, item], []);",
                expected: &[],
            },
            Case {
                name: "valid 18: items.reduce((acc, item) => ({ ...acc, [item.id]: item }), {",
                path: "file.ts",
                code: "items.reduce((acc, item) => ({ ...acc, [item.id]: item }), {});",
                expected: &[],
            },
            Case {
                name: "invalid 19: items.reduce((acc, item) => Object.assign({}, acc, { [item",
                path: "file.ts",
                code: "items.reduce((acc, item) => Object.assign({}, acc, { [item.id]: item }), {});",
                expected: &[(
                    28,
                    71,
                    "Do not copy the reducer accumulator on every iteration; growing copies can cause quadratic work. Mutate a fresh, locally owned accumulator and return it, or use an iterator pipeline/flatMap.",
                )],
            },
            Case {
                name: "invalid 20: items.reduceRight((acc, item) => Object.assign({}, acc, it",
                path: "file.ts",
                code: "items.reduceRight((acc, item) => Object.assign({}, acc, item), {});",
                expected: &[(
                    33,
                    61,
                    "Do not copy the reducer accumulator on every iteration; growing copies can cause quadratic work. Mutate a fresh, locally owned accumulator and return it, or use an iterator pipeline/flatMap.",
                )],
            },
            Case {
                name: "invalid 21: items.reduce((acc, item, index, array) => Object.assign({}",
                path: "file.ts",
                code: "items.reduce((acc, item, index, array) => Object.assign({}, acc, item), {});",
                expected: &[(
                    42,
                    70,
                    "Do not copy the reducer accumulator on every iteration; growing copies can cause quadratic work. Mutate a fresh, locally owned accumulator and return it, or use an iterator pipeline/flatMap.",
                )],
            },
            Case {
                name: "invalid 22: items.reduce(acc => Object.assign({}, acc), {});",
                path: "file.ts",
                code: "items.reduce(acc => Object.assign({}, acc), {});",
                expected: &[(
                    20,
                    42,
                    "Do not copy the reducer accumulator on every iteration; growing copies can cause quadratic work. Mutate a fresh, locally owned accumulator and return it, or use an iterator pipeline/flatMap.",
                )],
            },
            Case {
                name: "invalid 23: items.reduce(function (acc, item) { return Object.assign({",
                path: "file.ts",
                code: "items.reduce(function (acc, item) { return Object.assign({}, item, acc); }, {});",
                expected: &[(
                    43,
                    71,
                    "Do not copy the reducer accumulator on every iteration; growing copies can cause quadratic work. Mutate a fresh, locally owned accumulator and return it, or use an iterator pipeline/flatMap.",
                )],
            },
            Case {
                name: "invalid 24: items['reduce'](((acc, item) => Object['assign']({}, acc, ",
                path: "file.ts",
                code: "items['reduce'](((acc, item) => Object['assign']({}, acc, item)), {});",
                expected: &[(
                    32,
                    63,
                    "Do not copy the reducer accumulator on every iteration; growing copies can cause quadratic work. Mutate a fresh, locally owned accumulator and return it, or use an iterator pipeline/flatMap.",
                )],
            },
            Case {
                name: "invalid 25: items.reduce((acc = {}, item) => Object.assign({}, acc, it",
                path: "file.ts",
                code: "items.reduce((acc = {}, item) => Object.assign({}, acc, item), {});",
                expected: &[(
                    33,
                    61,
                    "Do not copy the reducer accumulator on every iteration; growing copies can cause quadratic work. Mutate a fresh, locally owned accumulator and return it, or use an iterator pipeline/flatMap.",
                )],
            },
            Case {
                name: "invalid 26: items.reduce((acc, item) => { const alias = acc; return Ob",
                path: "file.ts",
                code: "items.reduce((acc, item) => { const alias = acc; return Object.assign({}, alias, item); }, {});",
                expected: &[(
                    56,
                    86,
                    "Do not copy the reducer accumulator on every iteration; growing copies can cause quadratic work. Mutate a fresh, locally owned accumulator and return it, or use an iterator pipeline/flatMap.",
                )],
            },
            Case {
                name: "invalid 27: items.reduce((acc, item) => Object.assign({}, acc as State",
                path: "file.ts",
                code: "items.reduce((acc, item) => Object.assign({}, acc as State, item), {});",
                expected: &[(
                    28,
                    65,
                    "Do not copy the reducer accumulator on every iteration; growing copies can cause quadratic work. Mutate a fresh, locally owned accumulator and return it, or use an iterator pipeline/flatMap.",
                )],
            },
            Case {
                name: "invalid 28: items.reduce((acc, item) => { const next = Object.assign({",
                path: "file.ts",
                code: "items.reduce((acc, item) => { const next = Object.assign({}, acc); next[item.id] = item; return next; }, {});",
                expected: &[(
                    43,
                    65,
                    "Do not copy the reducer accumulator on every iteration; growing copies can cause quadratic work. Mutate a fresh, locally owned accumulator and return it, or use an iterator pipeline/flatMap.",
                )],
            },
            Case {
                name: "invalid 29: items.reduce((acc, item) => acc.concat([item]), []);",
                path: "file.ts",
                code: "items.reduce((acc, item) => acc.concat([item]), []);",
                expected: &[(
                    28,
                    46,
                    "Do not copy the reducer accumulator on every iteration; growing copies can cause quadratic work. Mutate a fresh, locally owned accumulator and return it, or use an iterator pipeline/flatMap.",
                )],
            },
            Case {
                name: "invalid 30: items.reduceRight((acc, item, index) => acc['concat']([ite",
                path: "file.ts",
                code: "items.reduceRight((acc, item, index) => acc['concat']([item]), [] as Item[]);",
                expected: &[(
                    40,
                    61,
                    "Do not copy the reducer accumulator on every iteration; growing copies can cause quadratic work. Mutate a fresh, locally owned accumulator and return it, or use an iterator pipeline/flatMap.",
                )],
            },
            Case {
                name: "invalid 31: items.reduce((acc, item) => { const next = acc.slice(); ne",
                path: "file.ts",
                code: "items.reduce((acc, item) => { const next = acc.slice(); next.push(item); return next; }, []);",
                expected: &[(
                    43,
                    54,
                    "Do not copy the reducer accumulator on every iteration; growing copies can cause quadratic work. Mutate a fresh, locally owned accumulator and return it, or use an iterator pipeline/flatMap.",
                )],
            },
            Case {
                name: "invalid 32: items.reduce((acc, item) => { const alias = acc; return al",
                path: "file.ts",
                code: "items.reduce((acc, item) => { const alias = acc; return alias.concat(item); }, []);",
                expected: &[(
                    56,
                    74,
                    "Do not copy the reducer accumulator on every iteration; growing copies can cause quadratic work. Mutate a fresh, locally owned accumulator and return it, or use an iterator pipeline/flatMap.",
                )],
            },
            Case {
                name: "invalid 33: const initial = []; items.reduce((acc, item) => acc.concat",
                path: "file.ts",
                code: "const initial = []; items.reduce((acc, item) => acc.concat(item), initial);",
                expected: &[(
                    48,
                    64,
                    "Do not copy the reducer accumulator on every iteration; growing copies can cause quadratic work. Mutate a fresh, locally owned accumulator and return it, or use an iterator pipeline/flatMap.",
                )],
            },
            Case {
                name: "invalid 34: items.reduce((acc, item) => { const next = Array.from(acc)",
                path: "file.ts",
                code: "items.reduce((acc, item) => { const next = Array.from(acc); next.push(item); return next; }, []);",
                expected: &[(
                    43,
                    58,
                    "Do not copy the reducer accumulator on every iteration; growing copies can cause quadratic work. Mutate a fresh, locally owned accumulator and return it, or use an iterator pipeline/flatMap.",
                )],
            },
            Case {
                name: "invalid 35: items.reduce((acc, item) => acc.toSpliced(acc.length, 0, i",
                path: "file.ts",
                code: "items.reduce((acc, item) => acc.toSpliced(acc.length, 0, item), []);",
                expected: &[(
                    28,
                    62,
                    "Do not copy the reducer accumulator on every iteration; growing copies can cause quadratic work. Mutate a fresh, locally owned accumulator and return it, or use an iterator pipeline/flatMap.",
                )],
            },
            Case {
                name: "invalid 36: items.reduce((acc, item) => acc.toSorted(), []);",
                path: "file.ts",
                code: "items.reduce((acc, item) => acc.toSorted(), []);",
                expected: &[(
                    28,
                    42,
                    "Do not copy the reducer accumulator on every iteration; growing copies can cause quadratic work. Mutate a fresh, locally owned accumulator and return it, or use an iterator pipeline/flatMap.",
                )],
            },
            Case {
                name: "invalid 37: items.reduce((acc, item) => acc.toReversed(), []);",
                path: "file.ts",
                code: "items.reduce((acc, item) => acc.toReversed(), []);",
                expected: &[(
                    28,
                    44,
                    "Do not copy the reducer accumulator on every iteration; growing copies can cause quadratic work. Mutate a fresh, locally owned accumulator and return it, or use an iterator pipeline/flatMap.",
                )],
            },
            Case {
                name: "invalid 38: items.reduce((acc, item) => acc.with(0, item), []);",
                path: "file.ts",
                code: "items.reduce((acc, item) => acc.with(0, item), []);",
                expected: &[(
                    28,
                    45,
                    "Do not copy the reducer accumulator on every iteration; growing copies can cause quadratic work. Mutate a fresh, locally owned accumulator and return it, or use an iterator pipeline/flatMap.",
                )],
            },
        ],
    );
}

/// Edge cases recorded from the upstream plugin under oxlint 1.86.
#[test]
fn edges() {
    check(
        "anti-slop/no-reduce-accumulator-copy",
        "[]",
        &[
            Case {
                name: "edge 0: items.reduce((acc, item) => Object.assign({}, ...acc), {});",
                path: "file.ts",
                code: "items.reduce((acc, item) => Object.assign({}, ...acc), {});",
                expected: &[],
            },
            Case {
                name: "edge 1: items.reduce((acc, item) => acc.concat(item), [], extra);",
                path: "file.ts",
                code: "items.reduce((acc, item) => acc.concat(item), [], extra);",
                expected: &[],
            },
            Case {
                name: "edge 2: items.reduce(((acc, item) => Object.assign({}, acc)) as Red",
                path: "file.ts",
                code: "items.reduce(((acc, item) => Object.assign({}, acc)) as Reducer, {});",
                expected: &[(
                    29,
                    51,
                    "Do not copy the reducer accumulator on every iteration; growing copies can cause quadratic work. Mutate a fresh, locally owned accumulator and return it, or use an iterator pipeline/flatMap.",
                )],
            },
            Case {
                name: "edge 3: items.reduce(function (this: Ctx, acc, item) { return Objec",
                path: "file.ts",
                code: "items.reduce(function (this: Ctx, acc, item) { return Object.assign({}, acc); }, {});",
                expected: &[],
            },
            Case {
                name: "edge 4: items.reduce((acc) => { const f = function () { return Obje",
                path: "file.ts",
                code: "items.reduce((acc) => { const f = function () { return Object.assign({}, acc); }; return acc; }, {});",
                expected: &[],
            },
            Case {
                name: "edge 5: items.reduce((acc, item) => { class C { x = Object.assign({",
                path: "file.ts",
                code: "items.reduce((acc, item) => { class C { x = Object.assign({}, acc); } return acc; }, {});",
                expected: &[(
                    44,
                    66,
                    "Do not copy the reducer accumulator on every iteration; growing copies can cause quadratic work. Mutate a fresh, locally owned accumulator and return it, or use an iterator pipeline/flatMap.",
                )],
            },
            Case {
                name: "edge 6: items?.reduce((acc, item) => acc?.concat(item), []);",
                path: "file.ts",
                code: "items?.reduce((acc, item) => acc?.concat(item), []);",
                expected: &[(
                    29,
                    46,
                    "Do not copy the reducer accumulator on every iteration; growing copies can cause quadratic work. Mutate a fresh, locally owned accumulator and return it, or use an iterator pipeline/flatMap.",
                )],
            },
            Case {
                name: "edge 7: items.reduce(([acc], item) => Object.assign({}, acc), {});",
                path: "file.ts",
                code: "items.reduce(([acc], item) => Object.assign({}, acc), {});",
                expected: &[],
            },
            Case {
                name: "edge 8: items.reduce((acc, item) => (acc as Item[]).concat(item), [",
                path: "file.ts",
                code: "items.reduce((acc, item) => (acc as Item[]).concat(item), []);",
                expected: &[(
                    28,
                    56,
                    "Do not copy the reducer accumulator on every iteration; growing copies can cause quadratic work. Mutate a fresh, locally owned accumulator and return it, or use an iterator pipeline/flatMap.",
                )],
            },
            Case {
                name: "edge 9: items.reduce((acc, item) => acc.concat(item), Array.of());",
                path: "file.ts",
                code: "items.reduce((acc, item) => acc.concat(item), Array.of());",
                expected: &[],
            },
            Case {
                name: "edge 10: items.reduce((acc, item) => acc.concat(item), [] satisfies",
                path: "file.ts",
                code: "items.reduce((acc, item) => acc.concat(item), [] satisfies Item[]);",
                expected: &[(
                    28,
                    44,
                    "Do not copy the reducer accumulator on every iteration; growing copies can cause quadratic work. Mutate a fresh, locally owned accumulator and return it, or use an iterator pipeline/flatMap.",
                )],
            },
            Case {
                name: "edge 11: items.reduce((acc, item) => Array.from(acc, f), []);",
                path: "file.ts",
                code: "items.reduce((acc, item) => Array.from(acc, f), []);",
                expected: &[(
                    28,
                    46,
                    "Do not copy the reducer accumulator on every iteration; growing copies can cause quadratic work. Mutate a fresh, locally owned accumulator and return it, or use an iterator pipeline/flatMap.",
                )],
            },
            Case {
                name: "edge 12: items.reduce((acc, item) => Object.assign({ a: 1 }, acc), ",
                path: "file.ts",
                code: "items.reduce((acc, item) => Object.assign({ a: 1 }, acc), {});",
                expected: &[(
                    28,
                    56,
                    "Do not copy the reducer accumulator on every iteration; growing copies can cause quadratic work. Mutate a fresh, locally owned accumulator and return it, or use an iterator pipeline/flatMap.",
                )],
            },
            Case {
                name: "edge 13: items.reduce((acc, item) => Object?.assign({}, acc), {});",
                path: "file.ts",
                code: "items.reduce((acc, item) => Object?.assign({}, acc), {});",
                expected: &[(
                    28,
                    51,
                    "Do not copy the reducer accumulator on every iteration; growing copies can cause quadratic work. Mutate a fresh, locally owned accumulator and return it, or use an iterator pipeline/flatMap.",
                )],
            },
            Case {
                name: "edge 14: type Array = Custom; items.reduce((acc, item) => Array.fro",
                path: "file.ts",
                code: "type Array = Custom; items.reduce((acc, item) => Array.from(acc), []);",
                expected: &[],
            },
            Case {
                name: "edge 15: let initial = []; items.reduce((acc, item) => acc.with(0, ",
                path: "file.ts",
                code: "let initial = []; items.reduce((acc, item) => acc.with(0, item), initial);",
                expected: &[],
            },
            Case {
                name: "edge 16: items.reduce((acc, item) => { const a = acc; const b = a; ",
                path: "file.ts",
                code: "items.reduce((acc, item) => { const a = acc; const b = a; return b.concat(item); }, []);",
                expected: &[(
                    65,
                    79,
                    "Do not copy the reducer accumulator on every iteration; growing copies can cause quadratic work. Mutate a fresh, locally owned accumulator and return it, or use an iterator pipeline/flatMap.",
                )],
            },
            Case {
                name: "edge 17: items.reduce(function named(acc) { return Object.assign({}",
                path: "file.ts",
                code: "items.reduce(function named(acc) { return Object.assign({}, acc); }, {});",
                expected: &[(
                    42,
                    64,
                    "Do not copy the reducer accumulator on every iteration; growing copies can cause quadratic work. Mutate a fresh, locally owned accumulator and return it, or use an iterator pipeline/flatMap.",
                )],
            },
            Case {
                name: "edge 18: items.reduce((acc, item) => acc.concat(item));",
                path: "file.ts",
                code: "items.reduce((acc, item) => acc.concat(item));",
                expected: &[],
            },
            Case {
                name: "edge 19: items.reduce((acc, item) => acc.concat(item), ...rest);",
                path: "file.ts",
                code: "items.reduce((acc, item) => acc.concat(item), ...rest);",
                expected: &[],
            },
            Case {
                name: "edge 20: items.reduce((...acc) => Object.assign({}, acc), {});",
                path: "file.ts",
                code: "items.reduce((...acc) => Object.assign({}, acc), {});",
                expected: &[],
            },
            Case {
                name: "edge 21: items.reduce(reducer, {}); function reducer(acc) { return ",
                path: "file.ts",
                code: "items.reduce(reducer, {}); function reducer(acc) { return Object.assign({}, acc); }",
                expected: &[],
            },
            Case {
                name: "edge 22: items.reduce((acc, item) => { const obj = { m() { return O",
                path: "file.ts",
                code: "items.reduce((acc, item) => { const obj = { m() { return Object.assign({}, acc); } }; return acc; }, {});",
                expected: &[],
            },
            Case {
                name: "edge 23: items.reduce((acc, item) => acc.slice(), initialList);",
                path: "file.ts",
                code: "items.reduce((acc, item) => acc.slice(), initialList);",
                expected: &[],
            },
            Case {
                name: "edge 24: const initialList: Item[] = []; items.reduce((acc, item) =",
                path: "file.ts",
                code: "const initialList: Item[] = []; items.reduce((acc, item) => acc.slice(), initialList);",
                expected: &[(
                    60,
                    71,
                    "Do not copy the reducer accumulator on every iteration; growing copies can cause quadratic work. Mutate a fresh, locally owned accumulator and return it, or use an iterator pipeline/flatMap.",
                )],
            },
            Case {
                name: "edge 25: items.reduce((acc, item) => acc.map(f), []);",
                path: "file.ts",
                code: "items.reduce((acc, item) => acc.map(f), []);",
                expected: &[],
            },
            Case {
                name: "edge 26: items.reduce((acc, item) => acc.concat(item), [])['concat'",
                path: "file.ts",
                code: "items.reduce((acc, item) => acc.concat(item), [])['concat'](other);",
                expected: &[(
                    28,
                    44,
                    "Do not copy the reducer accumulator on every iteration; growing copies can cause quadratic work. Mutate a fresh, locally owned accumulator and return it, or use an iterator pipeline/flatMap.",
                )],
            },
            Case {
                name: "edge 27: items.reduce((acc, item) => Object.assign({}, acc!), {});",
                path: "file.ts",
                code: "items.reduce((acc, item) => Object.assign({}, acc!), {});",
                expected: &[(
                    28,
                    51,
                    "Do not copy the reducer accumulator on every iteration; growing copies can cause quadratic work. Mutate a fresh, locally owned accumulator and return it, or use an iterator pipeline/flatMap.",
                )],
            },
            Case {
                name: "edge 28: items.reduce((acc, item) => { let alias = acc; return Obje",
                path: "file.ts",
                code: "items.reduce((acc, item) => { let alias = acc; return Object.assign({}, alias); }, {});",
                expected: &[],
            },
            Case {
                name: "edge 29: outer.reduce((total, row) => row.reduce((acc, x) => Object",
                path: "file.ts",
                code: "outer.reduce((total, row) => row.reduce((acc, x) => Object.assign({}, total), {}), {});",
                expected: &[],
            },
            Case {
                name: "edge 30: items.reduce((acc: State = {}, item) => Object.assign({}, ",
                path: "file.ts",
                code: "items.reduce((acc: State = {}, item) => Object.assign({}, acc), {});",
                expected: &[(
                    40,
                    62,
                    "Do not copy the reducer accumulator on every iteration; growing copies can cause quadratic work. Mutate a fresh, locally owned accumulator and return it, or use an iterator pipeline/flatMap.",
                )],
            },
        ],
    );
}
