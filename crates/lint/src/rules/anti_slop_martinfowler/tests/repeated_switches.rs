//! `anti-slop-martinfowler/repeated-switches`: expectations recorded from the upstream plugin
//! (github.com/k-mistele/anti-slop) under oxlint 1.86, one case per upstream
//! RuleTester case in `src/martinfowler/rules/repeated-switches.test.ts`.

use super::super::super::anti_slop::tests::{Case, check};

/// src/martinfowler/rules/repeated-switches.test.ts
#[test]
fn upstream() {
    check(
        "anti-slop-martinfowler/repeated-switches",
        "[]",
        &[
            Case {
                name: "valid 0: function label(e) { switch (e._tag) { case 'A': return 1; cas",
                path: "file.ts",
                code: "function label(e) { switch (e._tag) { case 'A': return 1; case 'B': return 3; default: return 2; } }",
                expected: &[],
            },
            Case {
                name: "valid 1: switch (a.kind) { case 'x': case 'y': } switch (b.kind) { cas",
                path: "file.ts",
                code: "switch (a.kind) { case 'x': case 'y': } switch (b.kind) { case 'x': case 'y': }",
                expected: &[],
            },
            Case {
                name: "valid 2: switch (part.type) { case 'text': case 'image': } switch (par",
                path: "file.ts",
                code: "switch (part.type) { case 'text': case 'image': } switch (part.type) { case 'text': case 'tool_use': }",
                expected: &[],
            },
            Case {
                name: "valid 3: if (event.type === 'click') a(); function f() { if (event.typ",
                path: "file.ts",
                code: "if (event.type === 'click') a(); function f() { if (event.type === 'key') b(); }",
                expected: &[],
            },
            Case {
                name: "valid 4: switch (true) { case a: case b: break; } switch (true) { case",
                path: "file.ts",
                code: "switch (true) { case a: case b: break; } switch (true) { case a: case b: break; }",
                expected: &[],
            },
            Case {
                name: "valid 5: if (k === 'a') a(); log(); if (k === 'b') b(); if (k === 'a')",
                path: "file.ts",
                code: "if (k === 'a') a(); log(); if (k === 'b') b(); if (k === 'a') a(); log(); if (k === 'b') b();",
                expected: &[],
            },
            Case {
                name: "valid 6: if (a.id === b.id) x(); else if (a.id === c.id) y(); if (a.id",
                path: "file.ts",
                code: "if (a.id === b.id) x(); else if (a.id === c.id) y(); if (a.id === b.id) z(); else if (a.id === c.id) w();",
                expected: &[],
            },
            Case {
                name: "valid 8: switch (e._tag) { case 'A': case 'B': } switch (e._tag) { cas",
                path: "src/e.test.ts",
                code: "switch (e._tag) { case 'A': case 'B': } switch (e._tag) { case 'A': case 'B': }",
                expected: &[],
            },
            Case {
                name: "invalid 9: \t\t\t\tfunction area(shape) { switch (shape.kind) { case 'circ",
                path: "file.ts",
                code: "\n\t\t\t\tfunction area(shape) { switch (shape.kind) { case 'circle': return 1; case 'square': return 2; } }\n\t\t\t\tfunction name(shape) { switch (shape.kind) { case 'circle': return 'c'; case 'square': return 's'; } }\n\t\t\t",
                expected: &[
                    (
                        28,
                        101,
                        "Possibly Repeated Switches: this file branches on `shape.kind` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                    ),
                    (
                        131,
                        208,
                        "Possibly Repeated Switches: this file branches on `shape.kind` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                    ),
                ],
            },
            Case {
                name: "invalid 10: \t\t\t\tfunction a(e) { if (e._tag === 'Ready') return 1; else",
                path: "file.ts",
                code: "\n\t\t\t\tfunction a(e) { if (e._tag === 'Ready') return 1; else if (e._tag === 'Done') return 2; }\n\t\t\t\tfunction b(e) { return e._tag === 'Ready' ? 'r' : e._tag === 'Done' ? 'd' : 'x'; }\n\t\t\t",
                expected: &[
                    (
                        21,
                        92,
                        "Possibly Repeated Switches: this file branches on `e._tag` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                    ),
                    (
                        122,
                        178,
                        "Possibly Repeated Switches: this file branches on `e._tag` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                    ),
                ],
            },
            Case {
                name: "invalid 11: \t\t\t\tfunction a(kind) {",
                path: "file.ts",
                code: "\n\t\t\t\tfunction a(kind) {\n\t\t\t\t\tif (kind === Kind.A) return 1;\n\t\t\t\t\tif (kind === Kind.B || kind === Kind.C) return 2;\n\t\t\t\t\treturn 3;\n\t\t\t\t}\n\t\t\t\tfunction b(kind) { switch (kind) { case Kind.A: return 'a'; case Kind.C: return 'c'; } }\n\t\t\t",
                expected: &[
                    (
                        29,
                        59,
                        "Possibly Repeated Switches: this file branches on `kind` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                    ),
                    (
                        159,
                        226,
                        "Possibly Repeated Switches: this file branches on `kind` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                    ),
                ],
            },
            Case {
                name: "invalid 12: switch (typeof value) { case 'string': case 'number': } co",
                path: "file.ts",
                code: "switch (typeof value) { case 'string': case 'number': } const t = typeof value === 'string' ? 1 : typeof value === 'number' ? 2 : 3;",
                expected: &[
                    (
                        0,
                        55,
                        "Possibly Repeated Switches: this file branches on `typeof value` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                    ),
                    (
                        66,
                        131,
                        "Possibly Repeated Switches: this file branches on `typeof value` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                    ),
                ],
            },
        ],
    );
}

/// src/martinfowler/rules/repeated-switches.test.ts, options `[{"minOccurrences":3}]`
#[test]
fn upstream_options_1() {
    check(
        "anti-slop-martinfowler/repeated-switches",
        "[{\"minOccurrences\":3}]",
        &[Case {
            name: "valid 7: switch (e._tag) { case 'A': case 'B': } switch (e._tag) { cas",
            path: "file.ts",
            code: "switch (e._tag) { case 'A': case 'B': } switch (e._tag) { case 'A': case 'B': }",
            expected: &[],
        }],
    );
}

/// src/martinfowler/rules/repeated-switches.test.ts, options `[{"minSharedCases":1}]`
#[test]
fn upstream_options_2() {
    check(
        "anti-slop-martinfowler/repeated-switches",
        "[{\"minSharedCases\":1}]",
        &[Case {
            name: "invalid 13: switch (part.type) { case 'text': case 'image': } switch (",
            path: "file.ts",
            code: "switch (part.type) { case 'text': case 'image': } switch (part.type) { case 'text': case 'tool_use': }",
            expected: &[
                (
                    0,
                    49,
                    "Possibly Repeated Switches: this file branches on `part.type` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                ),
                (
                    50,
                    102,
                    "Possibly Repeated Switches: this file branches on `part.type` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                ),
            ],
        }],
    );
}

/// src/martinfowler/rules/repeated-switches.test.ts, options `[{"ignoreTests":false}]`
#[test]
fn upstream_options_3() {
    check(
        "anti-slop-martinfowler/repeated-switches",
        "[{\"ignoreTests\":false}]",
        &[Case {
            name: "invalid 14: switch (e._tag) { case 'A': case 'B': } switch (e._tag) { ",
            path: "src/e.test.ts",
            code: "switch (e._tag) { case 'A': case 'B': } switch (e._tag) { case 'A': case 'B': }",
            expected: &[
                (
                    0,
                    39,
                    "Possibly Repeated Switches: this file branches on `e._tag` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                ),
                (
                    40,
                    79,
                    "Possibly Repeated Switches: this file branches on `e._tag` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                ),
            ],
        }],
    );
}

/// Edge cases recorded from upstream under oxlint 1.86: src/martinfowler/rules/repeated-switches.mfedges.test.ts
#[test]
fn edges() {
    check(
        "anti-slop-martinfowler/repeated-switches",
        "[]",
        &[
            Case {
                name: "invalid 0: switch (a .  b) { case 'x': case 'y': } switch (a . b) { ca",
                path: "file.ts",
                code: "switch (a .  b) { case 'x': case 'y': } switch (a . b) { case 'x': case 'y': }",
                expected: &[
                    (
                        0,
                        39,
                        "Possibly Repeated Switches: this file branches on `a . b` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                    ),
                    (
                        40,
                        78,
                        "Possibly Repeated Switches: this file branches on `a . b` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                    ),
                ],
            },
            Case {
                name: "invalid 1: switch ((x)) { case 'a': case 'b': } if (x === 'a') f(); el",
                path: "file.ts",
                code: "switch ((x)) { case 'a': case 'b': } if (x === 'a') f(); else if (x === 'b') g();",
                expected: &[
                    (
                        0,
                        36,
                        "Possibly Repeated Switches: this file branches on `x` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                    ),
                    (
                        37,
                        81,
                        "Possibly Repeated Switches: this file branches on `x` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                    ),
                ],
            },
            Case {
                name: "invalid 2: switch (x) { case ('a'): case 'b': } switch (x) { case 'a':",
                path: "file.ts",
                code: "switch (x) { case ('a'): case 'b': } switch (x) { case 'a': case ('b'): }",
                expected: &[
                    (
                        0,
                        36,
                        "Possibly Repeated Switches: this file branches on `x` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                    ),
                    (
                        37,
                        73,
                        "Possibly Repeated Switches: this file branches on `x` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                    ),
                ],
            },
            Case {
                name: "invalid 3: switch (x as T) { case 'a': case 'b': } switch (x!) { case ",
                path: "file.ts",
                code: "switch (x as T) { case 'a': case 'b': } switch (x!) { case 'a': case 'b': }",
                expected: &[
                    (
                        0,
                        39,
                        "Possibly Repeated Switches: this file branches on `x` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                    ),
                    (
                        40,
                        75,
                        "Possibly Repeated Switches: this file branches on `x` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                    ),
                ],
            },
            Case {
                name: "invalid 4: if ((x as T) === 'a') f(); else if ((x as T) === 'b') g(); ",
                path: "file.ts",
                code: "if ((x as T) === 'a') f(); else if ((x as T) === 'b') g(); switch (x as T) { case 'a': case 'b': }",
                expected: &[],
            },
            Case {
                name: "invalid 5: const r = x === 'a' ? 1 : (x === 'b' ? 2 : 3); const s = x ",
                path: "file.ts",
                code: "const r = x === 'a' ? 1 : (x === 'b' ? 2 : 3); const s = x === 'a' ? 1 : x === 'b' ? 2 : 3;",
                expected: &[
                    (
                        10,
                        45,
                        "Possibly Repeated Switches: this file branches on `x` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                    ),
                    (
                        57,
                        90,
                        "Possibly Repeated Switches: this file branches on `x` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                    ),
                ],
            },
            Case {
                name: "invalid 6: class A { static { if (x === 'a') f(); if (x === 'b') g(); ",
                path: "file.ts",
                code: "class A { static { if (x === 'a') f(); if (x === 'b') g(); } } switch (x) { case 'a': case 'b': }",
                expected: &[],
            },
            Case {
                name: "invalid 7: namespace N { if (x === 'a') f(); if (x === 'b') g(); } swi",
                path: "file.ts",
                code: "namespace N { if (x === 'a') f(); if (x === 'b') g(); } switch (x) { case 'a': case 'b': }",
                expected: &[],
            },
            Case {
                name: "invalid 8: switch (someVeryLongObjectName.withAVeryLongPropertyName.an",
                path: "file.ts",
                code: "switch (someVeryLongObjectName.withAVeryLongPropertyName.andAnotherOne.kind) { case 'a': case 'b': } switch (someVeryLongObjectName.withAVeryLongPropertyName.andAnotherOne.kind) { case 'a': case 'b': }",
                expected: &[
                    (
                        0,
                        100,
                        "Possibly Repeated Switches: this file branches on `someVeryLongObjectName.withAVeryLongPropertyName.andAnother…` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                    ),
                    (
                        101,
                        201,
                        "Possibly Repeated Switches: this file branches on `someVeryLongObjectName.withAVeryLongPropertyName.andAnother…` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                    ),
                ],
            },
            Case {
                name: "invalid 9: if ('a' === x) f(); else if ('b' === x) g(); switch (x) { c",
                path: "file.ts",
                code: "if ('a' === x) f(); else if ('b' === x) g(); switch (x) { case 'a': case 'b': }",
                expected: &[
                    (
                        0,
                        44,
                        "Possibly Repeated Switches: this file branches on `x` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                    ),
                    (
                        45,
                        79,
                        "Possibly Repeated Switches: this file branches on `x` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                    ),
                ],
            },
            Case {
                name: "invalid 10: if (x === y) f(); else if (x === z) g(); switch (x) { case",
                path: "file.ts",
                code: "if (x === y) f(); else if (x === z) g(); switch (x) { case y: case z: }",
                expected: &[],
            },
            Case {
                name: "invalid 11: if (x === Kind.A || x === Kind.B) f(); if (x === Kind.C) g",
                path: "file.ts",
                code: "if (x === Kind.A || x === Kind.B) f(); if (x === Kind.C) g(); switch (x) { case Kind.A: case Kind.C: }",
                expected: &[
                    (
                        0,
                        38,
                        "Possibly Repeated Switches: this file branches on `x` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                    ),
                    (
                        62,
                        102,
                        "Possibly Repeated Switches: this file branches on `x` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                    ),
                ],
            },
            Case {
                name: "invalid 12: if (x == null) f(); else if (x == 0) g(); switch (x) { cas",
                path: "file.ts",
                code: "if (x == null) f(); else if (x == 0) g(); switch (x) { case null: case 0: }",
                expected: &[
                    (
                        0,
                        41,
                        "Possibly Repeated Switches: this file branches on `x` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                    ),
                    (
                        42,
                        75,
                        "Possibly Repeated Switches: this file branches on `x` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                    ),
                ],
            },
            Case {
                name: "invalid 13: switch (x) { case `a`: case 'b': } switch (x) { case `a`: ",
                path: "file.ts",
                code: "switch (x) { case `a`: case 'b': } switch (x) { case `a`: case 'b': }",
                expected: &[
                    (
                        0,
                        34,
                        "Possibly Repeated Switches: this file branches on `x` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                    ),
                    (
                        35,
                        69,
                        "Possibly Repeated Switches: this file branches on `x` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                    ),
                ],
            },
            Case {
                name: "invalid 14: switch (x) { case `a${1}`: case 'b': } switch (x) { case `",
                path: "file.ts",
                code: "switch (x) { case `a${1}`: case 'b': } switch (x) { case `a${1}`: case 'b': }",
                expected: &[
                    (
                        0,
                        38,
                        "Possibly Repeated Switches: this file branches on `x` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                    ),
                    (
                        39,
                        77,
                        "Possibly Repeated Switches: this file branches on `x` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                    ),
                ],
            },
            Case {
                name: "invalid 15: const f = () => { if (x !== 'a') return; if (x !== 'b') re",
                path: "file.ts",
                code: "const f = () => { if (x !== 'a') return; if (x !== 'b') return; }; switch (x) { case 'a': case 'b': }",
                expected: &[
                    (
                        18,
                        40,
                        "Possibly Repeated Switches: this file branches on `x` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                    ),
                    (
                        67,
                        101,
                        "Possibly Repeated Switches: this file branches on `x` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                    ),
                ],
            },
            Case {
                name: "invalid 16: if (x === 'a') f(); if (x === 'b') g(); else h(); switch (",
                path: "file.ts",
                code: "if (x === 'a') f(); if (x === 'b') g(); else h(); switch (x) { case 'a': case 'b': }",
                expected: &[],
            },
            Case {
                name: "invalid 17: if (x === 'a') f(); if (y === 'b') g(); if (x === 'b') h()",
                path: "file.ts",
                code: "if (x === 'a') f(); if (y === 'b') g(); if (x === 'b') h(); switch (x) { case 'a': case 'b': }",
                expected: &[],
            },
            Case {
                name: "invalid 18: if (x === 'a') f(); if (x === 'b') g(); if (x === 'c') h()",
                path: "file.ts",
                code: "if (x === 'a') f(); if (x === 'b') g(); if (x === 'c') h(); switch (x) { case 'a': case 'c': }",
                expected: &[
                    (
                        0,
                        19,
                        "Possibly Repeated Switches: this file branches on `x` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                    ),
                    (
                        60,
                        94,
                        "Possibly Repeated Switches: this file branches on `x` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                    ),
                ],
            },
            Case {
                name: "invalid 19: switch (x) { case 'a': if (y === 1) f(); if (y === 2) g();",
                path: "file.ts",
                code: "switch (x) { case 'a': if (y === 1) f(); if (y === 2) g(); } switch (y) { case 1: case 2: }",
                expected: &[
                    (
                        23,
                        40,
                        "Possibly Repeated Switches: this file branches on `y` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                    ),
                    (
                        61,
                        91,
                        "Possibly Repeated Switches: this file branches on `y` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                    ),
                ],
            },
            Case {
                name: "invalid 20: if (x === 'a') f(); else if (x === 'b') g(); else if (y ==",
                path: "file.ts",
                code: "if (x === 'a') f(); else if (x === 'b') g(); else if (y === 'c') h(); switch (x) { case 'a': case 'b': }",
                expected: &[
                    (
                        0,
                        69,
                        "Possibly Repeated Switches: this file branches on `x` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                    ),
                    (
                        70,
                        104,
                        "Possibly Repeated Switches: this file branches on `x` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                    ),
                ],
            },
            Case {
                name: "invalid 21: if (x === 'a' || y === 'b') f(); if (x === 'b') g(); switc",
                path: "file.ts",
                code: "if (x === 'a' || y === 'b') f(); if (x === 'b') g(); switch (x) { case 'a': case 'b': }",
                expected: &[],
            },
            Case {
                name: "invalid 22: const a = x === 'a' ? 1 : y === 'b' ? 2 : 3; switch (x) { ",
                path: "file.ts",
                code: "const a = x === 'a' ? 1 : y === 'b' ? 2 : 3; switch (x) { case 'a': case 'b': }",
                expected: &[],
            },
            Case {
                name: "invalid 23: const a = (x === 'a' ? 1 : 2) ? (x === 'b' ? 3 : 4) : 5; s",
                path: "file.ts",
                code: "const a = (x === 'a' ? 1 : 2) ? (x === 'b' ? 3 : 4) : 5; switch (x) { case 'a': case 'b': }",
                expected: &[],
            },
            Case {
                name: "invalid 24: switch (x) { case 'a': case 'a': case 'b': } switch (x) { ",
                path: "file.ts",
                code: "switch (x) { case 'a': case 'a': case 'b': } switch (x) { case 'a': }",
                expected: &[],
            },
            Case {
                name: "invalid 25: switch (1) { case 'a': case 'b': } switch (1) { case 'a': ",
                path: "file.ts",
                code: "switch (1) { case 'a': case 'b': } switch (1) { case 'a': case 'b': }",
                expected: &[],
            },
            Case {
                name: "invalid 26: switch (-1) { case 'a': case 'b': } switch (-1) { case 'a'",
                path: "file.ts",
                code: "switch (-1) { case 'a': case 'b': } switch (-1) { case 'a': case 'b': }",
                expected: &[
                    (
                        0,
                        35,
                        "Possibly Repeated Switches: this file branches on `-1` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                    ),
                    (
                        36,
                        71,
                        "Possibly Repeated Switches: this file branches on `-1` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                    ),
                ],
            },
            Case {
                name: "invalid 27: switch (x) { case E.a: case E.b: } switch (x) { case E.a: ",
                path: "file.ts",
                code: "switch (x) { case E.a: case E.b: } switch (x) { case E.a: case E.b: }",
                expected: &[
                    (
                        0,
                        34,
                        "Possibly Repeated Switches: this file branches on `x` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                    ),
                    (
                        35,
                        69,
                        "Possibly Repeated Switches: this file branches on `x` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                    ),
                ],
            },
            Case {
                name: "invalid 28: switch (x) { case e.A: case e.B: } switch (x) { case e.A: ",
                path: "file.ts",
                code: "switch (x) { case e.A: case e.B: } switch (x) { case e.A: case e.B: }",
                expected: &[
                    (
                        0,
                        34,
                        "Possibly Repeated Switches: this file branches on `x` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                    ),
                    (
                        35,
                        69,
                        "Possibly Repeated Switches: this file branches on `x` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                    ),
                ],
            },
            Case {
                name: "invalid 29: if (x.y === 'a') f(); else if (x.y === 'b') g(); if (x?.y ",
                path: "file.ts",
                code: "if (x.y === 'a') f(); else if (x.y === 'b') g(); if (x?.y === 'a') f(); else if (x?.y === 'b') g();",
                expected: &[],
            },
            Case {
                name: "invalid 30: function f() { 'use strict'; if (x === 'a') f(); if (x ===",
                path: "file.ts",
                code: "function f() { 'use strict'; if (x === 'a') f(); if (x === 'b') g(); } switch (x) { case 'a': case 'b': }",
                expected: &[
                    (
                        29,
                        48,
                        "Possibly Repeated Switches: this file branches on `x` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                    ),
                    (
                        71,
                        105,
                        "Possibly Repeated Switches: this file branches on `x` over the same cases in 2 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                    ),
                ],
            },
            Case {
                name: "invalid 32: switch (x) { case 'a': case 'b': } switch (x) { case 'a': ",
                path: "tests/a.ts",
                code: "switch (x) { case 'a': case 'b': } switch (x) { case 'a': case 'b': }",
                expected: &[],
            },
        ],
    );
}

/// Edge cases recorded from upstream under oxlint 1.86: src/martinfowler/rules/repeated-switches.mfedges.test.ts, options `[{"minOccurrences":3,"minSharedCases":1}]`
#[test]
fn edges_options_1() {
    check(
        "anti-slop-martinfowler/repeated-switches",
        "[{\"minOccurrences\":3,\"minSharedCases\":1}]",
        &[Case {
            name: "invalid 31: switch (x) { case 'a': } switch (x) { case 'a': } switch (",
            path: "file.ts",
            code: "switch (x) { case 'a': } switch (x) { case 'a': } switch (x) { case 'a': }",
            expected: &[
                (
                    0,
                    24,
                    "Possibly Repeated Switches: this file branches on `x` over the same cases in 3 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                ),
                (
                    25,
                    49,
                    "Possibly Repeated Switches: this file branches on `x` over the same cases in 3 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                ),
                (
                    50,
                    74,
                    "Possibly Repeated Switches: this file branches on `x` over the same cases in 3 places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant.",
                ),
            ],
        }],
    );
}
