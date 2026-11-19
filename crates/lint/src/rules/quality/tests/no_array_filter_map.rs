//! `quality/no-array-filter-map`: expectations recorded from `@nkzw/oxlint-config` 3.0.0's
//! `quality` plugin under oxlint 1.94.0: byte spans and exact messages.
//!
//! The first tables replay `anti-slop/no-array-filter-map`'s cases
//! (`anti_slop/tests/no_array_filter_map.rs`, same names) with the findings the
//! upstream plugin produced for them.

use super::{Case, check};

/// src/rules/no-array-filter-map.test.ts
#[test]
fn upstream() {
    check(
        "quality/no-array-filter-map",
        "[]",
        &[
            Case {
                name: "valid 0: const users = []; users.values().filter(active).map(email).to",
                path: "file.ts",
                code: "const users = []; users.values().filter(active).map(email).toArray();",
                expected: &[],
            },
            Case {
                name: "valid 1: const users = []; users.values().map(email).filter(Boolean).t",
                path: "file.ts",
                code: "const users = []; users.values().map(email).filter(Boolean).toArray();",
                expected: &[],
            },
            Case {
                name: "valid 2: Iterator.from(users).filter(active).map(email).toArray();",
                path: "file.ts",
                code: "Iterator.from(users).filter(active).map(email).toArray();",
                expected: &[],
            },
            Case {
                name: "valid 3: function collect(users: IteratorObject<User>) { return users.",
                path: "file.ts",
                code: "function collect(users: IteratorObject<User>) { return users.filter(active).map(email).toArray(); }",
                expected: &[],
            },
            Case {
                name: "valid 4: const users = []; users.flatMap(user => user.active ? [user.e",
                path: "file.ts",
                code: "const users = []; users.flatMap(user => user.active ? [user.email] : []);",
                expected: &[],
            },
            Case {
                name: "valid 5: const users = []; users.map(email); users.filter(active);",
                path: "file.ts",
                code: "const users = []; users.map(email); users.filter(active);",
                expected: &[],
            },
            Case {
                name: "valid 6: const users = []; users.map(email).map(normalize);",
                path: "file.ts",
                code: "const users = []; users.map(email).map(normalize);",
                expected: &[],
            },
            Case {
                name: "valid 7: const users = []; users.filter(active).filter(verified);",
                path: "file.ts",
                code: "const users = []; users.filter(active).filter(verified);",
                expected: &[],
            },
            Case {
                name: "valid 8: const custom = { filter() { return this; }, map() {} }; custo",
                path: "file.ts",
                code: "const custom = { filter() { return this; }, map() {} }; custom.filter(active).map(email);",
                expected: &[],
            },
            Case {
                name: "valid 9: function collect(unknownReceiver) { return unknownReceiver.fi",
                path: "file.ts",
                code: "function collect(unknownReceiver) { return unknownReceiver.filter(active).map(email); }",
                expected: &[],
            },
            Case {
                name: "valid 10: const users = fetchUsers(); users.filter(active).map(email);",
                path: "file.ts",
                code: "const users = fetchUsers(); users.filter(active).map(email);",
                expected: &[],
            },
            Case {
                name: "valid 11: const users = []; function collect(users) { return users.fil",
                path: "file.ts",
                code: "const users = []; function collect(users) { return users.filter(active).map(email); }",
                expected: &[],
            },
            Case {
                name: "valid 12: let users = []; users = iterator; users.filter(active).map(e",
                path: "file.ts",
                code: "let users = []; users = iterator; users.filter(active).map(email);",
                expected: &[],
            },
            Case {
                name: "valid 13: const users = []; users[method](active).map(email);",
                path: "file.ts",
                code: "const users = []; users[method](active).map(email);",
                expected: &[],
            },
            Case {
                name: "valid 14: const first = second; const second = first; first.filter(act",
                path: "file.ts",
                code: "const first = second; const second = first; first.filter(active).map(email);",
                expected: &[],
            },
            Case {
                name: "invalid 15: [].filter(active).map(email);",
                path: "file.ts",
                code: "[].filter(active).map(email);",
                expected: &[(
                    0,
                    28,
                    "Avoid consecutive array `filter` and `map` passes. Prefer `.values().filter(...).map(...).toArray()` where iterator helpers are supported, or a single `flatMap`/mutating reducer. Preserve callback ordering, indexes, and filtering semantics.",
                )],
            },
            Case {
                name: "invalid 16: [].map(user => user.active ? user.email : undefined).filte",
                path: "file.ts",
                code: "[].map(user => user.active ? user.email : undefined).filter(email => email !== undefined);",
                expected: &[(
                    0,
                    89,
                    "Avoid consecutive array `map` and `filter` passes. Prefer `.values().map(...).filter(...).toArray()` where iterator helpers are supported, or a single `flatMap`/mutating reducer. Preserve callback ordering, indexes, and filtering semantics.",
                )],
            },
            Case {
                name: "invalid 17: const users = []; users.map(email).filter(Boolean);",
                path: "file.ts",
                code: "const users = []; users.map(email).filter(Boolean);",
                expected: &[(
                    18,
                    50,
                    "Avoid consecutive array `map` and `filter` passes. Prefer `.values().map(...).filter(...).toArray()` where iterator helpers are supported, or a single `flatMap`/mutating reducer. Preserve callback ordering, indexes, and filtering semantics.",
                )],
            },
            Case {
                name: "invalid 18: const users = []; const alias = users; alias.filter(active",
                path: "file.ts",
                code: "const users = []; const alias = users; alias.filter(active).map(email);",
                expected: &[(
                    39,
                    70,
                    "Avoid consecutive array `filter` and `map` passes. Prefer `.values().filter(...).map(...).toArray()` where iterator helpers are supported, or a single `flatMap`/mutating reducer. Preserve callback ordering, indexes, and filtering semantics.",
                )],
            },
            Case {
                name: "invalid 19: function collect(users: User[]) { return users.filter(acti",
                path: "file.ts",
                code: "function collect(users: User[]) { return users.filter(active).map(email); }",
                expected: &[(
                    41,
                    72,
                    "Avoid consecutive array `filter` and `map` passes. Prefer `.values().filter(...).map(...).toArray()` where iterator helpers are supported, or a single `flatMap`/mutating reducer. Preserve callback ordering, indexes, and filtering semantics.",
                )],
            },
            Case {
                name: "invalid 20: function collect(users: readonly User[]) { return users.ma",
                path: "file.ts",
                code: "function collect(users: readonly User[]) { return users.map(email).filter(present); }",
                expected: &[(
                    50,
                    82,
                    "Avoid consecutive array `map` and `filter` passes. Prefer `.values().map(...).filter(...).toArray()` where iterator helpers are supported, or a single `flatMap`/mutating reducer. Preserve callback ordering, indexes, and filtering semantics.",
                )],
            },
            Case {
                name: "invalid 21: function collect(users: ReadonlyArray<User>) { return user",
                path: "file.ts",
                code: "function collect(users: ReadonlyArray<User>) { return users.filter(active).map(email); }",
                expected: &[(
                    54,
                    85,
                    "Avoid consecutive array `filter` and `map` passes. Prefer `.values().filter(...).map(...).toArray()` where iterator helpers are supported, or a single `flatMap`/mutating reducer. Preserve callback ordering, indexes, and filtering semantics.",
                )],
            },
            Case {
                name: "invalid 22: function collect(users: Array<User>) { return users.filter",
                path: "file.ts",
                code: "function collect(users: Array<User>) { return users.filter(active).map(email); }",
                expected: &[(
                    46,
                    77,
                    "Avoid consecutive array `filter` and `map` passes. Prefer `.values().filter(...).map(...).toArray()` where iterator helpers are supported, or a single `flatMap`/mutating reducer. Preserve callback ordering, indexes, and filtering semantics.",
                )],
            },
            Case {
                name: "invalid 23: const users = [] as const; users['filter'](active)['map'](",
                path: "file.ts",
                code: "const users = [] as const; users['filter'](active)['map'](email);",
                expected: &[(
                    27,
                    64,
                    "Avoid consecutive array `filter` and `map` passes. Prefer `.values().filter(...).map(...).toArray()` where iterator helpers are supported, or a single `flatMap`/mutating reducer. Preserve callback ordering, indexes, and filtering semantics.",
                )],
            },
            Case {
                name: "invalid 24: const users = []; (users.filter(active)!).map(email);",
                path: "file.ts",
                code: "const users = []; (users.filter(active)!).map(email);",
                expected: &[(
                    18,
                    52,
                    "Avoid consecutive array `filter` and `map` passes. Prefer `.values().filter(...).map(...).toArray()` where iterator helpers are supported, or a single `flatMap`/mutating reducer. Preserve callback ordering, indexes, and filtering semantics.",
                )],
            },
            Case {
                name: "invalid 25: const users = []; users?.filter(active)?.map(email);",
                path: "file.ts",
                code: "const users = []; users?.filter(active)?.map(email);",
                expected: &[(
                    18,
                    51,
                    "Avoid consecutive array `filter` and `map` passes. Prefer `.values().filter(...).map(...).toArray()` where iterator helpers are supported, or a single `flatMap`/mutating reducer. Preserve callback ordering, indexes, and filtering semantics.",
                )],
            },
            Case {
                name: "invalid 26: const users = []; users.slice().filter(active).map(email);",
                path: "file.ts",
                code: "const users = []; users.slice().filter(active).map(email);",
                expected: &[(
                    18,
                    57,
                    "Avoid consecutive array `filter` and `map` passes. Prefer `.values().filter(...).map(...).toArray()` where iterator helpers are supported, or a single `flatMap`/mutating reducer. Preserve callback ordering, indexes, and filtering semantics.",
                )],
            },
            Case {
                name: "invalid 27: const users = []; users.filter(active).map(email).filter(B",
                path: "file.ts",
                code: "const users = []; users.filter(active).map(email).filter(Boolean);",
                expected: &[
                    (
                        18,
                        49,
                        "Avoid consecutive array `filter` and `map` passes. Prefer `.values().filter(...).map(...).toArray()` where iterator helpers are supported, or a single `flatMap`/mutating reducer. Preserve callback ordering, indexes, and filtering semantics.",
                    ),
                    (
                        18,
                        65,
                        "Avoid consecutive array `map` and `filter` passes. Prefer `.values().map(...).filter(...).toArray()` where iterator helpers are supported, or a single `flatMap`/mutating reducer. Preserve callback ordering, indexes, and filtering semantics.",
                    ),
                ],
            },
        ],
    );
}

/// Edge cases recorded from the upstream plugin under oxlint 1.86.
#[test]
fn edges() {
    check(
        "quality/no-array-filter-map",
        "[]",
        &[
            Case {
                name: "edge 0: const users = []; function f() { const users = getUsers(); ",
                path: "file.ts",
                code: "const users = []; function f() { const users = getUsers(); return users.filter(a).map(b); }",
                expected: &[],
            },
            Case { name: "edge 1: let users = []; users.filter(a).map(b);", path: "file.ts", code: "let users = []; users.filter(a).map(b);", expected: &[] },
            Case {
                name: "edge 2: let users: User[] = []; users.filter(a).map(b);",
                path: "file.ts",
                code: "let users: User[] = []; users.filter(a).map(b);",
                expected: &[(
                    24,
                    46,
                    "Avoid consecutive array `filter` and `map` passes. Prefer `.values().filter(...).map(...).toArray()` where iterator helpers are supported, or a single `flatMap`/mutating reducer. Preserve callback ordering, indexes, and filtering semantics.",
                )],
            },
            Case { name: "edge 3: var users = []; users.filter(a).map(b);", path: "file.ts", code: "var users = []; users.filter(a).map(b);", expected: &[] },
            Case {
                name: "edge 4: function f(...users: User[]) { return users.filter(a).map(b",
                path: "file.ts",
                code: "function f(...users: User[]) { return users.filter(a).map(b); }",
                expected: &[],
            },
            Case {
                name: "edge 5: function f(users: User[] = []) { return users.filter(a).map",
                path: "file.ts",
                code: "function f(users: User[] = []) { return users.filter(a).map(b); }",
                expected: &[(
                    40,
                    62,
                    "Avoid consecutive array `filter` and `map` passes. Prefer `.values().filter(...).map(...).toArray()` where iterator helpers are supported, or a single `flatMap`/mutating reducer. Preserve callback ordering, indexes, and filtering semantics.",
                )],
            },
            Case {
                name: "edge 6: function f(users?: User[]) { return users!.filter(a).map(b)",
                path: "file.ts",
                code: "function f(users?: User[]) { return users!.filter(a).map(b); }",
                expected: &[(
                    36,
                    59,
                    "Avoid consecutive array `filter` and `map` passes. Prefer `.values().filter(...).map(...).toArray()` where iterator helpers are supported, or a single `flatMap`/mutating reducer. Preserve callback ordering, indexes, and filtering semantics.",
                )],
            },
            Case {
                name: "edge 7: const users = []; users.filter(a)?.map(b);",
                path: "file.ts",
                code: "const users = []; users.filter(a)?.map(b);",
                expected: &[(
                    18,
                    41,
                    "Avoid consecutive array `filter` and `map` passes. Prefer `.values().filter(...).map(...).toArray()` where iterator helpers are supported, or a single `flatMap`/mutating reducer. Preserve callback ordering, indexes, and filtering semantics.",
                )],
            },
            Case {
                name: "edge 8: const users = []; (users.filter(a) as User[]).map(b);",
                path: "file.ts",
                code: "const users = []; (users.filter(a) as User[]).map(b);",
                expected: &[(
                    18,
                    52,
                    "Avoid consecutive array `filter` and `map` passes. Prefer `.values().filter(...).map(...).toArray()` where iterator helpers are supported, or a single `flatMap`/mutating reducer. Preserve callback ordering, indexes, and filtering semantics.",
                )],
            },
            Case {
                name: "edge 9: const users = []; users.flatMap(f).filter(a).map(b);",
                path: "file.ts",
                code: "const users = []; users.flatMap(f).filter(a).map(b);",
                expected: &[(
                    18,
                    51,
                    "Avoid consecutive array `filter` and `map` passes. Prefer `.values().filter(...).map(...).toArray()` where iterator helpers are supported, or a single `flatMap`/mutating reducer. Preserve callback ordering, indexes, and filtering semantics.",
                )],
            },
            Case {
                name: "edge 10: const users: User[] | undefined = x; users!.filter(a).map(",
                path: "file.ts",
                code: "const users: User[] | undefined = x; users!.filter(a).map(b);",
                expected: &[],
            },
            Case {
                name: "edge 11: const users = []; users.filter(a)[`map`](b);",
                path: "file.ts",
                code: "const users = []; users.filter(a)[`map`](b);",
                expected: &[],
            },
            Case {
                name: "edge 12: const users = [] satisfies User[]; users.map(a).filter(b);",
                path: "file.ts",
                code: "const users = [] satisfies User[]; users.map(a).filter(b);",
                expected: &[(
                    35,
                    57,
                    "Avoid consecutive array `map` and `filter` passes. Prefer `.values().map(...).filter(...).toArray()` where iterator helpers are supported, or a single `flatMap`/mutating reducer. Preserve callback ordering, indexes, and filtering semantics.",
                )],
            },
            Case {
                name: "edge 13: declare const users: readonly User[]; users.filter(a).map(",
                path: "file.ts",
                code: "declare const users: readonly User[]; users.filter(a).map(b);",
                expected: &[(
                    38,
                    60,
                    "Avoid consecutive array `filter` and `map` passes. Prefer `.values().filter(...).map(...).toArray()` where iterator helpers are supported, or a single `flatMap`/mutating reducer. Preserve callback ordering, indexes, and filtering semantics.",
                )],
            },
            Case {
                name: "edge 14: const users: User[] = getUsers(); users.push(x); users.fil",
                path: "file.ts",
                code: "const users: User[] = getUsers(); users.push(x); users.filter(a).map(b);",
                expected: &[(
                    49,
                    71,
                    "Avoid consecutive array `filter` and `map` passes. Prefer `.values().filter(...).map(...).toArray()` where iterator helpers are supported, or a single `flatMap`/mutating reducer. Preserve callback ordering, indexes, and filtering semantics.",
                )],
            },
            Case {
                name: "edge 15: const users = []; users.filter(a).map(b).filter(c).map(d);",
                path: "file.ts",
                code: "const users = []; users.filter(a).map(b).filter(c).map(d);",
                expected: &[
                    (
                        18,
                        40,
                        "Avoid consecutive array `filter` and `map` passes. Prefer `.values().filter(...).map(...).toArray()` where iterator helpers are supported, or a single `flatMap`/mutating reducer. Preserve callback ordering, indexes, and filtering semantics.",
                    ),
                    (
                        18,
                        50,
                        "Avoid consecutive array `map` and `filter` passes. Prefer `.values().map(...).filter(...).toArray()` where iterator helpers are supported, or a single `flatMap`/mutating reducer. Preserve callback ordering, indexes, and filtering semantics.",
                    ),
                    (
                        18,
                        57,
                        "Avoid consecutive array `filter` and `map` passes. Prefer `.values().filter(...).map(...).toArray()` where iterator helpers are supported, or a single `flatMap`/mutating reducer. Preserve callback ordering, indexes, and filtering semantics.",
                    ),
                ],
            },
            Case {
                name: "edge 16: for (const users of lists) users.filter(a).map(b);",
                path: "file.ts",
                code: "for (const users of lists) users.filter(a).map(b);",
                expected: &[],
            },
            Case {
                name: "edge 17: const [users] = [[]]; users.filter(a).map(b);",
                path: "file.ts",
                code: "const [users] = [[]]; users.filter(a).map(b);",
                expected: &[],
            },
            Case {
                name: "edge 18: const users: [User, User] = pair; users.map(a).filter(b);",
                path: "file.ts",
                code: "const users: [User, User] = pair; users.map(a).filter(b);",
                expected: &[(
                    34,
                    56,
                    "Avoid consecutive array `map` and `filter` passes. Prefer `.values().map(...).filter(...).toArray()` where iterator helpers are supported, or a single `flatMap`/mutating reducer. Preserve callback ordering, indexes, and filtering semantics.",
                )],
            },
            Case {
                name: "edge 19: const users: (User[]) = x; users.map(a).filter(b);",
                path: "file.ts",
                code: "const users: (User[]) = x; users.map(a).filter(b);",
                expected: &[(
                    27,
                    49,
                    "Avoid consecutive array `map` and `filter` passes. Prefer `.values().map(...).filter(...).toArray()` where iterator helpers are supported, or a single `flatMap`/mutating reducer. Preserve callback ordering, indexes, and filtering semantics.",
                )],
            },
            Case {
                name: "edge 20: const users: Users = x; users.map(a).filter(b);",
                path: "file.ts",
                code: "const users: Users = x; users.map(a).filter(b);",
                expected: &[],
            },
            Case {
                name: "edge 21: let users: User[] = []; users = other; users.filter(a).map",
                path: "file.ts",
                code: "let users: User[] = []; users = other; users.filter(a).map(b);",
                expected: &[],
            },
            Case {
                name: "edge 22: const users = []; const copy = users.toSorted(); copy.filt",
                path: "file.ts",
                code: "const users = []; const copy = users.toSorted(); copy.filter(a).map(b);",
                expected: &[(
                    49,
                    70,
                    "Avoid consecutive array `filter` and `map` passes. Prefer `.values().filter(...).map(...).toArray()` where iterator helpers are supported, or a single `flatMap`/mutating reducer. Preserve callback ordering, indexes, and filtering semantics.",
                )],
            },
            Case {
                name: "edge 23: const users = []; users.values().toArray().filter(a).map(b",
                path: "file.ts",
                code: "const users = []; users.values().toArray().filter(a).map(b);",
                expected: &[],
            },
            Case {
                name: "edge 24: class C { users: User[] = []; f() { return this.users.filt",
                path: "file.ts",
                code: "class C { users: User[] = []; f() { return this.users.filter(a).map(b); } }",
                expected: &[],
            },
            Case { name: "edge 25: const users = []; users.filter(a).map;", path: "file.ts", code: "const users = []; users.filter(a).map;", expected: &[] },
            Case {
                name: "edge 26: try {} catch (users) { users.filter(a).map(b); }",
                path: "file.ts",
                code: "try {} catch (users) { users.filter(a).map(b); }",
                expected: &[],
            },
            Case {
                name: "edge 27: const users = []; users.filter(a).forEach(b);",
                path: "file.ts",
                code: "const users = []; users.filter(a).forEach(b);",
                expected: &[],
            },
            Case {
                name: "edge 28: const users = []; users?.['filter'](a).map(b);",
                path: "file.ts",
                code: "const users = []; users?.['filter'](a).map(b);",
                expected: &[(
                    18,
                    45,
                    "Avoid consecutive array `filter` and `map` passes. Prefer `.values().filter(...).map(...).toArray()` where iterator helpers are supported, or a single `flatMap`/mutating reducer. Preserve callback ordering, indexes, and filtering semantics.",
                )],
            },
        ],
    );
}
