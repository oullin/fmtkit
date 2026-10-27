# TypeScript boundary rules

The native `boundaries/*` rules port the single-file checks from Omniyat's `oc verify boundaries`. All ten rules are off in the bundled policy. Enable them under `[lint.rules]` in `fmtkit.toml`.

## Rules

| Rule | Refuses |
| --- | --- |
| `boundaries/exported-class-count` | More than one top-level exported class declaration outside a path ending in `/index.ts`. Reports once, at the second class, with the total. Default-exported classes count. Local classes, class expressions, re-exports, and classes inside namespaces do not. |
| `boundaries/barrel-only` | The first statement other than an import or export in a path ending in `/index.ts`. Exported declarations such as `export const`, classes, and functions are refused. Re-exports, `export default expression`, and `export = expression` are accepted. Directives and empty statements are refused. |
| `boundaries/relative-module` | Module specifiers beginning with `./` or `../`. |
| `boundaries/reserved-internal-name` | Each configured name appearing as a case-insensitive substring of the repository-relative path or an identifier. String contents, quoted property keys, comments, and private identifiers do not count. |
| `boundaries/dynamic-property-access` | Calls to `Reflect.get` and `Reflect.set`, including optional calls. Computed properties such as `Reflect['get']`, other reflection methods, and ordinary element access are accepted. |
| `boundaries/explicit-any` | Every explicit `any` type keyword. `unknown` and text containing the word are accepted. |
| `boundaries/runtime-typeof` | Runtime `typeof` expressions. Type queries such as `type Value = typeof value` are accepted. |
| `boundaries/zod-infer` | Type references to exactly `z.infer`, `z.input`, or `z.output`, and declaration-level type-only named imports whose imported or local name is `infer`, from any module. Inline `import { type infer }` and namespace imports are accepted. |
| `boundaries/nullable-union` | Each union with a direct `null` or `undefined` member. A lone nullish type, a parenthesised member such as `string \| (null)`, and nullish types inside objects are accepted. Nested unions are checked separately. |
| `boundaries/reserved-specifier` | Imports matching a configured module prefix outside its allowed files, unless the module matches an exception. |

Module specifiers come from static imports, re-exports, external `import = require(...)` declarations, bare `require(...)` calls, and dynamic `import(...)`. String literals and templates without substitutions count. Other strings and type-level `import('module')` queries do not. A shadowed `require` still counts because the source check uses syntax rather than name resolution.

These checks retain the source extractor's treatment of parentheses. For example, `require(('./value'))`, `(require)('./value')`, `(Reflect).get(value, key)`, and `(Reflect.get)(value, key)` are accepted.

## Options

Every rule accepts at most one options object. Unknown keys and invalid option types are errors. Boundary rules report findings without changing source.

| Option | Rules | Meaning |
| --- | --- | --- |
| `exempt` | All | Ordered path globs, default `[]`. A matching positive entry skips the file. A leading `!` re-includes it. The last matching entry wins. |
| `names` | `reserved-internal-name` | Required, non-empty list of reserved strings. Messages preserve each configured spelling. |
| `rules` | `reserved-specifier` | Required, non-empty list of specifier entries. Each entry has `name`, `match`, optional `except`, optional `allow`, and `message`. |

Path globs match the whole repository-relative path, with forward slashes. `*` stays within a segment, `**` spans zero or more directories, and `?` and character classes match Unicode characters. Classes use Go’s `[^a]` negation; `[!a]` contains the literal characters `!` and `a`. A bare filename matches that root file; use `**/filename.ts` for any directory. Braces are literal. Blank and malformed patterns are errors.

`{package}` is rejected in `exempt` and `allow`. Expand it to each registered package directory before configuring fmtkit. There is no workspace registry in the linter.

Each specifier entry requires a unique, non-blank `name`, a non-empty `match` list, and a non-blank `message`. Built-in boundary rule names are reserved. `match` and `except` contain module names, not globs: `effect` matches `effect` and `effect/Layer`, but does not match `effection`. `allow` has the same ordered path semantics as `exempt`. An outer `exempt` skips every specifier entry for that file.

Configured findings use `boundaries/reserved-specifier/<name>` as their diagnostic code and preserve `message` verbatim. Severity and suppression use the configured `boundaries/reserved-specifier` rule. Standard token-backed comments such as `// eslint-disable-next-line boundaries/reserved-specifier` work; comment-like text in strings does not suppress findings.

## Translate Collections configuration

This example translates Collections' `cloud/boundaries.yml`, including its twelve specifier entries. It expands `{package}` using that consumer's registered directories and appends its two nullable-union allowlist files to `exempt`. Keep the reasons beside those entries when maintaining your own configuration.

The `lint.ignore` list retains the source roots derived from `cloud/oc.yml`. fmtkit still formats files outside those roots; the list scopes linting only. Update the roots and expanded package directories when the workspace registry changes.

```toml
[lint]
bundled = false
ignore = [
	"**",
	"!apps/**",
	"!connector/**",
	"!database/**",
	"!handover/**",
	"!identity/**",
	"!living/**",
	"!packages/**",
	"!portfolio/**",
	"!post-handover/**",
	"!pre-handover/**",
	"!vitest.cloudflare.config.ts",
	"!vitest.config.ts",
	"!vitest.coverage.config.ts",
	"!vitest.migrate.config.ts",
	"!vitest.runtime-coverage.config.ts",
]

[lint.rules]
"boundaries/exported-class-count" = "error"
"boundaries/barrel-only" = "error"
"boundaries/relative-module" = "error"
"boundaries/reserved-internal-name" = ["error", { names = ["lifecycle"] }]
"boundaries/dynamic-property-access" = "error"
"boundaries/explicit-any" = "error"
"boundaries/runtime-typeof" = ["error", { exempt = ["packages/primitives/src/json/json-record.ts"] }]
"boundaries/zod-infer" = "error"
"boundaries/nullable-union" = ["error", { exempt = [
	"**/*.test.ts",
	"**/*.test.tsx",
	"**/*.spec.ts",
	"**/*.spec.tsx",
	"**/*-dto.ts",
	"**/*-dto.tsx",
	"packages/*/src/contracts/**",
	"connector/core/contracts/**",
	"connector/plugins/*/*/contracts/**",
	"!packages/workspace/src/contracts/**",
	"living/src/contracts/**",
	"portfolio/src/contracts/**",
	"pre-handover/src/contracts/**",
	"handover/src/contracts/**",
	"post-handover/src/contracts/**",
	"identity/src/contracts/**",
	"packages/testing-api/src/**",
	"apps/tests/**",
	"handover/tests/**",
	"identity/tests/**",
	"living/tests/**",
	"portfolio/tests/**",
	"post-handover/tests/**",
	"pre-handover/tests/**",
	"packages/cache/src/**/dto/**",
	"packages/workflow/src/**/dto/**",
	"packages/container/src/**/dto/**",
	"packages/providers/src/**/dto/**",
	"packages/http/src/**/dto/**",
	"packages/primitives/src/**/dto/**",
	"packages/cloudflare-access/src/**/dto/**",
	"packages/kernel/src/**/dto/**",
	"packages/env/src/**/dto/**",
	"database/src/**/dto/**",
	"packages/auth/src/**/dto/**",
	"packages/gate/src/**/dto/**",
	"packages/access/src/**/dto/**",
	"packages/flags/src/**/dto/**",
	"packages/assistant/src/**/dto/**",
	"packages/devices/src/**/dto/**",
	"packages/twilio/src/**/dto/**",
	"packages/notifications/src/**/dto/**",
	"living/src/**/dto/**",
	"portfolio/src/**/dto/**",
	"pre-handover/src/**/dto/**",
	"handover/src/**/dto/**",
	"post-handover/src/**/dto/**",
	"identity/src/**/dto/**",
	"packages/offerings/src/**/dto/**",
	"connector/src/**/dto/**",
	"packages/events/src/**/dto/**",
	"packages/observability/src/**/dto/**",
	"packages/testing/src/**/dto/**",
	"packages/testing-faker/src/**/dto/**",
	"packages/testing-api/src/**/dto/**",
	"apps/api/src/**/dto/**",
	"connector/api/src/**/dto/**",
	"packages/migrate/src/**/dto/**",
	"apps/tests/src/**/dto/**",
	"packages/workspace/src/**/dto/**",
	"connector/**/dto/**",
	"packages/primitives/src/nullable/**",
	# Owned by the server notifications work.
	"packages/notifications/src/read/service.ts",
	# The optional-secret reveal helper is part of the Redacted API surface.
	"packages/primitives/src/redacted.ts",
] }]
"boundaries/reserved-specifier" = ["error", { rules = [
	{ name = "framework-cache", match = ["@omniyat-collection/framework/cache"], allow = ["packages/cache/**"], message = "@omniyat-collection/framework/cache may be imported only by packages/cache" },
	{ name = "framework-container", match = ["@omniyat-collection/framework/container"], allow = ["packages/container/**"], message = "@omniyat-collection/framework/container may be imported only by packages/container" },
	{ name = "framework-httpx", match = ["@omniyat-collection/framework/httpx"], except = ["@omniyat-collection/framework/httpx/routing/navigator"], allow = ["packages/http/**"], message = "@omniyat-collection/framework/httpx may be imported only by packages/http" },
	{ name = "framework-money", match = ["@omniyat-collection/framework/support/money"], allow = ["packages/kernel/src/money/money.ts"], message = "@omniyat-collection/framework/support/money may be imported only by packages/kernel/src/money/money.ts" },
	{ name = "framework-navigator", match = ["@omniyat-collection/framework/httpx/routing/navigator"], allow = ["packages/kernel/**"], message = "@omniyat-collection/framework/httpx/routing/navigator may be imported only by packages/kernel" },
	{ name = "framework-phone", match = ["@omniyat-collection/framework/support/phone"], allow = ["packages/auth/src/client/phone/**", "packages/testing-faker/src/**"], message = "@omniyat-collection/framework/support/phone may be imported only by packages/auth/src/client/phone and packages/testing-faker" },
	{ name = "framework-workflow", match = ["@omniyat-collection/framework/workflow"], allow = ["packages/workflow/**"], message = "@omniyat-collection/framework/workflow may be imported only by packages/workflow" },
	{ name = "better-result", match = ["better-result"], allow = ["packages/primitives/src/result.ts"], message = "better-result may be imported only by packages/primitives/src/result.ts" },
	{ name = "effect", match = ["effect", "@effect"], allow = ["packages/primitives/src/effect/**"], message = "effect may be imported only under packages/primitives/src/effect/" },
	{ name = "ai-sdk", match = ["ai", "@ai-sdk"], allow = ["packages/assistant/**"], message = "the AI SDK may be imported only by packages/assistant/" },
	{ name = "twilio", match = ["@twilio", "@omniyat/twilio", "twilio"], allow = ["packages/twilio/**", "packages/providers/**", "**/*.test.ts", "**/*.test.tsx"], message = "Twilio implementations may be imported only by provider composition" },
	{ name = "twilio-persistence", match = ["@db", "@omniyat/database", "drizzle-orm"], allow = ["**", "!packages/twilio/**", "**/*.test.ts", "**/*.test.tsx"], message = "Twilio adapters must emit consumer-owned results instead of accessing persistence" },
] }]
```

An `oc` allowlist entry becomes a final `exempt` path for its built-in rule. For a specifier entry, append the path to that entry's `allow` list so other entries continue to run. fmtkit does not accept `allowlist` or `frozen` options.

## Scope and parser differences

The rules check authored `.ts` and `.tsx` files and skip `.d.ts`, matching the source extractor. fmtkit discovers files independently of the `oc` workspace registry; use `lint.ignore` or `files.exclude` to retain the same source roots. JavaScript files and embedded scripts in other file types do not run these rules.

Both implementations inspect syntax without TypeScript type checking. oxc preserves the valid syntax used by these checks, including directives, optional calls, escaped identifiers, and parenthesised types. No detection difference was found in the ported fixtures. Malformed TypeScript differs: the compiler API can recover an AST and emit boundary facts, while fmtkit reports a syntax error and does not run native rules over that file. fmtkit also uses its standard line, column, and diagnostic sorting rather than the extractor's pre-order finding order.

Two checks remain in `oc`: `registered-workspace` needs the workspace registry to establish file ownership, and `undeclared-workspace-dependency` needs that registry plus each package's `package.json`. The frozen-allowlist check also stays there because it compares configuration against a Git baseline. None belongs in a single-file syntax rule.
