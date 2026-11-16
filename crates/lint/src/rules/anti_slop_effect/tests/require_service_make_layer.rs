//! `anti-slop-effect/require-service-make-layer`: expectations recorded from the upstream plugin
//! (github.com/k-mistele/anti-slop) under oxlint 1.86, one case per upstream
//! RuleTester case in `src/effect/rules/require-service-make-layer.test.ts`.

use super::super::super::anti_slop::tests::{Case, check};

/// src/effect/rules/require-service-make-layer.test.ts
#[test]
fn upstream() {
    check(
        "anti-slop-effect/require-service-make-layer",
        "[]",
        &[
            Case {
                name: "valid 0: \timport * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "\n\timport * as Context from \"effect/Context\";\n\timport * as Effect from \"effect/Effect\";\n\timport * as Layer from \"effect/Layer\";\n\n\t\texport class AppRegistry extends Context.Service<AppRegistry, { readonly find: () => Effect.Effect<void> }>()(\"@app/AppRegistry\", {\n\t\t\tmake: Effect.gen(function* () {\n\t\t\t\treturn { find: () => Effect.void };\n\t\t\t}),\n\t\t}) {\n\t\t\tstatic readonly layer = Layer.effect(this, this.make);\n\t\t}",
                expected: &[],
            },
            Case {
                name: "valid 1: \timport * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "\n\timport * as Context from \"effect/Context\";\n\timport * as Effect from \"effect/Effect\";\n\timport * as Layer from \"effect/Layer\";\n\n\t\texport class Mailbox extends Context.Service<Mailbox, { readonly id: string }>()(\"@app/Mailbox\") {\n\t\t\tstatic readonly make = (options: { readonly id: string }) => Effect.gen(function* () {\n\t\t\t\treturn { id: options.id };\n\t\t\t});\n\t\t\tstatic readonly layer = (options: { readonly id: string }) => Layer.effect(this, this.make(options));\n\t\t}",
                expected: &[],
            },
            Case {
                name: "valid 2: \timport * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "\n\timport * as Context from \"effect/Context\";\n\timport * as Effect from \"effect/Effect\";\n\timport * as Layer from \"effect/Layer\";\n\n\t\texport class Widgets extends Context.Service<Widgets, { readonly id: string }>()(\"@app/Widgets\", {\n\t\t\tmake: Effect.succeed({ id: \"w\" }),\n\t\t}) {\n\t\t\tstatic readonly layer = Layer.effect(Widgets, Widgets.make).pipe(Layer.provide(Other.layer));\n\t\t\tstatic readonly layerSelfBinding = Widgets.layer.pipe(Layer.provide(selfBinding));\n\t\t\tstatic readonly layerFor = (id: string) => Layer.effect(this, this.make).pipe(Layer.provide(Config.layer(id)));\n\t\t\tstatic readonly layerRouter = HttpRouter.middleware(Effect.succeed(1));\n\t\t\tstatic readonly layerStreams = Layer.effect(Streams, Widgets.use((widgets) => Effect.succeed(widgets)));\n\t\t}",
                expected: &[],
            },
            Case {
                name: "valid 3: \timport * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "\n\timport * as Context from \"effect/Context\";\n\timport * as Effect from \"effect/Effect\";\n\timport * as Layer from \"effect/Layer\";\n\n\t\texport class WorkerBindings extends Context.Service<WorkerBindings, { readonly loader: Loader }>()(\"@app/WorkerBindings\") {}",
                expected: &[],
            },
            Case {
                name: "valid 4: \timport * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "\n\timport * as Context from \"effect/Context\";\n\timport * as Effect from \"effect/Effect\";\n\timport * as Layer from \"effect/Layer\";\n\n\t\texport const live = Layer.effect(AppRegistry, AppRegistry.make);\n\t\texport const configured = Layer.effect(Mailbox, Mailbox.make({ id: \"x\" }));",
                expected: &[],
            },
            Case {
                name: "valid 5: \timport * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "\n\timport * as Context from \"effect/Context\";\n\timport * as Effect from \"effect/Effect\";\n\timport * as Layer from \"effect/Layer\";\n\n\t\timport { LanguageModel } from \"effect/unstable/ai\";\n\t\texport const layer = Layer.effect(LanguageModel.LanguageModel, make(options));",
                expected: &[],
            },
            Case {
                name: "valid 6: \t\timport * as Context from \"other-library\";",
                path: "file.ts",
                code: "\n\t\timport * as Context from \"other-library\";\n\t\timport * as Layer from \"effect/Layer\";\n\t\texport class Plain extends Context.Service<Plain, {}>()(\"plain\") {\n\t\t\tstatic readonly layer = Layer.succeed(this, {});\n\t\t}",
                expected: &[],
            },
            Case {
                name: "valid 7: \timport * as Context from \"effect/Context\";",
                path: "registry.test.ts",
                code: "\n\timport * as Context from \"effect/Context\";\n\timport * as Effect from \"effect/Effect\";\n\timport * as Layer from \"effect/Layer\";\n\n\t\t\texport const layerTest = Layer.effect(AppRegistry, Effect.gen(function* () { return {}; }));",
                expected: &[],
            },
            Case {
                name: "invalid 9: \timport * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "\n\timport * as Context from \"effect/Context\";\n\timport * as Effect from \"effect/Effect\";\n\timport * as Layer from \"effect/Layer\";\n\n\t\t\texport class Builds extends Context.Service<Builds, { readonly latest: Effect.Effect<void> }>()(\"@app/Builds\") {\n\t\t\t\tstatic readonly layerWatch = (input: Input) => Layer.effect(this, Effect.gen(function* () { return { latest: Effect.void }; }));\n\t\t\t}",
                expected: &[(
                    144,
                    150,
                    "Service \"Builds\" has no `make`. Add `make: Effect.gen(function* () { ...yield* dependencies...; return { ...operations } })` to its Context.Service options (or `static readonly make = (options: Options) => Effect.gen(...)` when construction needs plain options), then add `static readonly layer = Layer.effect(this, this.make)`.",
                )],
            },
            Case {
                name: "invalid 10: \timport * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "\n\timport * as Context from \"effect/Context\";\n\timport * as Effect from \"effect/Effect\";\n\timport * as Layer from \"effect/Layer\";\n\n\t\t\texport class SkillsRoot extends Context.Service<SkillsRoot, string>()(\"@app/SkillsRoot\", {\n\t\t\t\tmake: Effect.succeed(\"\"),\n\t\t\t}) {}",
                expected: &[(
                    144,
                    154,
                    "Service \"SkillsRoot\" has a `make` but no `static layer`. Add `static readonly layer = Layer.effect(this, this.make)`, or `static readonly layer = (options: Options) => Layer.effect(this, this.make(options))` when make takes options.",
                )],
            },
            Case {
                name: "invalid 11: \timport * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "\n\timport * as Context from \"effect/Context\";\n\timport * as Effect from \"effect/Effect\";\n\timport * as Layer from \"effect/Layer\";\n\n\t\t\texport class Store extends Context.Service<Store, StoreShape>()(\"@app/Store\", {\n\t\t\t\tmake: (impl: StoreShape) => Effect.succeed(impl),\n\t\t\t}) {\n\t\t\t\tstatic readonly layer = (impl: StoreShape) => Layer.succeed(this, impl);\n\t\t\t}",
                expected: &[(
                    277,
                    349,
                    "`Store.layer` must be built from `Store.make`. Write `static readonly layer = Layer.effect(this, this.make)` (add `.pipe(Layer.provide(...))` for local wiring), or `(options: Options) => Layer.effect(this, this.make(options))`; move any construction code into make.",
                )],
            },
            Case {
                name: "invalid 12: \timport * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "\n\timport * as Context from \"effect/Context\";\n\timport * as Effect from \"effect/Effect\";\n\timport * as Layer from \"effect/Layer\";\n\n\t\t\texport class Client extends Context.Service<Client, ClientShape>()(\"@app/Client\") {\n\t\t\t\tstatic readonly make = makeClient;\n\t\t\t\tstatic readonly layer = (config: Config) => Layer.effect(Client, makeClient(config));\n\t\t\t}",
                expected: &[(
                    258,
                    343,
                    "`Client.layer` must be built from `Client.make`. Write `static readonly layer = Layer.effect(this, this.make)` (add `.pipe(Layer.provide(...))` for local wiring), or `(options: Options) => Layer.effect(this, this.make(options))`; move any construction code into make.",
                )],
            },
            Case {
                name: "invalid 13: \timport * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "\n\timport * as Context from \"effect/Context\";\n\timport * as Effect from \"effect/Effect\";\n\timport * as Layer from \"effect/Layer\";\n\n\t\t\texport class Photon extends Context.Service<Photon, PhotonShape>()(\"@app/Photon\", {\n\t\t\t\tmake: Effect.succeed(photon),\n\t\t\t}) {\n\t\t\t\tstatic readonly layer = Layer.effect(this, this.make);\n\t\t\t\tstatic readonly layerUnavailable = (cause: Error) => Layer.succeed(Photon, unavailable(cause));\n\t\t\t}",
                expected: &[(
                    320,
                    415,
                    "`Photon.layerUnavailable` does not derive from `Photon.layer` or `Photon.make`. Write it as `Photon.layer.pipe(Layer.provide(...))` or `Layer.effect(this, this.make(options))`. A separate implementation belongs in its own service, or in test/ when it is a fake.",
                )],
            },
            Case {
                name: "invalid 14: \timport * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "\n\timport * as Context from \"effect/Context\";\n\timport * as Effect from \"effect/Effect\";\n\timport * as Layer from \"effect/Layer\";\n\n\t\t\timport { Foo } from \"./foo.ts\";\n\t\t\texport const FooLive = Layer.effect(Foo, Effect.gen(function* () {\n\t\t\t\treturn { run: () => Effect.void };\n\t\t\t}));",
                expected: &[(
                    189,
                    278,
                    "This Layer.effect builds `Foo` from an inline constructor. Move the constructor into the service class as `make: Effect.gen(...)`, add `static readonly layer = Layer.effect(this, this.make)`, and use `Foo.layer` here.",
                )],
            },
            Case {
                name: "invalid 15: \t\t\timport { Context, Effect, Layer } from \"effect\";",
                path: "file.ts",
                code: "\n\t\t\timport { Context, Effect, Layer } from \"effect\";\n\t\t\texport class Board extends Context.Service<Board, BoardShape>()(\"@app/Board\") {}\n\t\t\texport const layerBoard = Layer.effect(Board, makeBoard);",
                expected: &[(
                    166,
                    196,
                    "This Layer.effect builds `Board` from an inline constructor. Move the constructor into the service class as `make: Effect.gen(...)`, add `static readonly layer = Layer.effect(this, this.make)`, and use `Board.layer` here.",
                )],
            },
        ],
    );
}

/// src/effect/rules/require-service-make-layer.test.ts, options `[{"allowTagOnly":false}]`
#[test]
fn upstream_options_1() {
    check(
        "anti-slop-effect/require-service-make-layer",
        "[{\"allowTagOnly\":false}]",
        &[Case {
            name: "invalid 8: \timport * as Context from \"effect/Context\";",
            path: "file.ts",
            code: "\n\timport * as Context from \"effect/Context\";\n\timport * as Effect from \"effect/Effect\";\n\timport * as Layer from \"effect/Layer\";\n\n\t\t\texport class WorkerBindings extends Context.Service<WorkerBindings, { readonly loader: Loader }>()(\"@app/WorkerBindings\") {}",
            expected: &[(
                144,
                158,
                "Service \"WorkerBindings\" has no `make`. Add `make: Effect.gen(function* () { ...yield* dependencies...; return { ...operations } })` to its Context.Service options (or `static readonly make = (options: Options) => Effect.gen(...)` when construction needs plain options), then add `static readonly layer = Layer.effect(this, this.make)`.",
            )],
        }],
    );
}

/// Edge cases the upstream tests miss, recorded from the upstream plugin.
#[test]
fn edges() {
    check(
        "anti-slop-effect/require-service-make-layer",
        "[]",
        &[
            Case {
                name: "valid 0: import * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "import * as Context from \"effect/Context\";\nimport * as Effect from \"effect/Effect\";\nimport * as Layer from \"effect/Layer\";\nexport class Store extends Context.Service<Store, Shape>()(\"Store\", { make: Effect.gen(function* () { return {}; }) }) {\n\tstatic readonly layer = Layer.effect(this, this.make).pipe(Layer.provide(Dep.layer));\n\tstatic readonly layerTest = Store.layer.pipe(Layer.provide(Fake));\n\tstatic readonly layerOther = Layer.succeed(Other, {});\n}",
                expected: &[],
            },
            Case {
                name: "valid 1: import * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "import * as Context from \"effect/Context\";\nimport * as Effect from \"effect/Effect\";\nimport * as Layer from \"effect/Layer\";\nexport class Store extends Context.Service<Store, Shape>()(\"Store\", { make: Effect.gen(function* () { return {}; }) }) {\n\tstatic layer(options: Options) { return Layer.effect(Store, Store.make(options)); }\n}",
                expected: &[],
            },
            Case {
                name: "valid 2: import { Layer } from \"effect\";",
                path: "file.ts",
                code: "import { Layer } from \"effect\";\nimport { FileSystem } from \"@effect/platform\";\nconst live = Layer.effect(FileSystem.FileSystem, build);",
                expected: &[],
            },
            Case {
                name: "valid 3: import * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "import * as Context from \"effect/Context\";\nimport * as Effect from \"effect/Effect\";\nimport * as Layer from \"effect/Layer\";\nconst live = Layer.effect((Store), Store.make);",
                expected: &[],
            },
            Case {
                name: "valid 4: import * as Context from \"effect/Context\";",
                path: "store.test.ts",
                code: "import * as Context from \"effect/Context\";\nimport * as Effect from \"effect/Effect\";\nimport * as Layer from \"effect/Layer\";\nconst live = Layer.effect(Store, build);",
                expected: &[],
            },
            Case {
                name: "invalid 5: import * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "import * as Context from \"effect/Context\";\nimport * as Effect from \"effect/Effect\";\nimport * as Layer from \"effect/Layer\";\nexport class Store extends Context.Service<Store, Shape>()(\"Store\") { static readonly layerLive = Layer.succeed(this, {}); }",
                expected: &[(
                    136,
                    141,
                    "Service \"Store\" has no `make`. Add `make: Effect.gen(function* () { ...yield* dependencies...; return { ...operations } })` to its Context.Service options (or `static readonly make = (options: Options) => Effect.gen(...)` when construction needs plain options), then add `static readonly layer = Layer.effect(this, this.make)`.",
                )],
            },
            Case {
                name: "invalid 6: import * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "import * as Context from \"effect/Context\";\nimport * as Effect from \"effect/Effect\";\nimport * as Layer from \"effect/Layer\";\nexport class Store extends Context.Service<Store, Shape>()(\"Store\", { make: Effect.gen(function* () { return {}; }) }) {\n\tstatic readonly layer = Layer.effect(this, build);\n\tstatic readonly layerLive = Layer.sync(Store, () => ({}));\n}",
                expected: &[
                    (
                        245,
                        295,
                        "`Store.layer` must be built from `Store.make`. Write `static readonly layer = Layer.effect(this, this.make)` (add `.pipe(Layer.provide(...))` for local wiring), or `(options: Options) => Layer.effect(this, this.make(options))`; move any construction code into make.",
                    ),
                    (
                        297,
                        355,
                        "`Store.layerLive` does not derive from `Store.layer` or `Store.make`. Write it as `Store.layer.pipe(Layer.provide(...))` or `Layer.effect(this, this.make(options))`. A separate implementation belongs in its own service, or in test/ when it is a fake.",
                    ),
                ],
            },
            Case {
                name: "invalid 7: import { Layer as L } from \"effect\";",
                path: "file.ts",
                code: "import { Layer as L } from \"effect\";\nconst live = (L.effect)(Store as any, build);",
                expected: &[(
                    50,
                    81,
                    "This Layer.effect builds `Store` from an inline constructor. Move the constructor into the service class as `make: Effect.gen(...)`, add `static readonly layer = Layer.effect(this, this.make)`, and use `Store.layer` here.",
                )],
            },
        ],
    );
}

/// Edge cases the upstream tests miss, recorded from the upstream plugin, options `[{"allowTagOnly":false}]`
#[test]
fn edges_options_1() {
    check(
        "anti-slop-effect/require-service-make-layer",
        "[{\"allowTagOnly\":false}]",
        &[
            Case {
                name: "invalid 8: import * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "import * as Context from \"effect/Context\";\nimport * as Effect from \"effect/Effect\";\nimport * as Layer from \"effect/Layer\";\nexport class Store extends Context.Service<Store, Shape>()(\"Store\", { make: Effect.gen(function* () { return {}; }) }) {}",
                expected: &[(
                    136,
                    141,
                    "Service \"Store\" has a `make` but no `static layer`. Add `static readonly layer = Layer.effect(this, this.make)`, or `static readonly layer = (options: Options) => Layer.effect(this, this.make(options))` when make takes options.",
                )],
            },
            Case {
                name: "invalid 9: import * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "import * as Context from \"effect/Context\";\nimport * as Effect from \"effect/Effect\";\nimport * as Layer from \"effect/Layer\";\nexport class Store extends Context.Service<Store, Shape>()(\"Store\") {}",
                expected: &[(
                    136,
                    141,
                    "Service \"Store\" has no `make`. Add `make: Effect.gen(function* () { ...yield* dependencies...; return { ...operations } })` to its Context.Service options (or `static readonly make = (options: Options) => Effect.gen(...)` when construction needs plain options), then add `static readonly layer = Layer.effect(this, this.make)`.",
                )],
            },
        ],
    );
}

/// Hand-written: options that upstream's schema rejects (oxlint refuses the
/// whole config, so the oracle cannot record them) are an `Err` here.
#[test]
fn options_are_validated() {
    let factory =
        super::super::RULES.iter().find(|(name, _)| *name == "anti-slop-effect/require-service-make-layer").map(|(_, factory)| *factory).expect("registered");
    let build = |options: &str| factory(&serde_json::from_str::<Vec<serde_json::Value>>(options).expect("valid JSON"));

    assert!(build("[]").is_ok());
    assert!(build("[{}]").is_ok());
    assert!(build(r#"[{"allowTagOnly": false}]"#).is_ok());
    assert!(build(r#"[{"allowTagOnly": "no"}]"#).is_err());
    assert!(build(r#"[{"allowTag": false}]"#).is_err());
    assert!(build(r"[false]").is_err());
    assert!(build(r"[{}, {}]").is_err());
}
