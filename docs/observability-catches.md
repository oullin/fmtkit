# Observability catch rules

These native rules are off in the bundled policy. Enable them in `fmtkit.toml`:

```toml
[lint.rules]
"observability/no-unobserved-catch" = "error"
"observability/no-swallowing-promise-catch" = "error"
```

## Catch clauses

`observability/no-unobserved-catch` requires each `catch` body, including a bindingless `catch`, to report or throw an error. A body can also propagate its caught binding through a return value, variable initialiser, or `=` assignment. That value must reference the caught binding and contain an object property named `errorClass`:

```ts
try {
	await action();
} catch (error) {
	return { cause: error, errorClass: classify(error) };
}
```

The default observers are `reportException`, `*.reportException`, `OperationalLogger.*`, `console.warn`, `console.error`, and `Sentry.captureException`. String-literal property accesses such as `console['error'](error)` also count. Parentheses and TypeScript assertions around callees and owners are ignored.

Reporting or propagation inside a nested function, class, or catch does not handle the enclosing catch. This rule checks for syntax evidence; it does not prove that every execution path handles the error.

## Promise catch handlers

`observability/no-swallowing-promise-catch` flags `.catch(handler)` when the resolved handler:

- Has an empty block, or a block containing only empty statements.
- Explicitly returns no value, `undefined`, `void x`, or a conditional with an undefined branch.
- Has an expression body with one of those undefined results.

The rule follows same-file function declarations, methods, function initialisers, and aliases. Unresolved handlers such as an imported `reportException` are accepted. A non-empty block that reports an error without an explicit return is accepted. Nested functions, classes, and catch clauses own their returns.

Resolution uses lexical bindings, object initialisers, and class methods, including `this` and inherited methods. Receiver types that require TypeScript type checking, such as a factory call's inferred return type, remain unresolved.

## Allow an intentional swallow

Place this single-line comment immediately above the reported site's line, with a non-empty reason:

```ts
// observability: swallowed because the request is best effort
request.catch(() => {});

try {
	close();
}
// observability: swallowed because the resource is already closed
catch {}
```

The escape applies only to sites on the next line. A blank line breaks it. Comment text inside strings, templates, regular expressions, or block comments does not grant an escape. For a multiline chain, place the comment directly above the `.catch` line.

## Options

`no-unobserved-catch` and `no-swallowing-promise-catch` accept one optional object. Unknown keys are errors.

| Option      | Rules                 | Meaning                                                                                                                                                                             |
| ----------- | --------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ignore`    | Both                  | Unique glob strings matched against the repository-relative path, with forward slashes. Matching files are skipped by that rule. Defaults to `[]`.                                  |
| `observers` | `no-unobserved-catch` | Unique strings replacing the default observer list. Each is `name`, `Owner.member`, `Owner.*`, or `*.member`. An empty list leaves throws and propagation as the accepted evidence. |

```toml
[lint.rules]
"observability/no-unobserved-catch" = ["error", { ignore = ["**/*.test.ts", "packages/observability/**"], observers = ["reportException", "Logger.*", "*.reportException"] }]
"observability/no-swallowing-promise-catch" = ["error", { ignore = ["scripts/**"] }]
```

These options do not inherit the source check's repository-specific exclusions. Add any test, script, or reporter paths your repository needs to `ignore`.

## Catches outside failure boundaries

`observability/no-catch-outside-boundary` reports every catch clause and `.catch(handler)` call outside the configured failure boundaries. Reporting an error does not exempt a catch from this rule. It is off in the bundled policy.

Calls with a known non-callable argument and schema fallback chains rooted in an import binding from `zod`, `zod/v4`, or `zod/mini`, including lexical aliases, are excluded; a chain that passes through a Promise-returning method (`parseAsync`, `safeParseAsync`, `spa`, `then`, `finally`) is a rejection handler and still reports, as do unresolved Promise handlers.

By default, a catch block containing only a single `throw` statement is accepted anywhere. Rethrows and new errors, with or without a `cause`, count as translation. Promise handlers with a block containing only a single `throw` also qualify, using the same-file handler resolution from `no-swallowing-promise-catch`. Unresolved handlers are findings, including a handler reached through a binding that is written again after its declaration or through a member that is assigned, updated or deleted, directly or through an alias binding of the same object. Set `translate = false` to require even these catches to live at a boundary.

With `translate` enabled, a single `return` whose argument references the caught binding (or any name a destructured catch parameter binds) also counts as translation, including member reads, call or constructor arguments, and Promise arrow expression bodies; this is a lexical check with no type inference. Returns without that binding, bindingless catches, conditions, cleanup, logging, and bodies with multiple statements still report.

The [intentional-swallow comment](#allow-an-intentional-swallow) also exempts this rule, with the same token and next-line requirements. Findings use diagnostic code `observability/no-catch-outside-boundary` and message: “A catch outside a failure boundary: raise an OperationalFailure and let the edge observe it.”

The rule accepts one optional options object. Unknown keys and invalid option types are errors.

| Option       | Default | Meaning                                                                                                                                             |
| ------------ | ------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| `allow`      | `[]`    | Ordered repository-relative path globs. A positive match allows the file; a leading `!` re-includes it for checking. Last match wins.               |
| `boundaries` | unset   | `{ modules, names }`: a file that statically imports one of `names` from one of `modules` is a failure boundary, and every catch in it is accepted. |
| `translate`  | `true`  | Accept a single `throw` or a single return referencing the caught binding, including Promise arrow expression bodies.                               |

`allow` uses the [boundary rules' path glob semantics](boundaries.md#options), including Unicode matching, literal braces, and rejection of blank or malformed patterns and `{package}`. An empty list checks every file.

`boundaries` derives the failure boundaries from what a file imports, so a consumer does not maintain a second path list beside `boundaries/reserved-specifier`. `modules` and `names` are each required, non-empty arrays of unique, non-blank strings, and each name must be an identifier name; unknown keys inside `boundaries` are errors. A file counts when it contains a static `import` declaration whose source equals one of `modules` exactly, that is not type-only, and that has a named specifier whose imported name is one of `names`. The comparison uses the imported name, so `import { FailureBoundary as Boundary } from '@observability'` counts. Module names do not prefix-match: `@observability/runtime` does not match `@observability`. `import type { FailureBoundary }`, the inline `import { type FailureBoundary }`, namespace and default imports, `export { FailureBoundary } from`, `require(...)` and dynamic `import()` do not count. Import admission applies only to the authored `.ts` and `.tsx` files that `boundaries/reserved-specifier` checks, so a JavaScript, `.mts`, `.cts` or `.d.ts` file is never admitted by import and only `allow` can admit it. `allow` keeps its meaning and is evaluated first; a file passes if either admits it. Without `boundaries`, behaviour is unchanged. The import scan runs once per file.

```toml
[lint.rules]
"observability/no-catch-outside-boundary" = ["error", { allow = [
    "packages/observability/**",
    "**/worker.ts",
    "**/*.test.ts",
], translate = true }]
```

With `boundaries`, the `allow` list shrinks to the observability package, scripts and tests, and the rule accepts a catch wherever the file imports the boundary. `boundaries/reserved-specifier` stays the one place that decides who may import `FailureBoundary`, so a file that passes both rules is a boundary by construction.

```toml
[lint.rules]
"boundaries/reserved-specifier" = ["error", { rules = [
    { name = "failure-boundary", match = ["@observability", "@omniyat/observability"], allow = ["packages/*/src/edge/**"], message = "the failure boundary may be imported only by edge modules" },
] }]
"observability/no-catch-outside-boundary" = ["error", { allow = [
    "packages/observability/**",
    "**/scripts/**",
    "**/*.test.ts",
], boundaries = { modules = ["@observability", "@omniyat/observability"], names = ["FailureBoundary"] } }]
```
