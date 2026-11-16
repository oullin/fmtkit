//! `anti-slop/no-reprovide-ambient-service`: expectations recorded from the upstream plugin
//! (github.com/k-mistele/anti-slop) under oxlint 1.86, one case per upstream
//! RuleTester case in `src/rules/no-reprovide-ambient-service.test.ts`.

use super::{Case, check};

/// src/rules/no-reprovide-ambient-service.test.ts
#[test]
fn upstream() {
    check(
        "anti-slop/no-reprovide-ambient-service",
        "[]",
        &[
            Case {
                name: "valid 0:         import { Effect } from 'effect'",
                path: "file.ts",
                code: "\n        import { Effect } from 'effect'\n        import * as HttpClient from 'effect/unstable/http/HttpClient'\n        const operation = Effect.gen(function* () {\n          yield* HttpClient.HttpClient\n        })\n      ",
                expected: &[],
            },
            Case {
                name: "valid 1:         import { Effect } from 'effect'",
                path: "file.ts",
                code: "\n        import { Effect } from 'effect'\n        const operation = Effect.gen(function* () {\n          const first = yield* FirstService\n          return Effect.provideService(request, SecondService, first)\n        })\n      ",
                expected: &[],
            },
            Case {
                name: "valid 2:         import { Effect } from 'effect'",
                path: "file.ts",
                code: "\n        import { Effect } from 'effect'\n        import * as HttpClient from 'effect/unstable/http/HttpClient'\n        const operation = Effect.gen(function* () {\n          const baseClient = yield* HttpClient.HttpClient\n          const tenantClient = configureForTenant(baseClient)\n          return request.pipe(Effect.provideService(HttpClient.HttpClient, tenantClient))\n        })\n      ",
                expected: &[],
            },
            Case {
                name: "valid 3:         import { Effect } from 'effect'",
                path: "file.ts",
                code: "\n        import { Effect } from 'effect'\n        const operation = Effect.gen(function* () {\n          const first = yield* FirstService\n          return request.pipe(Effect.provideService(SecondService, first))\n        })\n      ",
                expected: &[],
            },
            Case {
                name: "valid 4:         import { Effect } from 'other-library'",
                path: "file.ts",
                code: "\n        import { Effect } from 'other-library'\n        const operation = Effect.gen(function* () {\n          const service = yield* Service\n          return request.pipe(Effect.provideService(Service, service))\n        })\n      ",
                expected: &[],
            },
            Case {
                name: "invalid 5:         import { Effect } from 'effect'",
                path: "file.ts",
                code: "\n        import { Effect } from 'effect'\n        import * as HttpClient from 'effect/unstable/http/HttpClient'\n        const ServiceLive = Layer.effect(Service, Effect.gen(function* () {\n          const httpClient = yield* HttpClient.HttpClient\n          return Service.of({\n            execute: () => request.pipe(\n              Effect.provideService(HttpClient.HttpClient, httpClient),\n            ),\n          })\n        }))\n      ",
                expected: &[(
                    330,
                    386,
                    "Do not capture \"HttpClient.HttpClient\" from the Effect environment and re-provide it with Effect.provideService. Leave \"HttpClient.HttpClient\" in the operation's R channel and provide its implementing Layer once at the application runtime composition root. If the runtime has an explicit service union, include \"HttpClient.HttpClient\" there; use Layer.provideMerge when the service must remain available after layer construction. A local provide is only appropriate for a genuinely dynamic implementation, such as a client configured from per-tenant credentials.",
                )],
            },
            Case {
                name: "invalid 6:         import * as Fx from 'effect/Effect'",
                path: "file.ts",
                code: "\n        import * as Fx from 'effect/Effect'\n        import * as HttpClient from 'effect/unstable/http/HttpClient'\n        const operation = Fx.gen(function* () {\n          const httpClient = yield* HttpClient.HttpClient\n          return request.pipe(Fx.provideService(HttpClient.HttpClient, httpClient))\n        })\n      ",
                expected: &[(
                    251,
                    303,
                    "Do not capture \"HttpClient.HttpClient\" from the Effect environment and re-provide it with Effect.provideService. Leave \"HttpClient.HttpClient\" in the operation's R channel and provide its implementing Layer once at the application runtime composition root. If the runtime has an explicit service union, include \"HttpClient.HttpClient\" there; use Layer.provideMerge when the service must remain available after layer construction. A local provide is only appropriate for a genuinely dynamic implementation, such as a client configured from per-tenant credentials.",
                )],
            },
            Case {
                name: "invalid 7:         import { Effect } from 'effect'",
                path: "file.ts",
                code: "\n        import { Effect } from 'effect'\n        import * as HttpClient from 'effect/unstable/http/HttpClient'\n        const operation = Effect.gen(function* () {\n          const httpClient = yield* HttpClient.HttpClient\n          return Effect.provideService(request, HttpClient.HttpClient, httpClient)\n        })\n      ",
                expected: &[(
                    238,
                    303,
                    "Do not capture \"HttpClient.HttpClient\" from the Effect environment and re-provide it with Effect.provideService. Leave \"HttpClient.HttpClient\" in the operation's R channel and provide its implementing Layer once at the application runtime composition root. If the runtime has an explicit service union, include \"HttpClient.HttpClient\" there; use Layer.provideMerge when the service must remain available after layer construction. A local provide is only appropriate for a genuinely dynamic implementation, such as a client configured from per-tenant credentials.",
                )],
            },
        ],
    );
}

/// Edge cases recorded from the upstream plugin under oxlint 1.86.
#[test]
fn edges() {
    check(
        "anti-slop/no-reprovide-ambient-service",
        "[]",
        &[
            // Recorded alone: in one oxlint run upstream's `createOnce` namespace set leaks
            // across files, so a batch run reports this from an earlier file's import.
            Case {
                name: "edge 0: const operation = Effect.gen(function* () {",
                path: "file.ts",
                code: "const operation = Effect.gen(function* () {\n  const client = yield* Client\n  return request.pipe(Effect.provideService(Client, client))\n})\nimport { Effect } from 'effect'\n",
                expected: &[],
            },
            Case {
                name: "edge 1: import { Effect as E } from 'effect'",
                path: "file.ts",
                code: "import { Effect as E } from 'effect'\nconst operation = E.gen(function* () {\n  const client = yield* Client\n  return request.pipe(E.provideService(Client, client))\n})\n",
                expected: &[(
                    129,
                    161,
                    "Do not capture \"Client\" from the Effect environment and re-provide it with Effect.provideService. Leave \"Client\" in the operation's R channel and provide its implementing Layer once at the application runtime composition root. If the runtime has an explicit service union, include \"Client\" there; use Layer.provideMerge when the service must remain available after layer construction. A local provide is only appropriate for a genuinely dynamic implementation, such as a client configured from per-tenant credentials.",
                )],
            },
            Case {
                name: "edge 2: import Fx from 'effect/Effect'",
                path: "file.ts",
                code: "import Fx from 'effect/Effect'\nconst operation = Fx.gen(function* () {\n  const client = yield* Client\n  return request.pipe(Fx.provideService(Client, client))\n})\n",
                expected: &[(
                    124,
                    157,
                    "Do not capture \"Client\" from the Effect environment and re-provide it with Effect.provideService. Leave \"Client\" in the operation's R channel and provide its implementing Layer once at the application runtime composition root. If the runtime has an explicit service union, include \"Client\" there; use Layer.provideMerge when the service must remain available after layer construction. A local provide is only appropriate for a genuinely dynamic implementation, such as a client configured from per-tenant credentials.",
                )],
            },
            Case {
                name: "edge 3: import type { Effect } from 'effect'",
                path: "file.ts",
                code: "import type { Effect } from 'effect'\nconst operation = Effect.gen(function* () {\n  const client = yield* Client\n  return request.pipe(Effect.provideService(Client, client))\n})\n",
                expected: &[(
                    134,
                    171,
                    "Do not capture \"Client\" from the Effect environment and re-provide it with Effect.provideService. Leave \"Client\" in the operation's R channel and provide its implementing Layer once at the application runtime composition root. If the runtime has an explicit service union, include \"Client\" there; use Layer.provideMerge when the service must remain available after layer construction. A local provide is only appropriate for a genuinely dynamic implementation, such as a client configured from per-tenant credentials.",
                )],
            },
            Case {
                name: "edge 4: import { Effect } from 'effect'",
                path: "file.ts",
                code: "import { Effect } from 'effect'\nconst operation = Effect.gen(function* () {\n  const client = yield* Client.Tag\n  return Effect.provideService(request, (Client.Tag as Tag), client!)\n})\n",
                expected: &[(
                    120,
                    180,
                    "Do not capture \"Client.Tag\" from the Effect environment and re-provide it with Effect.provideService. Leave \"Client.Tag\" in the operation's R channel and provide its implementing Layer once at the application runtime composition root. If the runtime has an explicit service union, include \"Client.Tag\" there; use Layer.provideMerge when the service must remain available after layer construction. A local provide is only appropriate for a genuinely dynamic implementation, such as a client configured from per-tenant credentials.",
                )],
            },
            Case {
                name: "edge 5: import { Effect } from 'effect'",
                path: "file.ts",
                code: "import { Effect } from 'effect'\nconst operation = Effect.gen(function* () {\n  let client = yield* Client\n  return request.pipe(Effect.provideService(Client, client))\n})\n",
                expected: &[(
                    127,
                    164,
                    "Do not capture \"Client\" from the Effect environment and re-provide it with Effect.provideService. Leave \"Client\" in the operation's R channel and provide its implementing Layer once at the application runtime composition root. If the runtime has an explicit service union, include \"Client\" there; use Layer.provideMerge when the service must remain available after layer construction. A local provide is only appropriate for a genuinely dynamic implementation, such as a client configured from per-tenant credentials.",
                )],
            },
            Case {
                name: "edge 6: import { Effect } from 'effect'",
                path: "file.ts",
                code: "import { Effect } from 'effect'\nconst operation = Effect.gen(function* () {\n  const client = (yield* (Client))\n  return request.pipe(Effect.provideService(Client, client))\n})\n",
                expected: &[(
                    133,
                    170,
                    "Do not capture \"Client\" from the Effect environment and re-provide it with Effect.provideService. Leave \"Client\" in the operation's R channel and provide its implementing Layer once at the application runtime composition root. If the runtime has an explicit service union, include \"Client\" there; use Layer.provideMerge when the service must remain available after layer construction. A local provide is only appropriate for a genuinely dynamic implementation, such as a client configured from per-tenant credentials.",
                )],
            },
            Case {
                name: "edge 7: import { Effect } from 'effect'",
                path: "file.ts",
                code: "import { Effect } from 'effect'\nconst operation = Effect.gen(function* () {\n  const client = yield Client\n  return request.pipe(Effect.provideService(Client, client))\n})\n",
                expected: &[],
            },
            Case {
                name: "edge 8: import { Effect } from 'effect'",
                path: "file.ts",
                code: "import { Effect } from 'effect'\nconst operation = Effect.gen(function* () {\n  const client = yield* Client\n  return request.pipe(Effect.provideService(Client, ...[client]))\n})\n",
                expected: &[],
            },
            Case {
                name: "edge 9: import { Effect } from 'effect'",
                path: "file.ts",
                code: "import { Effect } from 'effect'\nconst operation = Effect.gen(function* () {\n  const client = yield* Client\n  return request.pipe(Effect['provideService'](Client, client))\n})\n",
                expected: &[],
            },
            Case {
                name: "edge 10: import { Effect } from 'effect'",
                path: "file.ts",
                code: "import { Effect } from 'effect'\nconst operation = Effect.gen(function* () {\n  const client = yield* Client .Tag\n  return request.pipe(Effect.provideService(Client.Tag, client))\n})\n",
                expected: &[],
            },
            Case {
                name: "edge 11: import { Effect } from 'effect'",
                path: "file.ts",
                code: "import { Effect } from 'effect'\nconst operation = Effect.gen(function* () {\n  const client = yield* Client\n  return (client) => request.pipe(Effect.provideService(Client, client))\n})\n",
                expected: &[],
            },
            Case {
                name: "edge 12: import { Effect } from 'effect'",
                path: "file.ts",
                code: "import { Effect } from 'effect'\nconst operation = Effect.gen(function* () {\n  const client = yield* Client\n  return request.pipe(Effect?.provideService(Client, client))\n})\n",
                expected: &[(
                    129,
                    167,
                    "Do not capture \"Client\" from the Effect environment and re-provide it with Effect.provideService. Leave \"Client\" in the operation's R channel and provide its implementing Layer once at the application runtime composition root. If the runtime has an explicit service union, include \"Client\" there; use Layer.provideMerge when the service must remain available after layer construction. A local provide is only appropriate for a genuinely dynamic implementation, such as a client configured from per-tenant credentials.",
                )],
            },
            Case {
                name: "edge 13: import { Effect } from 'effect'",
                path: "file.ts",
                code: "import { Effect } from 'effect'\nconst operation = Effect.gen(function* () {\n  var client = yield* Client\n  var client = yield* Client\n  return request.pipe(Effect.provideService(Client, client))\n})\n",
                expected: &[],
            },
            Case {
                name: "edge 14: import { Effect } from 'effect'",
                path: "file.ts",
                code: "import { Effect } from 'effect'\nconst operation = Effect.gen(function* () {\n  const { client } = yield* Client\n  return request.pipe(Effect.provideService(Client, client))\n})\n",
                expected: &[(
                    133,
                    170,
                    "Do not capture \"Client\" from the Effect environment and re-provide it with Effect.provideService. Leave \"Client\" in the operation's R channel and provide its implementing Layer once at the application runtime composition root. If the runtime has an explicit service union, include \"Client\" there; use Layer.provideMerge when the service must remain available after layer construction. A local provide is only appropriate for a genuinely dynamic implementation, such as a client configured from per-tenant credentials.",
                )],
            },
            Case {
                name: "edge 15: import { Effect } from 'effect'",
                path: "file.ts",
                code: "import { Effect } from 'effect'\nconst operation = Effect.gen(function* () {\n  const client = yield* Client\n  return request.pipe((Effect.provideService as any)(Client, client))\n})\n",
                expected: &[(
                    129,
                    175,
                    "Do not capture \"Client\" from the Effect environment and re-provide it with Effect.provideService. Leave \"Client\" in the operation's R channel and provide its implementing Layer once at the application runtime composition root. If the runtime has an explicit service union, include \"Client\" there; use Layer.provideMerge when the service must remain available after layer construction. A local provide is only appropriate for a genuinely dynamic implementation, such as a client configured from per-tenant credentials.",
                )],
            },
            // Recorded alone: in one oxlint run upstream's `createOnce` namespace set leaks
            // across files, so a batch run reports this from an earlier file's import.
            Case {
                name: "edge 16: import * as Effect from 'effect'",
                path: "file.ts",
                code: "import * as Effect from 'effect'\nconst operation = Effect.gen(function* () {\n  const client = yield* Client\n  return request.pipe(Effect.provideService(Client, client))\n})\n",
                expected: &[],
            },
            // Recorded alone: in one oxlint run upstream's `createOnce` namespace set leaks
            // across files, so a batch run reports this from an earlier file's import.
            Case {
                name: "edge 17: import { Effect } from 'effect/Effect'",
                path: "file.ts",
                code: "import { Effect } from 'effect/Effect'\nconst operation = Effect.gen(function* () {\n  const client = yield* Client\n  return request.pipe(Effect.provideService(Client, client))\n})\n",
                expected: &[],
            },
            Case {
                name: "edge 18: import { \"Effect\" as Fx } from 'effect'",
                path: "file.ts",
                code: "import { \"Effect\" as Fx } from 'effect'\nconst operation = Fx.gen(function* () {\n  const client = yield* Client\n  return request.pipe(Fx.provideService(Client, client))\n})\n",
                expected: &[(
                    133,
                    166,
                    "Do not capture \"Client\" from the Effect environment and re-provide it with Effect.provideService. Leave \"Client\" in the operation's R channel and provide its implementing Layer once at the application runtime composition root. If the runtime has an explicit service union, include \"Client\" there; use Layer.provideMerge when the service must remain available after layer construction. A local provide is only appropriate for a genuinely dynamic implementation, such as a client configured from per-tenant credentials.",
                )],
            },
            Case {
                name: "edge 19: import { Effect } from 'effect'",
                path: "file.ts",
                code: "import { Effect } from 'effect'\nfunction* run() {\n  const client = yield* Client\n  return Effect.provideService(Client, client)\n}\n",
                expected: &[(
                    90,
                    127,
                    "Do not capture \"Client\" from the Effect environment and re-provide it with Effect.provideService. Leave \"Client\" in the operation's R channel and provide its implementing Layer once at the application runtime composition root. If the runtime has an explicit service union, include \"Client\" there; use Layer.provideMerge when the service must remain available after layer construction. A local provide is only appropriate for a genuinely dynamic implementation, such as a client configured from per-tenant credentials.",
                )],
            },
            Case {
                name: "edge 20: import { Effect } from 'effect'",
                path: "file.ts",
                code: "import { Effect } from 'effect'\nconst operation = Effect.gen(function* () {\n  const client = yield* Client\n  return Effect.provideService(client)\n})\n",
                expected: &[],
            },
        ],
    );
}
