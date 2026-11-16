//! `anti-slop-martinfowler/message-chains`: expectations recorded from the upstream plugin
//! (github.com/k-mistele/anti-slop) under oxlint 1.86, one case per upstream
//! RuleTester case in `src/martinfowler/rules/message-chains.test.ts`.

use super::super::super::anti_slop::tests::{Case, check};

/// src/martinfowler/rules/message-chains.test.ts
#[test]
fn upstream() {
    check(
        "anti-slop-martinfowler/message-chains",
        "[]",
        &[
            Case { name: "valid 0: const city = order.customer.address;", path: "file.ts", code: "const city = order.customer.address;", expected: &[] },
            Case {
                name: "valid 1: const city = order.customer.address.city;",
                path: "file.ts",
                code: "const city = order.customer.address.city;",
                expected: &[],
            },
            Case {
                name: "valid 2: const q = db.select().from(users).where(eq(users.id, id)).ord",
                path: "file.ts",
                code: "const q = db.select().from(users).where(eq(users.id, id)).orderBy(users.name).limit(1);",
                expected: &[],
            },
            Case {
                name: "valid 3: const s = z.string().min(1).max(10).optional();",
                path: "file.ts",
                code: "const s = z.string().min(1).max(10).optional();",
                expected: &[],
            },
            Case {
                name: "valid 4: const out = rows.filter(isActive).map(toDto).slice(0, 10).joi",
                path: "file.ts",
                code: "const out = rows.filter(isActive).map(toDto).slice(0, 10).join(',');",
                expected: &[],
            },
            Case {
                name: "valid 5: const program = layer.services.client.users.list.pipe(Effect.",
                path: "file.ts",
                code: "const program = layer.services.client.users.list.pipe(Effect.map(toUser));",
                expected: &[],
            },
            Case {
                name: "valid 6: const schema = Schema.Struct.fields.id.annotations;",
                path: "file.ts",
                code: "const schema = Schema.Struct.fields.id.annotations;",
                expected: &[],
            },
            Case {
                name: "valid 7: const flag = process.env.FEATURE.value.enabled;",
                path: "file.ts",
                code: "const flag = process.env.FEATURE.value.enabled;",
                expected: &[],
            },
            Case {
                name: "valid 8: const v = import.meta.env.VITE_APP.url.host;",
                path: "file.ts",
                code: "const v = import.meta.env.VITE_APP.url.host;",
                expected: &[],
            },
            Case {
                name: "valid 9: const x = config.servers[name].tls.cert;",
                path: "file.ts",
                code: "const x = config.servers[name].tls.cert;",
                expected: &[],
            },
            Case {
                name: "invalid 11: const name = order.customer.address.city.name;",
                path: "file.ts",
                code: "const name = order.customer.address.city.name;",
                expected: &[(
                    13,
                    45,
                    "Possibly Message Chains: `order.customer.address.city.name` navigates through 4 objects (limit 3), so this code depends on the structure of every object along the path. Consider Hide Delegate: ask the first object for what you need through one method (in Effect, a service operation that returns the value), or Extract Function on the chain and Move Function it next to the data it walks.",
                )],
            },
            Case {
                name: "invalid 12: const theme = session.getUser().getProfile().getSettings()",
                path: "file.ts",
                code: "const theme = session.getUser().getProfile().getSettings().getTheme();",
                expected: &[(
                    14,
                    69,
                    "Possibly Message Chains: `session.getUser().getProfile().getSettings().getTheme()` navigates through 4 objects (limit 3), so this code depends on the structure of every object along the path. Consider Hide Delegate: ask the first object for what you need through one method (in Effect, a service operation that returns the value), or Extract Function on the chain and Move Function it next to the data it walks.",
                )],
            },
            Case {
                name: "invalid 13: send(ctx.request.headers.authorization.token);",
                path: "file.ts",
                code: "send(ctx.request.headers.authorization.token);",
                expected: &[(
                    5,
                    44,
                    "Possibly Message Chains: `ctx.request.headers.authorization.token` navigates through 4 objects (limit 3), so this code depends on the structure of every object along the path. Consider Hide Delegate: ask the first object for what you need through one method (in Effect, a service operation that returns the value), or Extract Function on the chain and Move Function it next to the data it walks.",
                )],
            },
            Case {
                name: "invalid 14: this.deps.store.cache.entries.clear();",
                path: "file.ts",
                code: "this.deps.store.cache.entries.clear();",
                expected: &[(
                    0,
                    37,
                    "Possibly Message Chains: `this.deps.store.cache.entries.clear()` navigates through 4 objects (limit 3), so this code depends on the structure of every object along the path. Consider Hide Delegate: ask the first object for what you need through one method (in Effect, a service operation that returns the value), or Extract Function on the chain and Move Function it next to the data it walks.",
                )],
            },
            Case {
                name: "invalid 15: const id = rows[0].owner.team.org.id;",
                path: "file.ts",
                code: "const id = rows[0].owner.team.org.id;",
                expected: &[(
                    11,
                    36,
                    "Possibly Message Chains: `rows[0].owner.team.org.id` navigates through 4 objects (limit 3), so this code depends on the structure of every object along the path. Consider Hide Delegate: ask the first object for what you need through one method (in Effect, a service operation that returns the value), or Extract Function on the chain and Move Function it next to the data it walks.",
                )],
            },
        ],
    );
}

/// src/martinfowler/rules/message-chains.test.ts, options `[{"maxDepth":4}]`
#[test]
fn upstream_options_1() {
    check(
        "anti-slop-martinfowler/message-chains",
        "[{\"maxDepth\":4}]",
        &[Case {
            name: "valid 10: const city = order.customer.address.city.name;",
            path: "file.ts",
            code: "const city = order.customer.address.city.name;",
            expected: &[],
        }],
    );
}

/// src/martinfowler/rules/message-chains.test.ts, options `[{"ignorePascalCaseRoots":false}]`
#[test]
fn upstream_options_2() {
    check(
        "anti-slop-martinfowler/message-chains",
        "[{\"ignorePascalCaseRoots\":false}]",
        &[Case {
            name: "invalid 16: const p = Schema.Struct.fields.id.annotations;",
            path: "file.ts",
            code: "const p = Schema.Struct.fields.id.annotations;",
            expected: &[(
                10,
                45,
                "Possibly Message Chains: `Schema.Struct.fields.id.annotations` navigates through 4 objects (limit 3), so this code depends on the structure of every object along the path. Consider Hide Delegate: ask the first object for what you need through one method (in Effect, a service operation that returns the value), or Extract Function on the chain and Move Function it next to the data it walks.",
            )],
        }],
    );
}

/// Edge cases recorded from upstream under oxlint 1.86: src/martinfowler/rules/message-chains.mfedges.test.ts
#[test]
fn edges() {
    check(
        "anti-slop-martinfowler/message-chains",
        "[]",
        &[
            Case {
                name: "invalid 0: a?.b.c.d.e;",
                path: "file.ts",
                code: "a?.b.c.d.e;",
                expected: &[(
                    0,
                    10,
                    "Possibly Message Chains: `a?.b.c.d.e` navigates through 4 objects (limit 3), so this code depends on the structure of every object along the path. Consider Hide Delegate: ask the first object for what you need through one method (in Effect, a service operation that returns the value), or Extract Function on the chain and Move Function it next to the data it walks.",
                )],
            },
            Case {
                name: "invalid 1: a.b?.c.d.e;",
                path: "file.ts",
                code: "a.b?.c.d.e;",
                expected: &[(
                    0,
                    10,
                    "Possibly Message Chains: `a.b?.c.d.e` navigates through 4 objects (limit 3), so this code depends on the structure of every object along the path. Consider Hide Delegate: ask the first object for what you need through one method (in Effect, a service operation that returns the value), or Extract Function on the chain and Move Function it next to the data it walks.",
                )],
            },
            Case {
                name: "invalid 2: (a.b.c.d.e as X).f;",
                path: "file.ts",
                code: "(a.b.c.d.e as X).f;",
                expected: &[(
                    0,
                    18,
                    "Possibly Message Chains: `(a.b.c.d.e as X).f` navigates through 5 objects (limit 3), so this code depends on the structure of every object along the path. Consider Hide Delegate: ask the first object for what you need through one method (in Effect, a service operation that returns the value), or Extract Function on the chain and Move Function it next to the data it walks.",
                )],
            },
            Case {
                name: "invalid 3: (a.b.c.d as X).e;",
                path: "file.ts",
                code: "(a.b.c.d as X).e;",
                expected: &[(
                    0,
                    16,
                    "Possibly Message Chains: `(a.b.c.d as X).e` navigates through 4 objects (limit 3), so this code depends on the structure of every object along the path. Consider Hide Delegate: ask the first object for what you need through one method (in Effect, a service operation that returns the value), or Extract Function on the chain and Move Function it next to the data it walks.",
                )],
            },
            Case {
                name: "invalid 4: a.b.c.d.e!;",
                path: "file.ts",
                code: "a.b.c.d.e!;",
                expected: &[(
                    0,
                    10,
                    "Possibly Message Chains: `a.b.c.d.e!` navigates through 4 objects (limit 3), so this code depends on the structure of every object along the path. Consider Hide Delegate: ask the first object for what you need through one method (in Effect, a service operation that returns the value), or Extract Function on the chain and Move Function it next to the data it walks.",
                )],
            },
            Case {
                name: "invalid 5: a!.b!.c!.d!.e!;",
                path: "file.ts",
                code: "a!.b!.c!.d!.e!;",
                expected: &[(
                    0,
                    14,
                    "Possibly Message Chains: `a!.b!.c!.d!.e!` navigates through 4 objects (limit 3), so this code depends on the structure of every object along the path. Consider Hide Delegate: ask the first object for what you need through one method (in Effect, a service operation that returns the value), or Extract Function on the chain and Move Function it next to the data it walks.",
                )],
            },
            Case {
                name: "invalid 6: (a.b).c.d.e;",
                path: "file.ts",
                code: "(a.b).c.d.e;",
                expected: &[(
                    0,
                    11,
                    "Possibly Message Chains: `(a.b).c.d.e` navigates through 4 objects (limit 3), so this code depends on the structure of every object along the path. Consider Hide Delegate: ask the first object for what you need through one method (in Effect, a service operation that returns the value), or Extract Function on the chain and Move Function it next to the data it walks.",
                )],
            },
            Case {
                name: "invalid 7: ((a.b.c.d.e));",
                path: "file.ts",
                code: "((a.b.c.d.e));",
                expected: &[(
                    2,
                    11,
                    "Possibly Message Chains: `a.b.c.d.e` navigates through 4 objects (limit 3), so this code depends on the structure of every object along the path. Consider Hide Delegate: ask the first object for what you need through one method (in Effect, a service operation that returns the value), or Extract Function on the chain and Move Function it next to the data it walks.",
                )],
            },
            Case { name: "invalid 8: a.b.c.d.e();", path: "file.ts", code: "a.b.c.d.e();", expected: &[] },
            Case {
                name: "invalid 9: a.b.c.d.e.f();",
                path: "file.ts",
                code: "a.b.c.d.e.f();",
                expected: &[(
                    0,
                    13,
                    "Possibly Message Chains: `a.b.c.d.e.f()` navigates through 4 objects (limit 3), so this code depends on the structure of every object along the path. Consider Hide Delegate: ask the first object for what you need through one method (in Effect, a service operation that returns the value), or Extract Function on the chain and Move Function it next to the data it walks.",
                )],
            },
            Case {
                name: "invalid 10: class A { m() { this.#a.b.c.d; } #a; }",
                path: "file.ts",
                code: "class A { m() { this.#a.b.c.d; } #a; }",
                expected: &[(
                    16,
                    29,
                    "Possibly Message Chains: `this.#a.b.c.d` navigates through 4 objects (limit 3), so this code depends on the structure of every object along the path. Consider Hide Delegate: ask the first object for what you need through one method (in Effect, a service operation that returns the value), or Extract Function on the chain and Move Function it next to the data it walks.",
                )],
            },
            Case {
                name: "invalid 11: a[0].b.c.d.e;",
                path: "file.ts",
                code: "a[0].b.c.d.e;",
                expected: &[(
                    0,
                    12,
                    "Possibly Message Chains: `a[0].b.c.d.e` navigates through 4 objects (limit 3), so this code depends on the structure of every object along the path. Consider Hide Delegate: ask the first object for what you need through one method (in Effect, a service operation that returns the value), or Extract Function on the chain and Move Function it next to the data it walks.",
                )],
            },
            Case {
                name: "invalid 12: a['x'].b.c.d.e;",
                path: "file.ts",
                code: "a['x'].b.c.d.e;",
                expected: &[(
                    0,
                    14,
                    "Possibly Message Chains: `a['x'].b.c.d.e` navigates through 4 objects (limit 3), so this code depends on the structure of every object along the path. Consider Hide Delegate: ask the first object for what you need through one method (in Effect, a service operation that returns the value), or Extract Function on the chain and Move Function it next to the data it walks.",
                )],
            },
            Case {
                name: "invalid 13: a[`x`].b.c.d.e;",
                path: "file.ts",
                code: "a[`x`].b.c.d.e;",
                expected: &[(
                    0,
                    14,
                    "Possibly Message Chains: `a[`x`].b.c.d.e` navigates through 4 objects (limit 3), so this code depends on the structure of every object along the path. Consider Hide Delegate: ask the first object for what you need through one method (in Effect, a service operation that returns the value), or Extract Function on the chain and Move Function it next to the data it walks.",
                )],
            },
            Case {
                name: "invalid 14: a[null].b[1n].c[/r/].d[true].e;",
                path: "file.ts",
                code: "a[null].b[1n].c[/r/].d[true].e;",
                expected: &[(
                    0,
                    30,
                    "Possibly Message Chains: `a[null].b[1n].c[/r/].d[true].e` navigates through 4 objects (limit 3), so this code depends on the structure of every object along the path. Consider Hide Delegate: ask the first object for what you need through one method (in Effect, a service operation that returns the value), or Extract Function on the chain and Move Function it next to the data it walks.",
                )],
            },
            Case { name: "invalid 15: a.b.c[k].d.e.f;", path: "file.ts", code: "a.b.c[k].d.e.f;", expected: &[] },
            Case {
                name: "invalid 16: class A extends B { m() { super.a.b.c.d.e; } }",
                path: "file.ts",
                code: "class A extends B { m() { super.a.b.c.d.e; } }",
                expected: &[],
            },
            Case { name: "invalid 17: function F() { new.target.a.b.c.d; }", path: "file.ts", code: "function F() { new.target.a.b.c.d; }", expected: &[] },
            Case { name: "invalid 18: window.a.b.c.d.e;", path: "file.ts", code: "window.a.b.c.d.e;", expected: &[] },
            Case {
                name: "invalid 19: foo().a.b.c.d.e;",
                path: "file.ts",
                code: "foo().a.b.c.d.e;",
                expected: &[(
                    0,
                    15,
                    "Possibly Message Chains: `foo().a.b.c.d.e` navigates through 5 objects (limit 3), so this code depends on the structure of every object along the path. Consider Hide Delegate: ask the first object for what you need through one method (in Effect, a service operation that returns the value), or Extract Function on the chain and Move Function it next to the data it walks.",
                )],
            },
            Case {
                name: "invalid 20: (await x).a.b.c.d;",
                path: "file.ts",
                code: "(await x).a.b.c.d;",
                expected: &[(
                    0,
                    17,
                    "Possibly Message Chains: `(await x).a.b.c.d` navigates through 4 objects (limit 3), so this code depends on the structure of every object along the path. Consider Hide Delegate: ask the first object for what you need through one method (in Effect, a service operation that returns the value), or Extract Function on the chain and Move Function it next to the data it walks.",
                )],
            },
            Case {
                name: "invalid 21: order.customerAccountInformation.billingAddressDetails.cit",
                path: "file.ts",
                code: "order.customerAccountInformation.billingAddressDetails.cityNameValue.lowercase;",
                expected: &[(
                    0,
                    78,
                    "Possibly Message Chains: `order.customerAccountInformation.billingAddressDetails.city…` navigates through 4 objects (limit 3), so this code depends on the structure of every object along the path. Consider Hide Delegate: ask the first object for what you need through one method (in Effect, a service operation that returns the value), or Extract Function on the chain and Move Function it next to the data it walks.",
                )],
            },
            Case {
                name: "invalid 22: order.customerAccountInformation.billingAddressDetails.cit",
                path: "file.ts",
                code: "order.customerAccountInformation.billingAddressDetails.cityNameValu.lowercase;",
                expected: &[(
                    0,
                    77,
                    "Possibly Message Chains: `order.customerAccountInformation.billingAddressDetails.city…` navigates through 4 objects (limit 3), so this code depends on the structure of every object along the path. Consider Hide Delegate: ask the first object for what you need through one method (in Effect, a service operation that returns the value), or Extract Function on the chain and Move Function it next to the data it walks.",
                )],
            },
            Case {
                name: "invalid 23: order",
                path: "file.ts",
                code: "order\n  .customer\n  .address\n  .city\n  .name;",
                expected: &[(
                    0,
                    44,
                    "Possibly Message Chains: `order .customer .address .city .name` navigates through 4 objects (limit 3), so this code depends on the structure of every object along the path. Consider Hide Delegate: ask the first object for what you need through one method (in Effect, a service operation that returns the value), or Extract Function on the chain and Move Function it next to the data it walks.",
                )],
            },
            Case {
                name: "invalid 24: a.b.c.d.e = 1;",
                path: "file.ts",
                code: "a.b.c.d.e = 1;",
                expected: &[(
                    0,
                    9,
                    "Possibly Message Chains: `a.b.c.d.e` navigates through 4 objects (limit 3), so this code depends on the structure of every object along the path. Consider Hide Delegate: ask the first object for what you need through one method (in Effect, a service operation that returns the value), or Extract Function on the chain and Move Function it next to the data it walks.",
                )],
            },
            Case {
                name: "invalid 25: a.b.c.d.e++;",
                path: "file.ts",
                code: "a.b.c.d.e++;",
                expected: &[(
                    0,
                    9,
                    "Possibly Message Chains: `a.b.c.d.e` navigates through 4 objects (limit 3), so this code depends on the structure of every object along the path. Consider Hide Delegate: ask the first object for what you need through one method (in Effect, a service operation that returns the value), or Extract Function on the chain and Move Function it next to the data it walks.",
                )],
            },
            Case { name: "invalid 26: a.b.c.d.e.pipe(x);", path: "file.ts", code: "a.b.c.d.e.pipe(x);", expected: &[] },
            Case { name: "invalid 27: a.pipe(x).b.c.d.e;", path: "file.ts", code: "a.pipe(x).b.c.d.e;", expected: &[] },
            Case {
                name: "invalid 28: a.getB().getC().getD().getE();",
                path: "file.ts",
                code: "a.getB().getC().getD().getE();",
                expected: &[(
                    0,
                    29,
                    "Possibly Message Chains: `a.getB().getC().getD().getE()` navigates through 4 objects (limit 3), so this code depends on the structure of every object along the path. Consider Hide Delegate: ask the first object for what you need through one method (in Effect, a service operation that returns the value), or Extract Function on the chain and Move Function it next to the data it walks.",
                )],
            },
            Case { name: "invalid 29: a.get().b.c.d;", path: "file.ts", code: "a.get().b.c.d;", expected: &[] },
            Case { name: "invalid 30: a.getB(1).c.d.e;", path: "file.ts", code: "a.getB(1).c.d.e;", expected: &[] },
            Case { name: "invalid 31: a.getb().c.d.e;", path: "file.ts", code: "a.getb().c.d.e;", expected: &[] },
            Case {
                name: "invalid 32: x = a.b.c.d.e(1).f.g.h.i.j;",
                path: "file.ts",
                code: "x = a.b.c.d.e(1).f.g.h.i.j;",
                expected: &[(
                    4,
                    26,
                    "Possibly Message Chains: `a.b.c.d.e(1).f.g.h.i.j` navigates through 5 objects (limit 3), so this code depends on the structure of every object along the path. Consider Hide Delegate: ask the first object for what you need through one method (in Effect, a service operation that returns the value), or Extract Function on the chain and Move Function it next to the data it walks.",
                )],
            },
            Case {
                name: "invalid 33: `${a.b.c.d.e}`;",
                path: "file.ts",
                code: "`${a.b.c.d.e}`;",
                expected: &[(
                    3,
                    12,
                    "Possibly Message Chains: `a.b.c.d.e` navigates through 4 objects (limit 3), so this code depends on the structure of every object along the path. Consider Hide Delegate: ask the first object for what you need through one method (in Effect, a service operation that returns the value), or Extract Function on the chain and Move Function it next to the data it walks.",
                )],
            },
            Case { name: "invalid 34: a.b.c.d.e?.();", path: "file.ts", code: "a.b.c.d.e?.();", expected: &[] },
            Case {
                name: "invalid 35: a.b.c.d.e.f.g.h.i.j.k.l.m.n.o.p.q.r.s.t.u.v.w.x.y.z.aa.bb.",
                path: "file.ts",
                code: "a.b.c.d.e.f.g.h.i.j.k.l.m.n.o.p.q.r.s.t.u.v.w.x.y.z.aa.bb.cc.dd.ee;",
                expected: &[(
                    0,
                    66,
                    "Possibly Message Chains: `a.b.c.d.e.f.g.h.i.j.k.l.m.n.o.p.q.r.s.t.u.v.w.x.y.z.aa.bb.c…` navigates through 30 objects (limit 3), so this code depends on the structure of every object along the path. Consider Hide Delegate: ask the first object for what you need through one method (in Effect, a service operation that returns the value), or Extract Function on the chain and Move Function it next to the data it walks.",
                )],
            },
            Case {
                name: "invalid 36: const { x } = a.b.c.d.e;",
                path: "file.ts",
                code: "const { x } = a.b.c.d.e;",
                expected: &[(
                    14,
                    23,
                    "Possibly Message Chains: `a.b.c.d.e` navigates through 4 objects (limit 3), so this code depends on the structure of every object along the path. Consider Hide Delegate: ask the first object for what you need through one method (in Effect, a service operation that returns the value), or Extract Function on the chain and Move Function it next to the data it walks.",
                )],
            },
            Case {
                name: "invalid 37: f(a.b.c.d.e, g.h.i.j.k);",
                path: "file.ts",
                code: "f(a.b.c.d.e, g.h.i.j.k);",
                expected: &[
                    (
                        2,
                        11,
                        "Possibly Message Chains: `a.b.c.d.e` navigates through 4 objects (limit 3), so this code depends on the structure of every object along the path. Consider Hide Delegate: ask the first object for what you need through one method (in Effect, a service operation that returns the value), or Extract Function on the chain and Move Function it next to the data it walks.",
                    ),
                    (
                        13,
                        22,
                        "Possibly Message Chains: `g.h.i.j.k` navigates through 4 objects (limit 3), so this code depends on the structure of every object along the path. Consider Hide Delegate: ask the first object for what you need through one method (in Effect, a service operation that returns the value), or Extract Function on the chain and Move Function it next to the data it walks.",
                    ),
                ],
            },
            Case {
                name: "invalid 38: a[b.c.d.e.f];",
                path: "file.ts",
                code: "a[b.c.d.e.f];",
                expected: &[(
                    2,
                    11,
                    "Possibly Message Chains: `b.c.d.e.f` navigates through 4 objects (limit 3), so this code depends on the structure of every object along the path. Consider Hide Delegate: ask the first object for what you need through one method (in Effect, a service operation that returns the value), or Extract Function on the chain and Move Function it next to the data it walks.",
                )],
            },
            Case {
                name: "invalid 39: a.b.c.d.e`t`;",
                path: "file.ts",
                code: "a.b.c.d.e`t`;",
                expected: &[(
                    0,
                    9,
                    "Possibly Message Chains: `a.b.c.d.e` navigates through 4 objects (limit 3), so this code depends on the structure of every object along the path. Consider Hide Delegate: ask the first object for what you need through one method (in Effect, a service operation that returns the value), or Extract Function on the chain and Move Function it next to the data it walks.",
                )],
            },
            Case {
                name: "invalid 40: new a.b.c.d.e();",
                path: "file.ts",
                code: "new a.b.c.d.e();",
                expected: &[(
                    4,
                    13,
                    "Possibly Message Chains: `a.b.c.d.e` navigates through 4 objects (limit 3), so this code depends on the structure of every object along the path. Consider Hide Delegate: ask the first object for what you need through one method (in Effect, a service operation that returns the value), or Extract Function on the chain and Move Function it next to the data it walks.",
                )],
            },
            Case {
                name: "invalid 41: <T,>() => a.b.c.d.e<T>;",
                path: "file.ts",
                code: "<T,>() => a.b.c.d.e<T>;",
                expected: &[(
                    10,
                    19,
                    "Possibly Message Chains: `a.b.c.d.e` navigates through 4 objects (limit 3), so this code depends on the structure of every object along the path. Consider Hide Delegate: ask the first object for what you need through one method (in Effect, a service operation that returns the value), or Extract Function on the chain and Move Function it next to the data it walks.",
                )],
            },
            Case { name: "invalid 45: a.b.c.d.e;", path: "src/a.test.ts", code: "a.b.c.d.e;", expected: &[] },
        ],
    );
}

/// Edge cases recorded from upstream under oxlint 1.86: src/martinfowler/rules/message-chains.mfedges.test.ts, options `[{"maxDepth":1}]`
#[test]
fn edges_options_1() {
    check(
        "anti-slop-martinfowler/message-chains",
        "[{\"maxDepth\":1}]",
        &[Case {
            name: "invalid 42: a.b.c;",
            path: "file.ts",
            code: "a.b.c;",
            expected: &[(
                0,
                5,
                "Possibly Message Chains: `a.b.c` navigates through 2 objects (limit 1), so this code depends on the structure of every object along the path. Consider Hide Delegate: ask the first object for what you need through one method (in Effect, a service operation that returns the value), or Extract Function on the chain and Move Function it next to the data it walks.",
            )],
        }],
    );
}

/// Edge cases recorded from upstream under oxlint 1.86: src/martinfowler/rules/message-chains.mfedges.test.ts, options `[{"ignoreRoots":["order"]}]`
#[test]
fn edges_options_2() {
    check(
        "anti-slop-martinfowler/message-chains",
        "[{\"ignoreRoots\":[\"order\"]}]",
        &[Case {
            name: "invalid 43: window.a.b.c.d.e; order.a.b.c.d;",
            path: "file.ts",
            code: "window.a.b.c.d.e; order.a.b.c.d;",
            expected: &[(
                0,
                16,
                "Possibly Message Chains: `window.a.b.c.d.e` navigates through 5 objects (limit 3), so this code depends on the structure of every object along the path. Consider Hide Delegate: ask the first object for what you need through one method (in Effect, a service operation that returns the value), or Extract Function on the chain and Move Function it next to the data it walks.",
            )],
        }],
    );
}

/// Edge cases recorded from upstream under oxlint 1.86: src/martinfowler/rules/message-chains.mfedges.test.ts, options `[{"ignoreRoots":[]}]`
#[test]
fn edges_options_3() {
    check(
        "anti-slop-martinfowler/message-chains",
        "[{\"ignoreRoots\":[]}]",
        &[Case { name: "invalid 44: Schema.a.b.c.d;", path: "file.ts", code: "Schema.a.b.c.d;", expected: &[] }],
    );
}

/// Edge cases recorded from upstream under oxlint 1.86: src/martinfowler/rules/message-chains.mfedges.test.ts, options `[{"ignoreTests":false}]`
#[test]
fn edges_options_4() {
    check(
        "anti-slop-martinfowler/message-chains",
        "[{\"ignoreTests\":false}]",
        &[Case {
            name: "invalid 46: a.b.c.d.e;",
            path: "src/a.test.ts",
            code: "a.b.c.d.e;",
            expected: &[(
                0,
                9,
                "Possibly Message Chains: `a.b.c.d.e` navigates through 4 objects (limit 3), so this code depends on the structure of every object along the path. Consider Hide Delegate: ask the first object for what you need through one method (in Effect, a service operation that returns the value), or Extract Function on the chain and Move Function it next to the data it walks.",
            )],
        }],
    );
}
