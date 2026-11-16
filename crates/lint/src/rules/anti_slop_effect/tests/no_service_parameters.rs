//! `anti-slop-effect/no-service-parameters`: expectations recorded from the upstream plugin
//! (github.com/k-mistele/anti-slop) under oxlint 1.86, one case per upstream
//! RuleTester case in `src/effect/rules/no-service-parameters.test.ts`.

use super::super::super::anti_slop::tests::{Case, check};

/// src/effect/rules/no-service-parameters.test.ts
#[test]
fn upstream() {
    check(
        "anti-slop-effect/no-service-parameters",
        "[]",
        &[
            Case {
                name: "valid 0: \timport * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "\n\timport * as Context from \"effect/Context\";\n\timport * as Effect from \"effect/Effect\";\n\timport * as Layer from \"effect/Layer\";\n\timport * as ManagedRuntime from \"effect/ManagedRuntime\";\n\n\t\tconst find = (input: { readonly id: string }) => Effect.gen(function* () {\n\t\t\tconst store = yield* Store;\n\t\t\treturn yield* store.find(input.id);\n\t\t});",
                expected: &[],
            },
            Case {
                name: "valid 1: \timport * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "\n\timport * as Context from \"effect/Context\";\n\timport * as Effect from \"effect/Effect\";\n\timport * as Layer from \"effect/Layer\";\n\timport * as ManagedRuntime from \"effect/ManagedRuntime\";\n\n\t\tconst withLogging = <A>(layer: Layer.Layer<A>) => layer.pipe(Layer.provide(Logging.layer));\n\t\tconst merged = (left: Layer.Layer<A>, right: Layer.Layer<B>) => Layer.merge(left, right);\n\t\tfunction provideAll(layer: Layer.Layer<A>): Layer.Layer<A> {\n\t\t\treturn layer;\n\t\t}",
                expected: &[],
            },
            Case {
                name: "valid 2: \timport * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "\n\timport * as Context from \"effect/Context\";\n\timport * as Effect from \"effect/Effect\";\n\timport * as Layer from \"effect/Layer\";\n\timport * as ManagedRuntime from \"effect/ManagedRuntime\";\n\n\t\tconst config = Effect.contextWith((context: Context.Context<never>) => Effect.succeed(context));",
                expected: &[],
            },
            Case {
                name: "valid 3: \timport * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "\n\timport * as Context from \"effect/Context\";\n\timport * as Effect from \"effect/Effect\";\n\timport * as Layer from \"effect/Layer\";\n\timport * as ManagedRuntime from \"effect/ManagedRuntime\";\n\n\t\timport { Store } from \"./store.ts\";\n\t\tconst save = (store: Store) => store;",
                expected: &[],
            },
            Case {
                name: "valid 4: \t\timport * as Context from \"other-library\";",
                path: "file.ts",
                code: "\n\t\timport * as Context from \"other-library\";\n\t\tconst run = (context: Context.Context) => context;",
                expected: &[],
            },
            Case {
                name: "valid 5: \timport * as Context from \"effect/Context\";",
                path: "store.test.ts",
                code: "\n\timport * as Context from \"effect/Context\";\n\timport * as Effect from \"effect/Effect\";\n\timport * as Layer from \"effect/Layer\";\n\timport * as ManagedRuntime from \"effect/ManagedRuntime\";\n\n\t\t\tconst run = (context: Context.Context<Store>) => context;",
                expected: &[],
            },
            Case {
                name: "invalid 6: \timport * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "\n\timport * as Context from \"effect/Context\";\n\timport * as Effect from \"effect/Effect\";\n\timport * as Layer from \"effect/Layer\";\n\timport * as ManagedRuntime from \"effect/ManagedRuntime\";\n\n\t\t\tconst runAllow = <R>(allow: Allow<R>, context: Context.Context<R>) => Effect.runPromiseWith(context)(allow);",
                expected: &[(
                    227,
                    254,
                    "Do not pass a `Context.Context` as a parameter. `yield*` the services the operation needs inside it (`const store = yield* Store`) and let its requirements flow through R to the runtime that provides them; pass only plain values.",
                )],
            },
            Case {
                name: "invalid 7: \timport * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "\n\timport * as Context from \"effect/Context\";\n\timport * as Effect from \"effect/Effect\";\n\timport * as Layer from \"effect/Layer\";\n\timport * as ManagedRuntime from \"effect/ManagedRuntime\";\n\n\t\t\tfunction handle(runtime: ManagedRuntime.ManagedRuntime<Store, never>, request: Request) {\n\t\t\t\treturn runtime.runPromise(serve(request));\n\t\t\t}",
                expected: &[(
                    205,
                    257,
                    "Do not pass a `ManagedRuntime.ManagedRuntime` as a parameter. `yield*` the services the operation needs inside it (`const store = yield* Store`) and let its requirements flow through R to the runtime that provides them; pass only plain values.",
                )],
            },
            Case {
                name: "invalid 8: \timport * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "\n\timport * as Context from \"effect/Context\";\n\timport * as Effect from \"effect/Effect\";\n\timport * as Layer from \"effect/Layer\";\n\timport * as ManagedRuntime from \"effect/ManagedRuntime\";\n\n\t\t\tconst operation = (input: Input, layer: Layer.Layer<Store>) => run(input).pipe(Effect.provide(layer));",
                expected: &[(
                    222,
                    247,
                    "Do not pass a `Layer.Layer` as a parameter. `yield*` the services the operation needs inside it (`const store = yield* Store`) and let its requirements flow through R to the runtime that provides them; pass only plain values.",
                )],
            },
            Case {
                name: "invalid 9: \timport * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "\n\timport * as Context from \"effect/Context\";\n\timport * as Effect from \"effect/Effect\";\n\timport * as Layer from \"effect/Layer\";\n\timport * as ManagedRuntime from \"effect/ManagedRuntime\";\n\n\t\t\texport class Store extends Context.Service<Store, StoreShape>()(\"@app/Store\") {}\n\t\t\tconst save = (store: Store, value: string) => store.save(value);\n\t\t\tclass Worker { run(store: Store) {} }",
                expected: &[
                    (
                        287,
                        299,
                        "Do not pass the `Store` service as a parameter. `yield* Store` inside the operation and let the requirement flow through R to the layer that provides it; pass only plain values.",
                    ),
                    (
                        360,
                        372,
                        "Do not pass the `Store` service as a parameter. `yield* Store` inside the operation and let the requirement flow through R to the layer that provides it; pass only plain values.",
                    ),
                ],
            },
            Case {
                name: "invalid 10: \timport * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "\n\timport * as Context from \"effect/Context\";\n\timport * as Effect from \"effect/Effect\";\n\timport * as Layer from \"effect/Layer\";\n\timport * as ManagedRuntime from \"effect/ManagedRuntime\";\n\n\t\t\tinterface Operations {\n\t\t\t\treadonly run: (context: Context.Context<Store>) => Effect.Effect<void>;\n\t\t\t\texecute(runtime: ManagedRuntime.ManagedRuntime<Store, never>): void;\n\t\t\t}",
                expected: &[
                    (
                        231,
                        262,
                        "Do not pass a `Context.Context` as a parameter. `yield*` the services the operation needs inside it (`const store = yield* Store`) and let its requirements flow through R to the runtime that provides them; pass only plain values.",
                    ),
                    (
                        300,
                        352,
                        "Do not pass a `ManagedRuntime.ManagedRuntime` as a parameter. `yield*` the services the operation needs inside it (`const store = yield* Store`) and let its requirements flow through R to the runtime that provides them; pass only plain values.",
                    ),
                ],
            },
        ],
    );
}

/// Edge cases the upstream tests miss, recorded from the upstream plugin.
#[test]
fn edges() {
    check(
        "anti-slop-effect/no-service-parameters",
        "[]",
        &[
            Case {
                name: "valid 0: import { Context as C } from \"other\";",
                path: "file.ts",
                code: "import { Context as C } from \"other\";\nconst run = (context: C.Context<Store>) => context;",
                expected: &[],
            },
            Case {
                name: "valid 1: import * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "import * as Context from \"effect/Context\";\nimport * as Effect from \"effect/Effect\";\nimport * as Layer from \"effect/Layer\";\nconst run = (context: Context.Tag<Store>) => context;",
                expected: &[],
            },
            Case {
                name: "valid 2: import * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "import * as Context from \"effect/Context\";\nimport * as Effect from \"effect/Effect\";\nimport * as Layer from \"effect/Layer\";\nconst build = (layer: Layer.Layer<Store>): Layer.Layer<Store> => layer;",
                expected: &[],
            },
            Case {
                name: "valid 3: import * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "import * as Context from \"effect/Context\";\nimport * as Effect from \"effect/Effect\";\nimport * as Layer from \"effect/Layer\";\nconst build = (layer: Layer.Layer<Store>) => Layer.provide(layer, other);",
                expected: &[],
            },
            Case {
                name: "valid 4: import * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "import * as Context from \"effect/Context\";\nimport * as Effect from \"effect/Effect\";\nimport * as Layer from \"effect/Layer\";\nconst program = Effect.provideServiceEffect(Store, (context: Context.Context<Store>) => context);",
                expected: &[],
            },
            Case {
                name: "invalid 5: import { Context as C, Layer } from \"effect\";",
                path: "file.ts",
                code: "import { Context as C, Layer } from \"effect\";\nconst run = (context: C.Context<Store>, layer?: Layer.Layer<Store>) => run(context, layer);",
                expected: &[
                    (
                        59,
                        84,
                        "Do not pass a `C.Context` as a parameter. `yield*` the services the operation needs inside it (`const store = yield* Store`) and let its requirements flow through R to the runtime that provides them; pass only plain values.",
                    ),
                    (
                        86,
                        112,
                        "Do not pass a `Layer.Layer` as a parameter. `yield*` the services the operation needs inside it (`const store = yield* Store`) and let its requirements flow through R to the runtime that provides them; pass only plain values.",
                    ),
                ],
            },
            Case {
                name: "invalid 6: import * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "import * as Context from \"effect/Context\";\nimport * as Effect from \"effect/Effect\";\nimport * as Layer from \"effect/Layer\";\ntype Run = (context: Context.Context<Store>) => void;\ninterface Api { run(context: Context.Context<Store>): void }",
                expected: &[
                    (
                        135,
                        166,
                        "Do not pass a `Context.Context` as a parameter. `yield*` the services the operation needs inside it (`const store = yield* Store`) and let its requirements flow through R to the runtime that provides them; pass only plain values.",
                    ),
                    (
                        197,
                        228,
                        "Do not pass a `Context.Context` as a parameter. `yield*` the services the operation needs inside it (`const store = yield* Store`) and let its requirements flow through R to the runtime that provides them; pass only plain values.",
                    ),
                ],
            },
            Case {
                name: "invalid 7: import * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "import * as Context from \"effect/Context\";\nimport * as Effect from \"effect/Effect\";\nimport * as Layer from \"effect/Layer\";\nclass Store extends Context.Service<Store, Shape>()(\"Store\", {}) {}\nconst save = (store: Store, { id }: Input = {}) => store;\nfunction load(this: Window, store: Store) {}",
                expected: &[
                    (
                        205,
                        217,
                        "Do not pass the `Store` service as a parameter. `yield* Store` inside the operation and let the requirement flow through R to the layer that provides it; pass only plain values.",
                    ),
                    (
                        277,
                        289,
                        "Do not pass the `Store` service as a parameter. `yield* Store` inside the operation and let the requirement flow through R to the layer that provides it; pass only plain values.",
                    ),
                ],
            },
            Case {
                name: "invalid 8: import * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "import * as Context from \"effect/Context\";\nimport * as Effect from \"effect/Effect\";\nimport * as Layer from \"effect/Layer\";\nclass Api { constructor(private readonly context: Context.Context<Store> = initial) {} }",
                expected: &[(
                    147,
                    205,
                    "Do not pass a `Context.Context` as a parameter. `yield*` the services the operation needs inside it (`const store = yield* Store`) and let its requirements flow through R to the runtime that provides them; pass only plain values.",
                )],
            },
        ],
    );
}
