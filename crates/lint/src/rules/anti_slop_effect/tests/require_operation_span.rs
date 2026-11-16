//! `anti-slop-effect/require-operation-span`: expectations recorded from the upstream plugin
//! (github.com/k-mistele/anti-slop) under oxlint 1.86, one case per upstream
//! RuleTester case in `src/effect/rules/require-operation-span.test.ts`.

use super::super::super::anti_slop::tests::{Case, check};

/// src/effect/rules/require-operation-span.test.ts
#[test]
fn upstream() {
    check(
        "anti-slop-effect/require-operation-span",
        "[]",
        &[
            Case {
                name: "valid 0: \timport * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "\n\timport * as Context from \"effect/Context\";\n\timport * as Effect from \"effect/Effect\";\n\timport * as Layer from \"effect/Layer\";\n\n\t\tconst listWidgets = Effect.fn(\"widgets.list\")(function* (input: Input) {});\n\t\texport class Widgets extends Context.Service<Widgets, WidgetsShape>()(\"@app/Widgets\", {\n\t\t\tmake: Effect.gen(function* () {\n\t\t\t\tconst provider = yield* Provider;\n\t\t\t\tconst find = Effect.fn(\"widgets.find\")(function* (id: string) {\n\t\t\t\t\treturn yield* provider.find(id);\n\t\t\t\t});\n\t\t\t\treturn {\n\t\t\t\t\tfind,\n\t\t\t\t\tget: Effect.fn(\"widgets.get\")(function* (id: string) {}),\n\t\t\t\t\tstatus: (input: Input) => Effect.gen(function* () {}).pipe(Effect.withSpan(\"widgets.status\", { attributes: { id: input.id } })),\n\t\t\t\t\tlist: (input: Input) => listWidgets(input).pipe(Effect.provideService(Provider, provider)),\n\t\t\t\t\tall: provider.all.pipe(Effect.withSpan(\"widgets.all\")),\n\t\t\t\t\tclient: provider.client,\n\t\t\t\t\tformat: (widget: Widget) => widget.name,\n\t\t\t\t\tdelegate: (id: string) => provider.find(id),\n\t\t\t\t\tchanges: Stream.fromQueue(queue),\n\t\t\t\t};\n\t\t\t}),\n\t\t}) {\n\t\t\tstatic readonly layer = Layer.effect(this, this.make);\n\t\t}",
                expected: &[],
            },
            Case {
                name: "valid 1: \timport * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "\n\timport * as Context from \"effect/Context\";\n\timport * as Effect from \"effect/Effect\";\n\timport * as Layer from \"effect/Layer\";\n\n\t\texport class Widgets extends Context.Service<Widgets, WidgetsShape>()(\"@app/Widgets\", {\n\t\t\tmake: Effect.gen(function* () {\n\t\t\t\tconst helper = () => { return { run: () => Effect.void }; };\n\t\t\t\treturn Widgets.of({ helper });\n\t\t\t}),\n\t\t}) {}",
                expected: &[],
            },
            Case {
                name: "valid 2: \timport * as Context from \"effect/Context\";",
                path: "widgets.test.ts",
                code: "\n\timport * as Context from \"effect/Context\";\n\timport * as Effect from \"effect/Effect\";\n\timport * as Layer from \"effect/Layer\";\n\n\t\t\texport class Fake extends Context.Service<Fake, FakeShape>()(\"@test/Fake\", {\n\t\t\t\tmake: Effect.gen(function* () { return { run: () => Effect.void }; }),\n\t\t\t}) {}",
                expected: &[],
            },
            Case {
                name: "invalid 3: \timport * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "\n\timport * as Context from \"effect/Context\";\n\timport * as Effect from \"effect/Effect\";\n\timport * as Layer from \"effect/Layer\";\n\n\t\t\texport class Widgets extends Context.Service<Widgets, WidgetsShape>()(\"@app/Widgets\", {\n\t\t\t\tmake: Effect.gen(function* () {\n\t\t\t\t\tconst store = yield* Store;\n\t\t\t\t\tconst remove = (id: string) => store.remove(id).pipe(Effect.asVoid);\n\t\t\t\t\tfunction count() { return Effect.succeed(1); }\n\t\t\t\t\treturn {\n\t\t\t\t\t\tfind: (id: string) => Effect.gen(function* () { return yield* store.find(id); }),\n\t\t\t\t\t\tremove,\n\t\t\t\t\t\tcount,\n\t\t\t\t\t\tuntraced: Effect.fnUntraced(function* () {}),\n\t\t\t\t\t\trefresh() { return Effect.sync(() => store.refresh()); },\n\t\t\t\t\t};\n\t\t\t\t}),\n\t\t\t}) {\n\t\t\t\tstatic readonly layer = Layer.effect(this, this.make);\n\t\t\t}",
                expected: &[
                    (
                        434,
                        514,
                        "Operation \"find\" of service \"Widgets\" has no span. Define it with `Effect.fn(\"Widgets.find\")(function* (input) { ... })`, or end its pipe with `Effect.withSpan(\"Widgets.find\", { attributes: { ... } })`.",
                    ),
                    (
                        522,
                        528,
                        "Operation \"remove\" of service \"Widgets\" has no span. Define it with `Effect.fn(\"Widgets.remove\")(function* (input) { ... })`, or end its pipe with `Effect.withSpan(\"Widgets.remove\", { attributes: { ... } })`.",
                    ),
                    (
                        536,
                        541,
                        "Operation \"count\" of service \"Widgets\" has no span. Define it with `Effect.fn(\"Widgets.count\")(function* (input) { ... })`, or end its pipe with `Effect.withSpan(\"Widgets.count\", { attributes: { ... } })`.",
                    ),
                    (
                        549,
                        593,
                        "Operation \"untraced\" of service \"Widgets\" has no span. Define it with `Effect.fn(\"Widgets.untraced\")(function* (input) { ... })`, or end its pipe with `Effect.withSpan(\"Widgets.untraced\", { attributes: { ... } })`.",
                    ),
                    (
                        601,
                        657,
                        "Operation \"refresh\" of service \"Widgets\" has no span. Define it with `Effect.fn(\"Widgets.refresh\")(function* (input) { ... })`, or end its pipe with `Effect.withSpan(\"Widgets.refresh\", { attributes: { ... } })`.",
                    ),
                ],
            },
            Case {
                name: "invalid 4: \timport * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "\n\timport * as Context from \"effect/Context\";\n\timport * as Effect from \"effect/Effect\";\n\timport * as Layer from \"effect/Layer\";\n\n\t\t\texport class Mailbox extends Context.Service<Mailbox, MailboxShape>()(\"@app/Mailbox\") {\n\t\t\t\tstatic readonly make = (options: Options) =>\n\t\t\t\t\tEffect.gen(function* () {\n\t\t\t\t\t\tconst operations = { process: (id: string) => Effect.log(id) };\n\t\t\t\t\t\treturn Mailbox.of(operations);\n\t\t\t\t\t}).pipe(Effect.annotateLogs({ mailbox: options.name }));\n\t\t\t\tstatic readonly layer = (options: Options) => Layer.effect(this, this.make(options));\n\t\t\t}",
                expected: &[(
                    326,
                    365,
                    "Operation \"process\" of service \"Mailbox\" has no span. Define it with `Effect.fn(\"Mailbox.process\")(function* (input) { ... })`, or end its pipe with `Effect.withSpan(\"Mailbox.process\", { attributes: { ... } })`.",
                )],
            },
        ],
    );
}

/// Edge cases the upstream tests miss, recorded from the upstream plugin.
#[test]
fn edges() {
    check(
        "anti-slop-effect/require-operation-span",
        "[]",
        &[
            Case {
                name: "valid 0: import * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "import * as Context from \"effect/Context\";\nimport * as Effect from \"effect/Effect\";\nimport * as Layer from \"effect/Layer\";\nexport class Widgets extends Context.Service<Widgets, Shape>()(\"Widgets\", {\n\tmake: Effect.gen(function* () {\n\t\tconst traced = Effect.fn(\"w.traced\")(function* () {});\n\t\tconst run = () => traced();\n\t\treturn { run };\n\t}),\n}) {\n\tstatic readonly layer = Layer.effect(this, this.make);\n}",
                expected: &[],
            },
            Case {
                name: "valid 1: import * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "import * as Context from \"effect/Context\";\nimport * as Effect from \"effect/Effect\";\nimport * as Layer from \"effect/Layer\";\nexport class Widgets extends Context.Service<Widgets, Shape>()(\"Widgets\", {\n\tmake: Effect.gen(function* () {\n\t\treturn { \"run\": () => Effect.void.pipe(Effect.withSpan(\"w.run\")) };\n\t}),\n}) {\n\tstatic readonly layer = Layer.effect(this, this.make);\n}",
                expected: &[],
            },
            Case {
                name: "valid 2: import * as Context from \"effect/Context\";",
                path: "widgets.spec.ts",
                code: "import * as Context from \"effect/Context\";\nimport * as Effect from \"effect/Effect\";\nimport * as Layer from \"effect/Layer\";\nexport class Widgets extends Context.Service<Widgets, Shape>()(\"Widgets\", {\n\tmake: Effect.gen(function* () {\n\t\treturn { run: () => Effect.void };\n\t}),\n}) {\n\tstatic readonly layer = Layer.effect(this, this.make);\n}",
                expected: &[],
            },
            Case {
                name: "invalid 3: import * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "import * as Context from \"effect/Context\";\nimport * as Effect from \"effect/Effect\";\nimport * as Layer from \"effect/Layer\";\nexport class Widgets extends Context.Service<Widgets, Shape>()(\"Widgets\", {\n\tmake: Effect.gen(function* () {\n\t\treturn Widgets.of({ \"run\": () => Effect.void, [key]: () => Effect.void, gen: Effect.gen(function* () {}) });\n\t}),\n}) {\n\tstatic readonly layer = Layer.effect(this, this.make);\n}",
                expected: &[(
                    306,
                    338,
                    "Operation \"gen\" of service \"Widgets\" has no span. Define it with `Effect.fn(\"Widgets.gen\")(function* (input) { ... })`, or end its pipe with `Effect.withSpan(\"Widgets.gen\", { attributes: { ... } })`.",
                )],
            },
            Case {
                name: "invalid 4: import * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "import * as Context from \"effect/Context\";\nimport * as Effect from \"effect/Effect\";\nimport * as Layer from \"effect/Layer\";\nexport class Widgets extends Context.Service<Widgets, Shape>()(\"Widgets\", {\n\tmake: Effect.gen(function* () {\n\t\tconst ops = { run: () => Effect.succeed(1) };\n\t\treturn ops;\n\t}),\n}) {\n\tstatic readonly layer = Layer.effect(this, this.make);\n}",
                expected: &[(
                    248,
                    276,
                    "Operation \"run\" of service \"Widgets\" has no span. Define it with `Effect.fn(\"Widgets.run\")(function* (input) { ... })`, or end its pipe with `Effect.withSpan(\"Widgets.run\", { attributes: { ... } })`.",
                )],
            },
            Case {
                name: "invalid 5: import { Context as C, Effect as E } from \"effect\";",
                path: "file.ts",
                code: "import { Context as C, Effect as E } from \"effect\";\nexport class Widgets extends C.Service<Widgets, Shape>()(\"Widgets\", {\n\tmake: E.gen(function* () { return { run: (id: string) => E.succeed(id) }; }),\n}) {}",
                expected: &[(
                    159,
                    193,
                    "Operation \"run\" of service \"Widgets\" has no span. Define it with `Effect.fn(\"Widgets.run\")(function* (input) { ... })`, or end its pipe with `Effect.withSpan(\"Widgets.run\", { attributes: { ... } })`.",
                )],
            },
        ],
    );
}
