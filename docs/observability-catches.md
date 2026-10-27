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

Both rules accept one optional object. Unknown keys are errors.

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
