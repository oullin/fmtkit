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

By default, a catch block containing only a single `throw` statement is accepted anywhere. Rethrows and new errors, with or without a `cause`, count as translation. Promise handlers with a block containing only a single `throw` also qualify, using the same-file handler resolution from `no-swallowing-promise-catch`. Unresolved handlers are findings, including a handler identifier that is written again after its declaration. Set `translate = false` to require even these catches to live at a boundary.

The [intentional-swallow comment](#allow-an-intentional-swallow) also exempts this rule, with the same token and next-line requirements. Findings use diagnostic code `observability/no-catch-outside-boundary` and message: “A catch outside a failure boundary: raise an OperationalFailure and let the edge observe it.”

The rule accepts one optional options object. Unknown keys and invalid option types are errors.

| Option      | Default | Meaning                                                                                                                               |
| ----------- | ------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| `allow`     | `[]`    | Ordered repository-relative path globs. A positive match allows the file; a leading `!` re-includes it for checking. Last match wins. |
| `translate` | `true`  | Accept a catch or resolved promise handler whose block contains only one `throw` statement.                                           |

`allow` uses the [boundary rules' path glob semantics](boundaries.md#options), including Unicode matching, literal braces, and rejection of blank or malformed patterns and `{package}`. An empty list checks every file.

```toml
[lint.rules]
"observability/no-catch-outside-boundary" = ["error", { allow = [
    "packages/observability/**",
    "**/worker.ts",
    "**/*.test.ts",
], translate = true }]
```
