# Performance rules

These native rules catch work that a function repeats on every call but a module could do once, and queries that read more than the caller uses or run once per item. They are off in the bundled policy. Enable the ones you want in `fmtkit.toml`:

```toml
[lint.rules]
"perf/no-regex-in-function" = "error"
"perf/no-stateful-regex-test" = "error"
"perf/no-repeated-url-parse" = "error"
"perf/no-select-all" = "error"
"perf/no-query-in-loop" = "error"
"perf/prefer-shared-pattern" = ["error", { module = "@acme/patterns", patterns = { WHITESPACE = '/\s+/u' } }]
```

Each rule reads one file at a time. None of them checks types, so each one matches what is written in the file.

## Regular expressions

`perf/no-regex-in-function` reports a regular expression that a function builds on every call. That covers a literal, or `new RegExp(…)` and `RegExp(…)` called only with strings or templates without substitutions. Hoist it to a module-level constant:

```ts
const WORD = /\w+/u;

export function words(text: string): string[] {
	return text.match(WORD) ?? [];
}
```

The rule leaves these alone:

- A pattern built from a variable.
- A class field.
- Test files (`*.test.*` and `*.spec.*`).
- A `g` or `y` literal held in a variable. Its `lastIndex` carries over between calls, so a fresh pattern per call is what lets an `exec` loop start from the beginning.

A `g` or `y` literal passed straight to a call, such as `text.replace(/a/gu, '')`, is still reported. `replace`, `split` and `matchAll` don't depend on the pattern's previous state, so it's safe to share.

`perf/no-stateful-regex-test` makes sharing patterns safe. It reports `test` or `exec` called on a pattern with the `g` or `y` flag: a literal, `new RegExp(source, flags)`, or a `const` in the same file bound to either. Such a pattern starts each call from the previous match's `lastIndex`, so a shared one gives different answers for the same input. Give `test` and `exec` a pattern without those flags. The rule can't see the flags of an imported pattern or a `let` binding, so it doesn't report them.

`perf/prefer-shared-pattern` reports a literal that a shared module already exports. Both options are required:

| Option     | Meaning                                                                                  |
| ---------- | ---------------------------------------------------------------------------------------- |
| `module`   | The import specifier named in the message, such as `@acme/patterns`.                     |
| `patterns` | Each exported name mapped to its literal exactly as written, flags included: `'/\s+/u'`. |

The declaration that defines a name, `export const WHITESPACE = /\s+/u;`, isn't reported. A literal only matches when its text is identical, so `/\s+/gu` doesn't match `/\s+/u`.

## URLs

`perf/no-repeated-url-parse` reports a second `new URL(…)` of the same input in the same function, or a second one at module level. Parse the input once and read every part you need from that `URL`:

```ts
const url = new URL(base);

return { origin: url.origin, host: url.hostname };
```

Inputs only match when they always read the same value: names, property reads, strings and templates without substitutions. `new URL(next())` never matches another parse. `new URL(path, base)` and `new URL(base)` are different inputs.

## Queries

`perf/no-select-all` reports Drizzle's `select()` and `selectDistinct()` before `.from(…)`, and `returning()` after `insert`, `update` or `delete`, when they're called with no column map. Without a map they read every column, including large `jsonb` and `text` columns the caller never reads. Pass the columns the caller uses:

```ts
await db.select({ id: users.id, email: users.email }).from(users);
```

`perf/no-query-in-loop` reports an awaited query inside the body of a `for`, `for…in`, `for…of`, `while` or `do…while` loop: the N+1 pattern. Here a query is an awaited call chain that starts from a receiver (`tx.select()…`) or from a receiver read off `this` (`this.db.update(t)…`). Read every item in one query before the loop, with `inArray`, a join, or one `sql` statement. These aren't reported:

- An await in the loop's iterable, such as `for (const row of await db.select(…))`.
- A query in a callback, such as `items.map(async …)`, because the callback is a new function.

| Option      | Default                              | Meaning                                                 |
| ----------- | ------------------------------------ | ------------------------------------------------------- |
| `receivers` | `["db", "transaction", "trx", "tx"]` | The receiver names to check. Replaces the default list. |

To allow an intentional loop, such as writes that must run in order, add an `oxlint-disable-next-line` comment that gives the reason.
