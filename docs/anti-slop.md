# Anti-slop rules

These native rules reject the low-evidence code that coding agents tend to write: casts without a stated invariant, `unknown` leaking through contracts, conditional spreads, module mocks, and tests that wait on the clock. Each message says what to write instead.

Most of them are ports of the Oxlint plugin [anti-slop](https://github.com/k-mistele/anti-slop) (Dillon Mulroy's original, maintained in Kyle Mistele's fork). The rest are adapted from t3code's Oxlint plugin. The ported rules report what the upstream plugin reports under Oxlint 1.86, at the same byte spans and with the same messages: their tests replay the upstream test cases with the findings the upstream plugin produced for them.

Besides the core `anti-slop/*` rules, the [`anti-slop-martinfowler/*` group](#code-smells-anti-slop-martinfowler) flags code smells from Martin Fowler's _Refactoring_.

Every rule is off in the bundled policy. Enable the ones you want in `fmtkit.toml`:

```toml
[lint.rules]
"anti-slop/max-ternary-depth" = ["error", { maxDepth = 2 }]
"anti-slop/no-conditional-spread" = "error"
"anti-slop/require-safety-comment-for-type-assertion" = "error"
```

The rules read one file at a time and do not use the type checker, so they cannot see types or signatures declared in other files.

## Core (`anti-slop/*`)

| Rule                                                     | What it reports                                                                                                                                                                     |
| -------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `max-ternary-depth`                                      | A ternary chain deeper than `maxDepth` (default 2), counted through parentheses, `as`, `!` and `satisfies`. Use a lookup table, a `switch`, or a named decision function.           |
| `no-array-filter-map`                                    | `.filter(…).map(…)` or `.map(…).filter(…)` on a value the file shows is an array. Use one pass or lazy iterator helpers.                                                            |
| `no-chained-type-assertions`                             | Nested `as` or angle-bracket assertions, unless every link is `as const`.                                                                                                           |
| `no-conditional-empty-object-spread`                     | `...(flag ? value : {})` used to omit a property.                                                                                                                                   |
| `no-conditional-spread`                                  | Spreading any ternary or logical expression (`&&`, `\|\|`, `??`) in objects, arrays, call arguments and JSX attributes. Build the value in separate statements.                     |
| `no-disabled-type-assertion-rules`                       | An inline disable comment for a rule that keeps type assertions out, or one that names no rule. A reason does not make it acceptable. `additionalRules` adds rule names to protect. |
| `no-known-value-widening`                                | A value the file already knows flowing into an explicitly broad or anonymous type.                                                                                                  |
| `no-module-mocking`                                      | Vitest or Jest module mocking.                                                                                                                                                      |
| `no-object-parameters`                                   | Parameters typed `object`, directly or through an alias.                                                                                                                            |
| `no-reduce-accumulator-copy`                             | Copying a reducer's growing accumulator on every iteration with `Object.assign`, `Array.from` or array copy methods. Spreads are `oxc/no-accumulating-spread`'s job.                |
| `no-reflect-apply`, `no-reflect-get`                     | `Reflect.apply` and `Reflect.get`.                                                                                                                                                  |
| `no-reprovide-ambient-service`                           | An Effect service read with `yield* Service` and handed straight back with `Effect.provideService(Service, service)`.                                                               |
| `no-runtime-typeof`                                      | Runtime `typeof` checks. Decode external values at their I/O boundary.                                                                                                              |
| `no-shape-in-symbol-names`                               | "shape" anywhere in a symbol name, in any letter case.                                                                                                                              |
| `no-unknown-parameters`                                  | Parameters typed `unknown`, or a union containing it. `cause` and the subject of a type predicate or assertion (`value is T`, `asserts value`) are allowed.                         |
| `no-unknown-returns`                                     | An explicit `unknown` or `Promise<unknown>` return type.                                                                                                                            |
| `no-unknown-type-aliases`                                | A module-level alias that resolves to `unknown`.                                                                                                                                    |
| `no-unsafe-dictionary-type`                              | A dictionary whose value type is `unknown`, `any`, `object` or `{}`, or a union or alias that holds one of them.                                                                    |
| `no-widen-then-assert`                                   | A local `const` that widens a known value and is later asserted back to a narrower type.                                                                                            |
| `require-readable-spacing`                               | A missing blank line around imports, top-level statements, declarations, multiline variables, control flow, and after block-like statements. Fixable.                               |
| `require-safety-comment-for-type-assertion`              | A type assertion, other than `as const`, without a nearby `SAFETY:` comment stating its invariant.                                                                                  |
| `require-suppression-reason`                             | A suppression that does not name its rules or say why after `--`, and directives for other linters.                                                                                 |
| `no-ambient-nondeterminism`                              | Reading the clock or the entropy source outside a mock.                                                                                                                             |
| `no-hookless-use-function`                               | A React `use*` function that calls no hook.                                                                                                                                         |
| `no-low-value-comments`                                  | Comments that repeat the code, divide a file into sections, label test steps, or keep dead code.                                                                                    |
| `no-polling-loop-in-tests`, `no-real-time-wait-in-tests` | Tests that spin until something becomes true, or sleep on the real clock.                                                                                                           |
| `no-test-in-loop`                                        | Tests registered inside a `for` loop. Use `.each`.                                                                                                                                  |
| `no-unreasoned-test-skip`                                | A skipped or to-do test without a comment that links its issue.                                                                                                                     |

`require-readable-spacing` and the formatter agree. `fmtkit format` applies lint fixes before formatting, and the formatted result never triggers the rule again. The formatter's own spacing is looser, though, so `fmtkit check` can still report this rule on code the formatter produced, for example two consecutive top-level statements.

### Behaviour not ported

- `no-reprovide-ambient-service`: upstream keeps the Effect names it has seen imported for the whole run, so a name imported in one file can trigger findings in later files. fmtkit judges each file on its own imports.
- `require-readable-spacing`: inside an `export =` the rule skips, a multiline template literal counts its own blank lines, where upstream reads it as one token.

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
