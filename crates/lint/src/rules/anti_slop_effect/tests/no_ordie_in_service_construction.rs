//! `anti-slop-effect/no-ordie-in-service-construction`: expectations recorded from the upstream plugin
//! (github.com/k-mistele/anti-slop) under oxlint 1.86, one case per upstream
//! RuleTester case in `src/effect/rules/no-ordie-in-service-construction.test.ts`.

use super::super::super::anti_slop::tests::{Case, check};

/// src/effect/rules/no-ordie-in-service-construction.test.ts
#[test]
fn upstream() {
    check(
        "anti-slop-effect/no-ordie-in-service-construction",
        "[]",
        &[
            Case {
                name: "valid 0: \timport * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "\n\timport * as Context from \"effect/Context\";\n\timport * as Effect from \"effect/Effect\";\n\timport * as Layer from \"effect/Layer\";\n\n\t\texport class Store extends Context.Service<Store, StoreShape>()(\"@app/Store\", {\n\t\t\tmake: Effect.gen(function* () {\n\t\t\t\tconst sql = yield* Sql;\n\t\t\t\treturn {\n\t\t\t\t\tfind: (id: string) => Effect.gen(function* () {\n\t\t\t\t\t\treturn yield* sql.find(id).pipe(Effect.orDie);\n\t\t\t\t\t}),\n\t\t\t\t\tlist: Effect.orDie(sql.list),\n\t\t\t\t};\n\t\t\t}),\n\t\t}) {\n\t\t\tstatic readonly layer = Layer.effect(this, this.make);\n\t\t}",
                expected: &[],
            },
            Case {
                name: "valid 1: \timport * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "\n\timport * as Context from \"effect/Context\";\n\timport * as Effect from \"effect/Effect\";\n\timport * as Layer from \"effect/Layer\";\n\n\t\texport class Store extends Context.Service<Store, StoreShape>()(\"@app/Store\", {\n\t\t\tmake: Effect.gen(function* () {\n\t\t\t\tconst find = Effect.fn(\"Store.find\")(function* (id: string) {\n\t\t\t\t\treturn yield* lookup(id).pipe(Effect.orDie);\n\t\t\t\t});\n\t\t\t\tconst unwrap = Effect.fnUntraced(function* (id: string) { return id; }, Effect.orDie);\n\t\t\t\tconst collect = Stream.runCollect(entries).pipe(Effect.orDie);\n\t\t\t\treturn { find, unwrap, collect };\n\t\t\t}),\n\t\t}) {\n\t\t\tstatic readonly layer = Layer.effect(this, this.make);\n\t\t}",
                expected: &[],
            },
            Case {
                name: "valid 2: \timport * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "\n\timport * as Context from \"effect/Context\";\n\timport * as Effect from \"effect/Effect\";\n\timport * as Layer from \"effect/Layer\";\n\n\t\tconst program = Effect.gen(function* () {\n\t\t\tyield* risky.pipe(Effect.orDie);\n\t\t});",
                expected: &[],
            },
            Case {
                name: "valid 3: \timport * as Context from \"effect/Context\";",
                path: "store.test.ts",
                code: "\n\timport * as Context from \"effect/Context\";\n\timport * as Effect from \"effect/Effect\";\n\timport * as Layer from \"effect/Layer\";\n\n\t\t\tconst layer = Store.layer.pipe(Layer.orDie);",
                expected: &[],
            },
            Case {
                name: "valid 4: \t\timport * as Layer from \"other-library\";",
                path: "file.ts",
                code: "\n\t\timport * as Layer from \"other-library\";\n\t\tconst layer = live.pipe(Layer.orDie);",
                expected: &[],
            },
            Case {
                name: "invalid 5: \timport * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "\n\timport * as Context from \"effect/Context\";\n\timport * as Effect from \"effect/Effect\";\n\timport * as Layer from \"effect/Layer\";\n\n\t\t\texport class Client extends Context.Service<Client, ClientShape>()(\"@app/Client\", {\n\t\t\t\tmake: Effect.gen(function* () {\n\t\t\t\t\tconst token = yield* Config.string(\"TOKEN\").pipe(Effect.orDie);\n\t\t\t\t\treturn { token };\n\t\t\t\t}),\n\t\t\t}) {\n\t\t\t\tstatic readonly layer = Layer.effect(this, this.make);\n\t\t\t}",
                expected: &[(
                    305,
                    317,
                    "`Effect.orDie` turns a service construction failure into a defect. Classify the failure as a Schema.TaggedError and let it flow through the layer's error channel (`Layer.Layer<Service, ThatError>`), or handle it here with Effect.catchTag.",
                )],
            },
            Case {
                name: "invalid 6: \timport * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "\n\timport * as Context from \"effect/Context\";\n\timport * as Effect from \"effect/Effect\";\n\timport * as Layer from \"effect/Layer\";\n\n\t\t\texport class Client extends Context.Service<Client, ClientShape>()(\"@app/Client\", {\n\t\t\t\tmake: Effect.gen(function* () {\n\t\t\t\t\treturn {};\n\t\t\t\t}).pipe(Effect.orDieWith((error) => error)),\n\t\t\t}) {\n\t\t\t\tstatic readonly layer = Layer.effect(this, this.make);\n\t\t\t}",
                expected: &[(
                    279,
                    295,
                    "`Effect.orDieWith` turns a service construction failure into a defect. Classify the failure as a Schema.TaggedError and let it flow through the layer's error channel (`Layer.Layer<Service, ThatError>`), or handle it here with Effect.catchTag.",
                )],
            },
            Case {
                name: "invalid 7: \timport * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "\n\timport * as Context from \"effect/Context\";\n\timport * as Effect from \"effect/Effect\";\n\timport * as Layer from \"effect/Layer\";\n\n\t\t\texport class Database extends Context.Service<Database, DatabaseShape>()(\"@app/Database\") {\n\t\t\t\tstatic readonly make = (file: string) => Effect.gen(function* () {\n\t\t\t\t\tyield* migrate(file).pipe(Effect.orDie);\n\t\t\t\t\treturn {};\n\t\t\t\t});\n\t\t\t\tstatic readonly layer = (file: string) => Layer.effect(this, this.make(file));\n\t\t\t}",
                expected: &[(
                    325,
                    337,
                    "`Effect.orDie` turns a service construction failure into a defect. Classify the failure as a Schema.TaggedError and let it flow through the layer's error channel (`Layer.Layer<Service, ThatError>`), or handle it here with Effect.catchTag.",
                )],
            },
            Case {
                name: "invalid 8: \timport * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "\n\timport * as Context from \"effect/Context\";\n\timport * as Effect from \"effect/Effect\";\n\timport * as Layer from \"effect/Layer\";\n\n\t\t\texport const live = Layer.effect(Store, Effect.gen(function* () {\n\t\t\t\tconst key = yield* Effect.suspend(() => loadKey.pipe(Effect.orDie));\n\t\t\t\treturn { key };\n\t\t\t}));",
                expected: &[(
                    254,
                    266,
                    "`Effect.orDie` turns a service construction failure into a defect. Classify the failure as a Schema.TaggedError and let it flow through the layer's error channel (`Layer.Layer<Service, ThatError>`), or handle it here with Effect.catchTag.",
                )],
            },
            Case {
                name: "invalid 9: \timport * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "\n\timport * as Context from \"effect/Context\";\n\timport * as Effect from \"effect/Effect\";\n\timport * as Layer from \"effect/Layer\";\n\n\t\t\tconst logLayer = layerJsonl(path).pipe(Layer.orDie);",
                expected: &[(
                    170,
                    181,
                    "`Layer.orDie` hides this layer's construction failures as defects. Leave them in the layer's error channel as Schema.TaggedErrors and handle them where the runtime is built.",
                )],
            },
        ],
    );
}

/// Edge cases the upstream tests miss, recorded from the upstream plugin.
#[test]
fn edges() {
    check(
        "anti-slop-effect/no-ordie-in-service-construction",
        "[]",
        &[
            Case {
                name: "valid 0: import { Layer } from \"other\";",
                path: "file.ts",
                code: "import { Layer } from \"other\";\nconst layer = live.pipe(Layer.orDie);",
                expected: &[],
            },
            Case {
                name: "valid 1: import * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "import * as Context from \"effect/Context\";\nimport * as Effect from \"effect/Effect\";\nimport * as Layer from \"effect/Layer\";\nconst program = Effect.gen(function* () { yield* risky.pipe(Effect.orDieWith(f)); });",
                expected: &[],
            },
            Case {
                name: "valid 2: import * as Context from \"effect/Context\";",
                path: "store.spec.ts",
                code: "import * as Context from \"effect/Context\";\nimport * as Effect from \"effect/Effect\";\nimport * as Layer from \"effect/Layer\";\nconst layer = live.pipe(Layer.orDie);",
                expected: &[],
            },
            Case {
                name: "invalid 3: import { Layer as L } from \"effect\";",
                path: "file.ts",
                code: "import { Layer as L } from \"effect\";\nconst layer = live.pipe((L.orDie));",
                expected: &[(
                    62,
                    69,
                    "`Layer.orDie` hides this layer's construction failures as defects. Leave them in the layer's error channel as Schema.TaggedErrors and handle them where the runtime is built.",
                )],
            },
            Case {
                name: "invalid 4: import * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "import * as Context from \"effect/Context\";\nimport * as Effect from \"effect/Effect\";\nimport * as Layer from \"effect/Layer\";\nexport class Store extends Context.Service<Store, Shape>()(\"Store\", {\n\tmake: Effect.gen(function* () {\n\t\tconst sql = yield* (Sql.pipe(Effect.orDie));\n\t\treturn { find: () => sql };\n\t}),\n}) {\n\tstatic readonly layer = Layer.effect(this, Effect.gen(function* () { return yield* load.pipe(Effect.orDieWith(f)); }));\n}",
                expected: &[
                    (
                        257,
                        269,
                        "`Effect.orDie` turns a service construction failure into a defect. Classify the failure as a Schema.TaggedError and let it flow through the layer's error channel (`Layer.Layer<Service, ThatError>`), or handle it here with Effect.catchTag.",
                    ),
                    (
                        407,
                        423,
                        "`Effect.orDieWith` turns a service construction failure into a defect. Classify the failure as a Schema.TaggedError and let it flow through the layer's error channel (`Layer.Layer<Service, ThatError>`), or handle it here with Effect.catchTag.",
                    ),
                ],
            },
        ],
    );
}
