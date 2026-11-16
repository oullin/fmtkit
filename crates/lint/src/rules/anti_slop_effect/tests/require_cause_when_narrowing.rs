//! `anti-slop-effect/require-cause-when-narrowing`: expectations recorded from the upstream plugin
//! (github.com/k-mistele/anti-slop) under oxlint 1.86, one case per upstream
//! RuleTester case in `src/effect/rules/require-cause-when-narrowing.test.ts`.

use super::super::super::anti_slop::tests::{Case, check};

/// src/effect/rules/require-cause-when-narrowing.test.ts
#[test]
fn upstream() {
    check(
        "anti-slop-effect/require-cause-when-narrowing",
        "[]",
        &[
            Case {
                name: "valid 0: \timport * as Effect from \"effect/Effect\";",
                path: "file.ts",
                code: "\n\timport * as Effect from \"effect/Effect\";\n\n\t\tconst a = decode(text).pipe(Effect.mapError((cause) => BadRequest.make({ reason: \"Invalid JSON\", cause })));\n\t\tconst b = decode(text).pipe(Effect.mapError((error) => new BadRequest({ reason: error.message })));\n\t\tconst c = decode(text).pipe(Effect.mapError(() => BadRequest.make({ reason: \"Invalid JSON\", ...details })));",
                expected: &[],
            },
            Case {
                name: "valid 1: \timport * as Effect from \"effect/Effect\";",
                path: "file.ts",
                code: "\n\timport * as Effect from \"effect/Effect\";\n\n\t\tconst a = decode(text).pipe(Effect.tapError(Effect.logError), Effect.mapError(() => BadRequest.make({ reason: \"Invalid JSON\" })));\n\t\tconst b = Effect.fn(\"decode\")(function* () {}, Effect.tapCause(Effect.logError), Effect.catch(() => Effect.fail(new Failed())));\n\t\tconst c = Effect.mapError(Effect.tapErrorCause(decode(text), Effect.logError), () => new Failed());",
                expected: &[],
            },
            Case {
                name: "valid 2: \timport * as Effect from \"effect/Effect\";",
                path: "file.ts",
                code: "\n\timport * as Effect from \"effect/Effect\";\n\n\t\tconst a = load.pipe(Effect.catchTag(\"NotFound\", () => Effect.fail(new Missing())));\n\t\tconst b = load.pipe(Effect.catchTags({ NotFound: () => Effect.fail(new Missing()) }));",
                expected: &[],
            },
            Case {
                name: "valid 3: \timport * as Effect from \"effect/Effect\";",
                path: "file.ts",
                code: "\n\timport * as Effect from \"effect/Effect\";\n\n\t\tconst a = load.pipe(Effect.orElse(() => Effect.succeed(new Fallback())));\n\t\tconst b = load.pipe(Effect.catch(() => Effect.sync(() => new Fallback())));\n\t\tconst c = load.pipe(Effect.catch(() => Effect.succeed(Option.make(1))));",
                expected: &[],
            },
            Case {
                name: "valid 4: \t\timport * as Effect from \"other-library\";",
                path: "file.ts",
                code: "\n\t\timport * as Effect from \"other-library\";\n\t\tconst a = load.pipe(Effect.mapError(() => new Failed()));",
                expected: &[],
            },
            Case {
                name: "invalid 5: \timport * as Effect from \"effect/Effect\";",
                path: "file.ts",
                code: "\n\timport * as Effect from \"effect/Effect\";\n\n\t\t\tconst a = decode(text).pipe(Effect.mapError(() => BadRequest.make({ reason: \"Invalid JSON\" })));",
                expected: &[(
                    97,
                    140,
                    "This Effect.mapError handler discards the original error and constructs a new one without it. Name the handler's parameter and pass it as `cause` (`(error) => new BadRequest({ ..., cause: error })`), or log it first with `Effect.tapError(Effect.logError)` before narrowing.",
                )],
            },
            Case {
                name: "invalid 6: \timport * as Effect from \"effect/Effect\";",
                path: "file.ts",
                code: "\n\timport * as Effect from \"effect/Effect\";\n\n\t\t\tconst a = Effect.mapError(decode(text), (_error) => new McpDialFailed({ message: \"bad frame\" }));",
                expected: &[(
                    99,
                    142,
                    "This Effect.mapError handler discards the original error and constructs a new one without it. Name the handler's parameter and pass it as `cause` (`(error) => new McpDialFailed({ ..., cause: error })`), or log it first with `Effect.tapError(Effect.logError)` before narrowing.",
                )],
            },
            Case {
                name: "invalid 7: \timport * as Effect from \"effect/Effect\";",
                path: "file.ts",
                code: "\n\timport * as Effect from \"effect/Effect\";\n\n\t\t\tconst a = read.pipe(Effect.catch(() => AuthUnavailable.make({ error: \"unavailable\" })));\n\t\t\tconst b = read.pipe(Effect.catchAll(() => Effect.fail(new Unavailable())));\n\t\t\tconst c = read.pipe(Effect.orElse(() => Effect.gen(function* () {\n\t\t\t\treturn yield* new Unavailable({ reason: \"down\" });\n\t\t\t})));\n\t\t\tconst d = read.pipe(Effect.catchCause(() => { return Effect.fail(Errors.Unavailable.make()); }));",
                expected: &[
                    (
                        86,
                        132,
                        "This Effect.catch handler discards the original error and constructs a new one without it. Name the handler's parameter and pass it as `cause` (`(error) => new AuthUnavailable({ ..., cause: error })`), or log it first with `Effect.tapError(Effect.logError)` before narrowing.",
                    ),
                    (
                        193,
                        210,
                        "This Effect.catchAll handler discards the original error and constructs a new one without it. Name the handler's parameter and pass it as `cause` (`(error) => new Unavailable({ ..., cause: error })`), or log it first with `Effect.tapError(Effect.logError)` before narrowing.",
                    ),
                    (
                        302,
                        337,
                        "This Effect.orElse handler discards the original error and constructs a new one without it. Name the handler's parameter and pass it as `cause` (`(error) => new Unavailable({ ..., cause: error })`), or log it first with `Effect.tapError(Effect.logError)` before narrowing.",
                    ),
                    (
                        416,
                        441,
                        "This Effect.catchCause handler discards the original error and constructs a new one without it. Name the handler's parameter and pass it as `cause` (`(error) => new Errors.Unavailable({ ..., cause: error })`), or log it first with `Effect.tapError(Effect.logError)` before narrowing.",
                    ),
                ],
            },
            Case {
                name: "invalid 8: \timport * as Effect from \"effect/Effect\";",
                path: "file.ts",
                code: "\n\timport * as Effect from \"effect/Effect\";\n\n\t\t\tconst a = read.pipe(Effect.tapError(Effect.logError)).pipe(Effect.map(f), Effect.mapError(() => new Failed()));\n\t\t\tconst b = read.pipe(Effect.mapError(() => new Failed()), Effect.tapError(Effect.logError));",
                expected: &[(
                    204,
                    216,
                    "This Effect.mapError handler discards the original error and constructs a new one without it. Name the handler's parameter and pass it as `cause` (`(error) => new Failed({ ..., cause: error })`), or log it first with `Effect.tapError(Effect.logError)` before narrowing.",
                )],
            },
        ],
    );
}

/// Edge cases the upstream tests miss, recorded from the upstream plugin.
#[test]
fn edges() {
    check(
        "anti-slop-effect/require-cause-when-narrowing",
        "[]",
        &[
            Case {
                name: "valid 0: import * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "import * as Context from \"effect/Context\";\nimport * as Effect from \"effect/Effect\";\nimport * as Layer from \"effect/Layer\";\nconst a = load.pipe(Effect.mapError((...args) => new Failed()));",
                expected: &[],
            },
            Case {
                name: "valid 1: import * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "import * as Context from \"effect/Context\";\nimport * as Effect from \"effect/Effect\";\nimport * as Layer from \"effect/Layer\";\nconst a = load.pipe(Effect.mapError(function (this: Ctx) { return new Failed(); }));",
                expected: &[(
                    189,
                    201,
                    "This Effect.mapError handler discards the original error and constructs a new one without it. Name the handler's parameter and pass it as `cause` (`(error) => new Failed({ ..., cause: error })`), or log it first with `Effect.tapError(Effect.logError)` before narrowing.",
                )],
            },
            Case {
                name: "valid 2: import * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "import * as Context from \"effect/Context\";\nimport * as Effect from \"effect/Effect\";\nimport * as Layer from \"effect/Layer\";\nconst a = load.pipe(Effect.mapError(handler));",
                expected: &[],
            },
            Case {
                name: "valid 3: import * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "import * as Context from \"effect/Context\";\nimport * as Effect from \"effect/Effect\";\nimport * as Layer from \"effect/Layer\";\nconst a = load.pipe(Effect.tap(log), Effect.mapError(() => new Failed()));",
                expected: &[(
                    182,
                    194,
                    "This Effect.mapError handler discards the original error and constructs a new one without it. Name the handler's parameter and pass it as `cause` (`(error) => new Failed({ ..., cause: error })`), or log it first with `Effect.tapError(Effect.logError)` before narrowing.",
                )],
            },
            Case {
                name: "invalid 4: import { Effect as E } from \"effect\";",
                path: "file.ts",
                code: "import { Effect as E } from \"effect\";\nconst a = load.pipe((E.mapError)((() => (new Failed()))));",
                expected: &[(
                    79,
                    91,
                    "This Effect.mapError handler discards the original error and constructs a new one without it. Name the handler's parameter and pass it as `cause` (`(error) => new Failed({ ..., cause: error })`), or log it first with `Effect.tapError(Effect.logError)` before narrowing.",
                )],
            },
            Case {
                name: "invalid 5: import * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "import * as Context from \"effect/Context\";\nimport * as Effect from \"effect/Effect\";\nimport * as Layer from \"effect/Layer\";\nconst a = load.pipe(Effect.mapError((error) => Errors.Failed.make({ message: \"x\" })));",
                expected: &[(
                    170,
                    206,
                    "This Effect.mapError handler discards the original error and constructs a new one without it. Name the handler's parameter and pass it as `cause` (`(error) => new Errors.Failed({ ..., cause: error })`), or log it first with `Effect.tapError(Effect.logError)` before narrowing.",
                )],
            },
            Case {
                name: "invalid 6: import * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "import * as Context from \"effect/Context\";\nimport * as Effect from \"effect/Effect\";\nimport * as Layer from \"effect/Layer\";\nconst a = load.pipe(Effect.catch((_) => Effect.gen(function* () { const helper = () => new Inner(); return yield* Effect.fail(new Outer()); })));",
                expected: &[(
                    249,
                    260,
                    "This Effect.catch handler discards the original error and constructs a new one without it. Name the handler's parameter and pass it as `cause` (`(error) => new Outer({ ..., cause: error })`), or log it first with `Effect.tapError(Effect.logError)` before narrowing.",
                )],
            },
            Case {
                name: "invalid 7: import * as Context from \"effect/Context\";",
                path: "file.ts",
                code: "import * as Context from \"effect/Context\";\nimport * as Effect from \"effect/Effect\";\nimport * as Layer from \"effect/Layer\";\nconst a = Effect.mapError(load, () => cond ? new A() : new B({ cause: 1 }));",
                expected: &[],
            },
            Case {
                name: "this-parameter invalid 1: import * as Effect from \"effect/Effect\";",
                path: "file.ts",
                code: "import * as Effect from \"effect/Effect\";\nconst a = load.pipe(Effect.mapError(function (this: Ctx, error) { return new Failed(); }));",
                expected: &[(
                    114,
                    126,
                    "This Effect.mapError handler discards the original error and constructs a new one without it. Name the handler's parameter and pass it as `cause` (`(error) => new Failed({ ..., cause: error })`), or log it first with `Effect.tapError(Effect.logError)` before narrowing.",
                )],
            },
            Case {
                name: "this-parameter invalid 3: import * as Effect from \"effect/Effect\";",
                path: "file.ts",
                code: "import * as Effect from \"effect/Effect\";\nconst a = load.pipe(Effect.mapError(function (this: Ctx, error) { return new Failed({ message: error.message }); }));",
                expected: &[(
                    114,
                    152,
                    "This Effect.mapError handler discards the original error and constructs a new one without it. Name the handler's parameter and pass it as `cause` (`(error) => new Failed({ ..., cause: error })`), or log it first with `Effect.tapError(Effect.logError)` before narrowing.",
                )],
            },
            Case {
                name: "this-parameter invalid 4: import * as Effect from \"effect/Effect\";",
                path: "file.ts",
                code: "import * as Effect from \"effect/Effect\";\nconst a = load.pipe(Effect.mapError(function (this: Ctx) { return new Failed({ message: this.message }); }));",
                expected: &[(
                    107,
                    144,
                    "This Effect.mapError handler discards the original error and constructs a new one without it. Name the handler's parameter and pass it as `cause` (`(error) => new Failed({ ..., cause: error })`), or log it first with `Effect.tapError(Effect.logError)` before narrowing.",
                )],
            },
        ],
    );
}
