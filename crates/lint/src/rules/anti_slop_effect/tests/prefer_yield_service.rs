//! `anti-slop-effect/prefer-yield-service`: expectations recorded from the upstream plugin
//! (github.com/k-mistele/anti-slop) under oxlint 1.86, one case per upstream
//! RuleTester case in `src/effect/rules/prefer-yield-service.test.ts`.

use super::super::super::anti_slop::tests::{Case, check};

/// src/effect/rules/prefer-yield-service.test.ts
#[test]
fn upstream() {
    check(
        "anti-slop-effect/prefer-yield-service",
        "[]",
        &[
            Case {
                name: "valid 0: \timport * as Effect from \"effect/Effect\";",
                path: "file.ts",
                code: "\n\timport * as Effect from \"effect/Effect\";\n\timport * as Layer from \"effect/Layer\";\n\n\t\tconst make = Effect.gen(function* () {\n\t\t\tconst fs = yield* FileSystem;\n\t\t\treturn { read: (path: string) => fs.read(path) };\n\t\t});",
                expected: &[],
            },
            Case {
                name: "valid 1: \timport * as Effect from \"effect/Effect\";",
                path: "file.ts",
                code: "\n\timport * as Effect from \"effect/Effect\";\n\timport * as Layer from \"effect/Layer\";\n\n\t\tconst make = Effect.gen(function* () {\n\t\t\tconst context = yield* Effect.context<Store>();\n\t\t\treturn { onMessage: (message: Message) => Effect.runForkWith(context)(handle(message)) };\n\t\t});",
                expected: &[],
            },
            Case {
                name: "valid 2: \timport * as Effect from \"effect/Effect\";",
                path: "file.ts",
                code: "\n\timport * as Effect from \"effect/Effect\";\n\timport * as Layer from \"effect/Layer\";\n\n\t\tconst make = Effect.gen(function* () {\n\t\t\tconst services = yield* Effect.context<Store>();\n\t\t\tconst store = Context.get(services, Store);\n\t\t\treturn { store };\n\t\t});",
                expected: &[],
            },
            Case {
                name: "valid 3: \timport * as Effect from \"effect/Effect\";",
                path: "file.ts",
                code: "\n\timport * as Effect from \"effect/Effect\";\n\timport * as Layer from \"effect/Layer\";\n\n\t\tconst make = Effect.gen(function* () {\n\t\t\tconst services = yield* Effect.context<Store>();\n\t\t\treturn { run: work.pipe(Effect.provide(Store.layer)), services };\n\t\t});",
                expected: &[],
            },
            Case {
                name: "valid 4: \t\timport * as Effect from \"other-library\";",
                path: "file.ts",
                code: "\n\t\timport * as Effect from \"other-library\";\n\t\tconst make = Effect.gen(function* () {\n\t\t\tconst services = yield* Effect.context();\n\t\t\treturn work.pipe(Effect.provide(services));\n\t\t});",
                expected: &[],
            },
            Case {
                name: "invalid 5: \timport * as Effect from \"effect/Effect\";",
                path: "file.ts",
                code: "\n\timport * as Effect from \"effect/Effect\";\n\timport * as Layer from \"effect/Layer\";\n\n\t\t\tconst make = Effect.gen(function* () {\n\t\t\t\tconst fileAccess = yield* Effect.context<FileSystem | Path>();\n\t\t\t\treturn { build: (dir: string) => runBuild(dir).pipe(Effect.provideContext(fileAccess)) };\n\t\t\t});",
                expected: &[(
                    136,
                    191,
                    "Do not capture the context with `Effect.context` and provide it back with Effect.provideContext. `yield*` each service the operation uses (`const store = yield* Store`) and let the nested effect's requirements flow through its R channel to the layer or runtime that provides them.",
                )],
            },
            Case {
                name: "invalid 6: \timport * as Effect from \"effect/Effect\";",
                path: "file.ts",
                code: "\n\timport * as Effect from \"effect/Effect\";\n\timport * as Layer from \"effect/Layer\";\n\n\t\t\tconst make = Effect.gen(function* () {\n\t\t\t\tconst services = yield* Effect.context<Store | Crypto>();\n\t\t\t\treturn { process: Effect.provide(process, services) };\n\t\t\t});",
                expected: &[(
                    136,
                    186,
                    "Do not capture the context with `Effect.context` and provide it back with Effect.provide. `yield*` each service the operation uses (`const store = yield* Store`) and let the nested effect's requirements flow through its R channel to the layer or runtime that provides them.",
                )],
            },
            Case {
                name: "invalid 7: \t\t\timport { Effect, Layer } from \"effect\";",
                path: "file.ts",
                code: "\n\t\t\timport { Effect, Layer } from \"effect\";\n\t\t\tconst layer = Layer.unwrap(Effect.gen(function* () {\n\t\t\t\tconst context = yield* Effect.context<Store>();\n\t\t\t\treturn Live.pipe(Layer.provide(context as never));\n\t\t\t}));",
                expected: &[(
                    110,
                    150,
                    "Do not capture the context with `Effect.context` and provide it back with Layer.provide. `yield*` each service the operation uses (`const store = yield* Store`) and let the nested effect's requirements flow through its R channel to the layer or runtime that provides them.",
                )],
            },
            Case {
                name: "invalid 8: \timport * as Effect from \"effect/Effect\";",
                path: "file.ts",
                code: "\n\timport * as Effect from \"effect/Effect\";\n\timport * as Layer from \"effect/Layer\";\n\n\t\t\tconst layer = Layer.unwrap(Effect.gen(function* () {\n\t\t\t\tconst context = yield* Effect.context<Store>();\n\t\t\t\treturn Live.pipe(Layer.provide(Layer.succeedContext(context)));\n\t\t\t}));",
                expected: &[(
                    150,
                    190,
                    "Do not capture the context with `Effect.context` and provide it back with Layer.succeedContext. `yield*` each service the operation uses (`const store = yield* Store`) and let the nested effect's requirements flow through its R channel to the layer or runtime that provides them.",
                )],
            },
        ],
    );
}

/// Edge cases the upstream tests miss, recorded from the upstream plugin.
#[test]
fn edges() {
    check(
        "anti-slop-effect/prefer-yield-service",
        "[]",
        &[
            Case {
                name: "valid 0: import * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "import * as Context from \"effect/Context\";\nimport * as Effect from \"effect/Effect\";\nimport * as Layer from \"effect/Layer\";\nconst make = Effect.gen(function* () { const services = yield* Effect.context(); return work.pipe(Effect.provide(other)); });",
                expected: &[],
            },
            Case {
                name: "valid 1: import * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "import * as Context from \"effect/Context\";\nimport * as Effect from \"effect/Effect\";\nimport * as Layer from \"effect/Layer\";\nconst make = Effect.gen(function* () { let services = yield* Effect.context(); { const services = other; work.pipe(Effect.provide(services)); } });",
                expected: &[],
            },
            Case {
                name: "invalid 2: import { Effect as E } from \"effect\";",
                path: "file.ts",
                code: "import { Effect as E } from \"effect\";\nconst make = E.gen(function* () { const services = (yield* E.context<Store>()) as Context; return work.pipe((E.provide)((services))); });",
                expected: &[(
                    78,
                    127,
                    "Do not capture the context with `Effect.context` and provide it back with E.provide. `yield*` each service the operation uses (`const store = yield* Store`) and let the nested effect's requirements flow through its R channel to the layer or runtime that provides them.",
                )],
            },
            Case {
                name: "invalid 3: import * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "import * as Context from \"effect/Context\";\nimport * as Effect from \"effect/Effect\";\nimport * as Layer from \"effect/Layer\";\nconst make = Effect.gen(function* () { const services = yield* Effect.context(); a.pipe(Effect.provide(services)); b.pipe(Effect.provideContext(services)); });",
                expected: &[(
                    168,
                    202,
                    "Do not capture the context with `Effect.context` and provide it back with Effect.provide. `yield*` each service the operation uses (`const store = yield* Store`) and let the nested effect's requirements flow through its R channel to the layer or runtime that provides them.",
                )],
            },
        ],
    );
}
