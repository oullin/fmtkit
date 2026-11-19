# Anti-slop rules

These native rules reject the low-evidence code that coding agents tend to write: casts without a stated invariant, `unknown` leaking through contracts, conditional spreads, module mocks, and tests that wait on the clock. Each message says what to write instead.

Most of them are ports of the Oxlint plugin [anti-slop](https://github.com/k-mistele/anti-slop) (Dillon Mulroy's original, maintained in Kyle Mistele's fork). The rest are adapted from t3code's Oxlint plugin. The ported rules report what the upstream plugin reports under Oxlint 1.86, at the same byte spans and with the same messages: their tests replay the upstream test cases with the findings the upstream plugin produced for them.

Besides the core `anti-slop/*` rules, the [`anti-slop-martinfowler/*` group](#code-smells-anti-slop-martinfowler) flags code smells from Martin Fowler's _Refactoring_, and the [`anti-slop-effect/*` group](#effect-anti-slop-effect) checks Effect programs.

Every `anti-slop*` rule is off in the bundled policy. Twelve rules that the k-mistele fork shares with `@nkzw/oxlint-config` 3.0 are no longer here: they are the bundled [`quality/*` rules](#nkzw-quality-quality), and their `anti-slop/` names are aliases. Enable the ones you want in `fmtkit.toml`:

```toml
[lint.rules]
"anti-slop/max-doc-comment-lines" = ["error", { max = 2 }]
"anti-slop/max-ternary-depth" = ["error", { maxDepth = 2 }]
"anti-slop/no-conditional-spread" = "error"
"anti-slop/require-safety-comment-for-type-assertion" = "error"
```

The rules read one file at a time and do not use the type checker, so they cannot see types or signatures declared in other files.

## Core (`anti-slop/*`)

| Rule                                                     | What it reports                                                                                                                                                                                                                    |
| -------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `max-doc-comment-lines`                                  | A doc comment longer than `max` lines, and a doc comment of any length on a private member (`private` or `#name`) or a constructor. Options: `max` (2), `ignoreTests` (false), `ignorePatterns` ([]). [More below](#doc-comments). |
| `max-ternary-depth`                                      | A ternary chain deeper than `maxDepth` (default 2), counted through parentheses, `as`, `!` and `satisfies`. Use a lookup table, a `switch`, or a named decision function.                                                          |
| `no-conditional-spread`                                  | Spreading any ternary or logical expression (`&&`, `\|\|`, `??`) in objects, arrays, call arguments and JSX attributes. Build the value in separate statements.                                                                    |
| `no-disabled-type-assertion-rules`                       | An inline disable comment for a rule that keeps type assertions out, or one that names no rule. A reason does not make it acceptable. `additionalRules` adds rule names to protect.                                                |
| `no-module-mocking`                                      | Vitest or Jest module mocking.                                                                                                                                                                                                     |
| `no-reprovide-ambient-service`                           | An Effect service read with `yield* Service` and handed straight back with `Effect.provideService(Service, service)`.                                                                                                              |
| `no-runtime-typeof`                                      | Runtime `typeof` checks. Decode external values at their I/O boundary.                                                                                                                                                             |
| `no-shape-in-symbol-names`                               | "shape" anywhere in a symbol name, in any letter case.                                                                                                                                                                             |
| `no-unknown-parameters`                                  | Parameters typed `unknown`, or a union containing it. `cause` and the subject of a type predicate or assertion (`value is T`, `asserts value`) are allowed.                                                                        |
| `require-readable-spacing`                               | A missing blank line around imports, top-level statements, declarations, multiline variables, control flow, and after block-like statements. Fixable.                                                                              |
| `require-safety-comment-for-type-assertion`              | A type assertion, other than `as const`, without a nearby `SAFETY:` comment stating its invariant.                                                                                                                                 |
| `require-suppression-reason`                             | A suppression that does not name its rules or say why after `--`, and directives for other linters.                                                                                                                                |
| `no-ambient-nondeterminism`                              | Reading the clock or the entropy source outside a mock.                                                                                                                                                                            |
| `no-hookless-use-function`                               | A React `use*` function that calls no hook.                                                                                                                                                                                        |
| `no-low-value-comments`                                  | Comments that repeat the code, divide a file into sections, label test steps, or keep dead code.                                                                                                                                   |
| `no-polling-loop-in-tests`, `no-real-time-wait-in-tests` | Tests that spin until something becomes true, or sleep on the real clock.                                                                                                                                                          |
| `no-test-in-loop`                                        | Tests registered inside a `for` loop. Use `.each`.                                                                                                                                                                                 |
| `no-unreasoned-test-skip`                                | A skipped or to-do test without a comment that links its issue.                                                                                                                                                                    |

`require-readable-spacing` and the formatter agree. `fmtkit format` applies lint fixes before formatting, and the formatted result never triggers the rule again. The formatter's own spacing is looser, though, so `fmtkit check` can still report this rule on code the formatter produced, for example two consecutive top-level statements.

### Doc comments

`max-doc-comment-lines` targets comments written for their own sake: the paragraph that narrates a design above a class, or the comment on a member that only its own class calls. `no-low-value-comments` catches comments that repeat the code; this rule catches the ones that say too much.

A doc comment is the run of own-line comments, `/** … */` blocks or `//` lines, directly above a declaration, a statement, a class member, an interface or enum member, an object property or a constructor parameter property. A blank line between the comment and the code ends it. Only content lines count: lines holding nothing but `/**`, `*/` or the `*` gutter, and blank lines, do not.

- Longer than `max` lines (default 2): "Keep a doc comment to what the code can't say, in at most 2 lines."
- Any length on a `private` or `#name` member, a `private` parameter property, or a constructor (private or not): drop it, and say what callers need on the class or on the factory they call.

The rule never counts or reports a comment that a tool or another rule needs as written:

- Tool directives, whose lines are skipped: `oxlint-`, `eslint-`, `tslint-` and `deno-lint-` comments, `@ts-expect-error`, `@ts-ignore` and other `@ts-` directives, `@effect-diagnostics`, `prettier-ignore`, `oxfmt-ignore`, `biome-ignore`, `fallow-ignore-…`, `istanbul`, `c8` and `v8` coverage hints, `/// <reference …>`, `#region`/`#endregion`, `@__PURE__` and `@__NO_SIDE_EFFECTS__`, `@vite-ignore`, webpack magic comments, `@jsx`, `@vitest-environment` and `@jest-environment`, and preserved `/*! … */`, `@license` and `@preserve` comments. These are the comments `no-low-value-comments` skips too.
- The `// observability: swallowed because …` escape the [`observability/*` catch rules](observability-catches.md#allow-an-intentional-swallow) read.
- A comment stating a `SAFETY:` invariant, which `require-safety-comment-for-type-assertion` requires, whatever its length.
- A license or copyright notice (`Copyright`, `License`, `SPDX-License-Identifier`, `©`) that opens the file, after an optional hashbang.
- A comment matching one of `ignorePatterns`: regular expressions, tested against the comment's text without its delimiters. Add your perfectionist `partitionByComment` patterns here, for example `ignorePatterns = ["^\\s*Part:"]`, so a partition comment above a private member is not reported.

`ignoreTests` (default false) skips files in `__tests__`, `__mocks__`, `test` or `tests` directories and `*.test.*` or `*.spec.*` scripts, as the `anti-slop-martinfowler/*` rules do. To skip other files, use a [lint exemption](../README.md#lint-exemptions). The rule reads TypeScript and JavaScript only; Go files are not checked.

### Behaviour not ported

- `no-reprovide-ambient-service`: upstream keeps the Effect names it has seen imported for the whole run, so a name imported in one file can trigger findings in later files. fmtkit judges each file on its own imports.
- `require-readable-spacing`: inside an `export =` the rule skips, a multiline template literal counts its own blank lines, where upstream reads it as one token.

## nkzw quality (`quality/*`)

`@nkzw/oxlint-config` 3.0 ships twelve rules adapted from Dillon Mulroy's anti-slop as its `quality` plugin, and the bundled policy enables all of them. They report what the nkzw plugin reports under Oxlint 1.94, at the same byte spans and with the same messages, and take no options.

| Rule                                 | What it reports                                                                                                                                                      |
| ------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `no-array-filter-map`                | `.filter(…).map(…)` or `.map(…).filter(…)` on a value the file shows is an array. Use one pass or lazy iterator helpers.                                             |
| `no-chained-type-assertions`         | Nested `as` or angle-bracket assertions, unless every link is `as const`.                                                                                            |
| `no-conditional-empty-object-spread` | `...(flag ? value : {})` used to omit a property.                                                                                                                    |
| `no-known-value-widening`            | A value the file already knows flowing into an explicitly broad or anonymous type.                                                                                   |
| `no-object-parameters`               | Parameters typed `object`, directly or through an alias.                                                                                                             |
| `no-reduce-accumulator-copy`         | Copying a reducer's growing accumulator on every iteration with `Object.assign`, `Array.from` or array copy methods. Spreads are `oxc/no-accumulating-spread`'s job. |
| `no-reflect-apply`, `no-reflect-get` | `Reflect.apply` and `Reflect.get`.                                                                                                                                   |
| `no-unknown-returns`                 | An explicit `unknown` or `Promise<unknown>` return type.                                                                                                             |
| `no-unknown-type-aliases`            | A module-level alias that resolves to `unknown`.                                                                                                                     |
| `no-unsafe-dictionary-type`          | A dictionary whose value type is `unknown`, `any`, `object` or `{}`, or a union or alias that holds one of them.                                                     |
| `no-widen-then-assert`               | A local `const` that widens a known value and is later asserted back to a narrower type.                                                                             |

fmtkit shipped the same twelve rules as `anti-slop/*`, ported from the k-mistele fork, and folded them into these. The old names are aliases: `"anti-slop/no-object-parameters" = "warn"` under `[lint.rules]`, an `[[exempt]]` entry, or an `oxlint-disable` directive that names one configures or suppresses the `quality/` rule, and findings are reported under the `quality/` name. Configuring both names of one rule is an error. Seven rules behave as the fork's did. The other five follow the nkzw plugin: `no-known-value-widening`, `no-object-parameters`, `no-unknown-returns`, `no-unknown-type-aliases` and `no-unsafe-dictionary-type` resolve type names declared inside blocks and namespaces and through generic aliases, where the fork saw module-level aliases only, and `no-object-parameters` names only the parameter in its message.

## Code smells (`anti-slop-martinfowler/*`)

These rules flag smells from Martin Fowler's _Refactoring_ that a syntax tree can show. A smell is a reason to look, not proof of a problem, so enable them at `warn`. Upstream leaves `message-chains` off by default.

```toml
[lint.rules]
"anti-slop-martinfowler/global-data" = "warn"
"anti-slop-martinfowler/long-function" = ["warn", { maxStatements = 30, maxLines = 80 }]
"anti-slop-martinfowler/long-parameter-list" = ["warn", { max = 4 }]
"anti-slop-martinfowler/loops" = "warn"
"anti-slop-martinfowler/mutable-data" = "warn"
"anti-slop-martinfowler/mysterious-name" = ["warn", { allowNames = ["fetch", "layer", "lookup", "make", "of"] }]
"anti-slop-martinfowler/repeated-switches" = "warn"
```

| Rule                  | What it reports                                                                                                                                                                        | Options (defaults)                                                                                                                                        |
| --------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `global-data`         | An exported `let` or `var`, or an exported object, array, `Map` or `Set` that the file mutates.                                                                                        | —                                                                                                                                                         |
| `long-function`       | A function whose own statements or lines, leaving out nested functions, exceed a limit.                                                                                                | `maxStatements` (30), `maxLines` (80)                                                                                                                     |
| `long-parameter-list` | A function with more positional parameters than `max`. Rest parameters and `this` do not count.                                                                                        | `max` (4)                                                                                                                                                 |
| `loops`               | A `for` loop that builds an array, object, `Map` or `Set` declared outside it. Loops that exit early, write other outer state, or update a buffer in place are skipped.                | `includeWhile` (false)                                                                                                                                    |
| `message-chains`      | Property reads and zero-argument `getX()` calls that navigate through more than `maxDepth` objects. Builders, `.pipe(…)` chains and global roots are skipped.                          | `maxDepth` (3), `ignoreRoots` (`console`, `document`, `globalThis`, `location`, `navigator`, `process`, `self`, `window`), `ignorePascalCaseRoots` (true) |
| `mutable-data`        | A module-level `let` or `var` that functions reassign, with the number of functions that write it. Exported bindings are left to `global-data`.                                        | —                                                                                                                                                         |
| `mysterious-name`     | A top-level function named with a bare generic verb (`run`, `handle`, `process`) or a generic verb and a vague noun (`processData`), and a `const` that only aliases another function. | `verbs`, `vagueNouns`, `allowNames`, `checkAliases` (true)                                                                                                |
| `repeated-switches`   | One value branched on over the same cases in several places in a file, whether by `switch`, `if` chains or nested ternaries.                                                           | `minOccurrences` (2), `minSharedCases` (2)                                                                                                                |

Every rule also takes `ignoreTests` (default true), which skips files in `__tests__`, `__mocks__`, `test` or `tests` directories and `*.test.*` or `*.spec.*` scripts. Upstream matches those directories anywhere in the absolute path; fmtkit matches them within the repository.

## Effect (`anti-slop-effect/*`)

These rules encode a review policy for [Effect](https://effect.website) programs: tagged errors and values go through Effect's constructors and handlers, services own their construction and layers, and failures keep their cause and span. Enable them only in repositories that use Effect.

```toml
[lint.rules]
"anti-slop-effect/require-cause-when-narrowing" = "error"
"anti-slop-effect/no-test-code-in-src" = ["error", { sourceDirectories = ["src"] }]
"anti-slop-effect/require-service-make-layer" = ["error", { allowTagOnly = true }]
```

`Effect`, `Layer`, `Context` and the other modules are recognised by their imports from `effect` or `effect/<Module>`.

| Rule                               | What it reports                                                                                                                                                                                                                                              |
| ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `no-manual-effect-error-tag`       | Comparing or switching on `_tag` inside a broad `Effect.catch`, `catchAll` or `catchIf` handler. Use `Effect.catchTag(s)` or `Effect.catchReason(s)`.                                                                                                        |
| `no-manual-tag-comparison`         | Comparing or switching on `_tag` anywhere else. Use `Match.tag(s)`, `Predicate.isTagged` or a tagged enum's `$match`.                                                                                                                                        |
| `no-manual-tagged-construction`    | An object literal or class property that writes `_tag` by hand. Use the Schema or Data constructor.                                                                                                                                                          |
| `no-manual-tagged-type`            | A `_tag` property in a type literal or interface. Derive the type from a tagged Schema or Data constructor.                                                                                                                                                  |
| `no-ordie-in-service-construction` | `Effect.orDie` or `orDieWith` while a service is constructed, and `Layer.orDie` anywhere. Keep construction failures typed in the layer's error channel.                                                                                                     |
| `no-service-constructor-imports`   | Importing a project-local `make<Capability>` constructor. Import the owning layer and yield the service.                                                                                                                                                     |
| `no-service-parameters`            | Parameters typed `Context.Context`, `ManagedRuntime.ManagedRuntime`, `Layer.Layer`, or a service class declared in the file. Layer combinators and callbacks the Effect API dictates are allowed.                                                            |
| `no-test-code-in-src`              | Test files, imports from test directories, and test-named exports (`layerTest`, `FakeClock`, `makeMockStore`) inside a source directory. Options: `sourceDirectories` (`["src"]`), `testDirectories` (`["test", "tests", "__tests__"]`), `testNamePatterns`. |
| `prefer-effect-match`              | A chain of two or more ternaries comparing one value with literals. Use `Match`.                                                                                                                                                                             |
| `prefer-tagged-error-handling`     | `Predicate.isTagged` inside a broad `Effect.catch*` handler. Use `Effect.catchTag(s)`.                                                                                                                                                                       |
| `prefer-yield-service`             | A context captured with `yield* Effect.context()` and passed back to `Effect.provide`, `provideContext`, `Layer.provide` or `Layer.succeedContext`. Yield each service instead.                                                                              |
| `require-cause-when-narrowing`     | An `Effect.mapError`, `catch`, `catchAll`, `catchCause` or `orElse` handler that ignores its error and builds a new one without `cause`, unless an earlier `tapError` step logged it.                                                                        |
| `require-operation-span`           | An operation returned by a service's `make` that is not traced with `Effect.fn("name")` or `Effect.withSpan`.                                                                                                                                                |
| `require-service-make-layer`       | A service class without `make` and `static layer = Layer.effect(this, this.make)`, or a variant layer that does not derive from them. `allowTagOnly` (true) allows a service with neither.                                                                   |

Test and spec files are skipped by the rules that concern production code. `no-test-code-in-src` matches its directory names within the repository path; upstream matched them anywhere in the absolute path.
