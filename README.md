# fmtkit

[![Tests](https://github.com/oullin/fmtkit/actions/workflows/tests.yml/badge.svg)](https://github.com/oullin/fmtkit/actions/workflows/tests.yml)
[![Release](https://github.com/oullin/fmtkit/actions/workflows/release.yml/badge.svg)](https://github.com/oullin/fmtkit/actions/workflows/release.yml)

One formatter and one gate for repositories that mix Go with TypeScript, JavaScript, Vue, HTML, and Markdown. fmtkit formats, applies the layout rules standard formatters skip, lints, scores complexity, and runs `go vet`, with one report and one exit code.

Coming from 0.x? Read [Migrating to 2.0](docs/migrating-to-2.0.md).

## Install

Every channel ships two binaries, `fmtkit` and `fmtkit-go-helper`. Keep them together.

```sh
# Homebrew
brew install oullin/fmtkit/fmtkit

# Docker (linux/amd64, linux/arm64; includes Go for go vet)
docker run --rm -u "$(id -u):$(id -g)" -v "$PWD:/work" ghcr.io/oullin/fmtkit:latest check --all

# From source (Rust 1.96+, Go 1.27.4)
git clone https://github.com/oullin/fmtkit.git && cd fmtkit && make build
```

Release archives (`fmtkit-<target>.tar.xz`) cover `aarch64`/`x86_64` on macOS and Linux. Put both binaries in one directory on your `PATH`; `make build` writes them to `storage/target/release/fmtkit` and `storage/go-helper/fmtkit-go-helper`.

fmtkit finds the helper through `FMTKIT_GO_HELPER`, then beside itself (as invoked, then with symlinks resolved), then in `../share/fmtkit/` (Homebrew), then on `PATH`. A missing or incompatible helper exits with code 3.

No Node.js is needed. `go` on `PATH` is needed only for `go vet` and `--resolve-imports`. Opt-in [type-aware rules](docs/type-aware.md) also need the native `tsrs` binary (`npm i -D tsrs`).

## Quickstart

Run it anywhere inside a Git repository:

```sh
fmtkit check          # report on changed files; write nothing
fmtkit format         # fix changed files, then report what is left
fmtkit check --all    # every file; use this in CI
fmtkit format --all   # fix every file
```

Changed means different from `HEAD` in the work tree or index, or untracked and not ignored. A clean tree checks nothing without `--all`.

## What it does

fmtkit changes layout and applies safe lint fixes; it does not change behaviour on purpose. A file whose syntax tree changes unexpectedly, that fails to parse, or that does not reach a fixed point is reported and left untouched.

**Go:** fmtkit's [spacing rule](docs/spacing.md) (blank lines around control flow and declarations, one item per line in multi-line composite literals, types moved to the top, `//go:embed` repair), then `gofmt`, then `goimports` (grouping and sorting only, unless `resolve_imports` is on), then the opt-in [naming checks](docs/naming.md#go), then complexity scoring and the [report-only nesting and line-length checks](#go-nesting-and-line-length), then `go vet` on the packages in scope (`./...` per module under `--all`).

**TypeScript and JavaScript:** Oxlint fixes, braces on unbraced bodies, class members ordered (properties, constructor, methods), short declarations before multiline ones, statement blank lines (including one between consecutive test blocks such as `describe`, `it`, and `beforeEach` calls), one property per line in objects with two or more properties (outside template literals), the oxc formatter, one call per line in fluent chains, a fixed layout for Drizzle queries, and one argument per line in calls and `new` expressions with a nested call, object, or array, or a ternary with one of those in a branch. The schedule repeats until the file stops changing.

```diff
-const rows = await db.select().from(users).where(eq(users.id, id));
-const limit = 10;
-function pick(n: number) { if (n > limit) return rows; return [] }
+const rows = await db.select()
+	.from(users)
+	.where(eq(users.id, id));
+
+const limit = 10;
+
+function pick(n: number) {
+	if (n > limit) {
+		return rows;
+	}
+
+	return [];
+}
```

**Vue, HTML, Markdown:** formatted as whole documents. Scripts inside them go through the script pipeline; styles and CSS fences through the oxc CSS formatter. Markdown prose is not rewrapped.

**Lint:** Oxlint (pinned to `oxlint_v1.94.0`) with a bundled policy of 221 rules, derived from [`@nkzw/oxlint-config`](https://github.com/nkzw-tech/oxlint-config) 3.0 with its sorting preset, including native `perfectionist/*`, `@nkzw/*`, `quality/*`, and `no-only-tests/*` rules. The [`anti-slop/*`, `anti-slop-martinfowler/*` and `anti-slop-effect/*` rules](docs/anti-slop.md) are available but off by default (twelve former `anti-slop/*` rules are now the bundled `quality/*` rules, and their old names still work as aliases); the last two flag code smells from Martin Fowler's _Refactoring_ and check Effect programs. The native [`boundaries/*` rules](docs/boundaries.md) provide opt-in TypeScript file, type, and module policies ported from Omniyat’s `oc` engine. `error` fails the run; `warn` does not. The native [observability catch rules](docs/observability-catches.md) are also off by default; enable them to require catch reporting or propagation and reject promise handlers that discard errors. The native [`perf/*` rules](docs/perf.md) are opt-in too: they flag regular expressions built on every call, repeated URL parsing, Drizzle reads of every column, and queries awaited inside loops. The opt-in [`naming/*` rules](docs/naming.md) report folders, files and declarations whose names start with what their path already says (`naming/no-redundant-name-prefix`), and declared names with more words than a limit (`naming/max-name-words`).

### What it leaves alone

- Files ignored by `.gitignore`, `.ignore`, or `[files] exclude`, and anything under `.git`, `node_modules`, or `vendor`.
- Symbolic links, `.d.ts`/`.d.mts`/`.d.cts`, `*.gen.go`, and files marked `// Code generated ... DO NOT EDIT.`
- JSON, YAML, TOML, standalone CSS, Vue custom blocks, Pug, Sass, Stylus, CoffeeScript, and Markdown fences in other languages.
- Tailwind class order.

## Usage

| Command               | What it does                                                          |
| --------------------- | --------------------------------------------------------------------- |
| `fmtkit format`       | Rewrite files in place, then report what is left.                     |
| `fmtkit check`        | Report what `format` would change and every finding. Write nothing.   |
| `fmtkit serve`        | Stay running so `format` and `check` start warm; see [Serve](#serve). |
| `fmtkit version`      | Print the version.                                                    |
| `fmtkit <cmd> --help` | Print a command's flags.                                              |

`format` and `check` share these flags:

| Flag                      | Meaning                                                                                 |
| ------------------------- | --------------------------------------------------------------------------------------- |
| `[PATHS]...`              | Cover these files or directories, changed or not.                                       |
| `--all`                   | Cover every tracked and untracked, non-ignored file.                                    |
| `--go` / `--ts`           | Run only one lane (`--ts` includes Vue, HTML, Markdown).                                |
| `-j`, `--jobs <N>`        | Worker threads, and the build processes `go vet` may start in all; 0 means one per CPU. |
| `--no-cache`              | Skip the cache.                                                                         |
| `--format <FORMAT>`       | `text` (default), `json`, or `agent`.                                                   |
| `-q`, `--quiet`           | Print only findings and the summary.                                                    |
| `--color <WHEN>`          | `auto` (default), `always`, or `never`.                                                 |
| `--stdin-filepath <PATH>` | Format stdin as if it were at `PATH`, for editors.                                      |
| `--resolve-imports`       | Let goimports add and remove imports (slow).                                            |

Outside Git, the root is the current directory and the default scope is every non-ignored file. A path outside the root is a usage error; a missing path fails the run.

With `--stdin-filepath`, `format` writes the result to stdout and `check` exits 1 if formatting would change it. Stdin is formatted only, not scored or fully linted.

```sh
printf 'const a = {b:1}\n' | fmtkit format --stdin-filepath web/x.ts
```

### Environment

| Variable           | Effect                                                     |
| ------------------ | ---------------------------------------------------------- |
| `FMTKIT_CONFIG`    | Config file path, relative to the root.                    |
| `FMTKIT_JOBS`      | Worker threads when `--jobs` is not given.                 |
| `FMTKIT_CACHE_DIR` | Cache directory (default: the OS cache dir + `fmtkit/v2`). |
| `FMTKIT_GO_HELPER` | Path of `fmtkit-go-helper`.                                |
| `FMTKIT_TSRS`      | Path of the native `tsrs` binary, for type-aware rules.    |
| `NO_COLOR`         | Disable colour under `--color auto`.                       |

### Cache

fmtkit caches each file's outcome, keyed by its bytes, path, the fmtkit version, the configuration, and the mode. Like git's index, it trusts a file's size, inode, and times and skips unchanged files without reading them. It also remembers directory listings and clean `go vet` runs per module. `--no-cache` bypasses all of it for one run.

### Serve

`fmtkit serve` keeps a repository's cache, linter, workers, and git state loaded. While it runs, `format` and `check` in that repository hand their work to it and finish in a few milliseconds. Results are identical to a local run.

```sh
fmtkit serve           # in its own terminal
fmtkit serve --watch   # also watch the tree (kqueue on macOS, inotify on Linux)
fmtkit serve --idle 30m   # stop after half an hour without a call
```

- One server covers every work tree of its repository (`git worktree`): start it in any of them, and calls from the others reach it. It keeps each work tree's cache and git state apart, shares one worker pool and one linter, and forgets a work tree once it is removed.

- With `--watch`, each run looks only at what changed since the previous one. A flood of changes or dropped kernel events makes the next run look at everything. If the watch hits the open-file limit (macOS) or `fs.inotify.max_user_watches` (Linux), the server logs why and continues without it.
- The server runs one request at a time, and refuses requests from another fmtkit version or a shell whose relevant environment differs (`FMTKIT_*`, `GIT_*`, `GO*`, `CGO_*`, `HOME`, `PATH`, `XDG_*`). Refused runs simply run locally.
- It stops when its executable is replaced, when the repository is removed, or with `--idle`, after that long without a call. `--no-cache` and `--stdin-filepath` always run locally.
- It listens on an owner-only Unix socket beside the cache. A second server for the same repository exits with code 2. Not available on Windows.

## Configuration

fmtkit reads `fmtkit.toml` at the repository root (or `FMTKIT_CONFIG`). Every key is optional; unknown keys are errors. These are the defaults:

```toml
jobs = 0                              # 0 = one worker per CPU; also caps the build processes go vet starts

[files]
exclude = ["node_modules/", "vendor/"] # gitignore-style; replaces the default list

[go]
spacing = true
gofmt = true
goimports = true
resolve_imports = false
vet = true
max_name_words = false                # naming/max-name-words for Go declarations
max_words = 2                         # its limit for package-level names, methods and fields
# max_local_words = 1                 # its limit for locals and parameters; unset = max_words
no_package_stutter = false            # naming/no-package-stutter: worker.WorkerSpec
max_depth = 4                         # go/max-depth; 0 disables
max_nested_funcs = 3                  # go/max-nested-funcs; 0 disables
max_line_length = 120                 # go/line-length; a tab counts as ts.format.tab_width; 0 disables

[ts.format]                           # also applies to Vue, HTML, Markdown, CSS
use_tabs = true
tab_width = 4
print_width = 200
single_quote = true
semi = true
trailing_comma = "all"                # "all", "es5", "none"
arrow_parens = "always"               # "always", "avoid"

[lint]
bundled = true                        # false = start from no rules
ignore = []                           # files lint skips (still formatted)
# exemptions = "lint-exempt.toml"     # [[exempt]] entries, relative to this file
# type_aware = "nkzw"                 # nkzw's ten type-aware rules, run through tsrs
# tsrs = "node_modules/@tsrs/darwin-arm64/tsrs"   # where tsrs is, if fmtkit cannot find it

[lint.rules]
eqeqeq = ["error", "always"]          # severity, or [severity, options...]
"quality/no-object-parameters" = "warn"
"unused-directive" = "warn"              # default "error"; "off" stops the reports

[complexity]
cyclomatic = 15                       # 0 disables
cognitive = 20
```

A rule setting replaces the bundled one entirely, options included, so repeat the options when you only change the severity. Severities are `off`/`allow`, `warn`, and `error`/`deny`.

The bundled policy reports a disable directive (`// oxlint-disable-next-line rule`, or its `eslint-` form) that suppresses nothing as an `unused-directive` error, as `@nkzw/oxlint-config` 3.0 does. `fix` removes an unused `oxlint-` directive; it leaves an `eslint-` directive in place, because ESLint may still read it. Configure it as the rule `unused-directive` under `[lint.rules]` (`"warn"`, or `"off"` to stop the reports) or exempt files from it under `[[exempt]]`; it takes no options. Without the bundled policy it is off until you turn it on.

Type-aware rules (oxc's tsgolint rules, such as `typescript/no-floating-promises`) need the TypeScript type checker. They are opt-in: name one under `[lint.rules]`, or set `[lint] type_aware = "nkzw"` for the ten that `@nkzw/oxlint-config` 3.0 enables, which the bundled policy leaves out. fmtkit then runs them once per run through the native [tsrs](https://www.npmjs.com/package/tsrs) binary (`npm i -D tsrs`) and reports them like any other finding. Directives, `[[exempt]]` entries, and `[lint] ignore` apply to them. A run that needs tsrs and cannot find it exits with code 2 and says how to install it. These findings are not cached and have no fixes; see [Type-aware rules](docs/type-aware.md).

## Lint exemptions

An exemption turns one rule off for the files its patterns match; every other rule still applies to those files. It replaces a disable comment above each finding when a rule does not fit a whole set of files. List the entries in a file named by `[lint] exemptions`, resolved relative to the directory of the configuration file:

```toml
# fmtkit.toml
[lint]
exemptions = "lint-exempt.toml"
```

```toml
# lint-exempt.toml
[[exempt]]
rule = "eslint/require-await"
reason = "`async` keeps the Promise contract, so a throw reaches the caller as a rejection."
paths = ["**/*.test.ts", "e2e/**", "apps/api/src/worker/fetch.ts"]
```

The same entries may also be written inline as `[[lint.exempt]]` in `fmtkit.toml`; both lists apply.

- `rule` is a rule name as `[lint.rules]` accepts it, for oxlint rules and fmtkit's own rules alike. `reason` and `paths` are required and must not be empty, and unknown keys are errors.
- `paths` are gitignore-style patterns matched against repository-relative paths, as `[lint] ignore` is: a pattern without a slash matches at any depth, and a directory covers everything below it.
- An unknown rule, or a rule that is off, is a configuration error (exit 2). So is an exemptions file that is missing or does not parse; the message names the file and the line.
- A finding the entry exempts is dropped together with its fix.
- A run over the whole repository (`--all` with no paths) reports each exact path that does not exist as a `lint/exempt` error, so the list only shrinks. An exact path contains a slash before its last segment, as `apps/api/src/worker/fetch.ts` or `/fetch.ts` do, and no `*`, `?`, `[`, `{`, `\` or leading `!`. Glob patterns and slash-free names, which match at any depth, are not judged.

## Complexity

fmtkit scores every function in Go and script files (not Vue, `.mjs`, `.cjs`, or tests) and fails when a score exceeds its limit. Go uses gocyclo and gocognit; scripts use fmtkit's own scorer.

```text
[complexity/cognitive] line 7: web/store.ts#grade scores 37 (limit 20)
```

A key is `path#name`: `store/store.go#(*Store).Get`, `web/store.ts#Store.get size`, or `web/handlers.ts#handler:6` for a repeated name. Anonymous functions count toward the function around them.

Exempt a function with a reason:

```toml
[[complexity.allow]]
key = "web/store.ts#grade"
reason = "One branch per grade band; becomes a lookup table."
```

An entry that matches no function fails the run, so the list only shrinks. The bundled `eslint/complexity` rule (max 12) is separate; turn it off under `[lint.rules]` if you want one limit.

### Go nesting and line length

The bundled script policy enforces `max-depth` 4 and `max-nested-callbacks` 3, and the script printer wraps long lines. `gofmt` never wraps, so the Go lane reports three lint findings instead. They never change the text and fail file-based `check` and `format` runs, as script lint errors do. They apply to test files too. Stdin runs (`--stdin-filepath`, for editors) skip them. In `check`, a file that formatting would change is checked again on the text on disk only when the formatted text had findings, so a long line that formatting would shorten isn't reported.

```text
[go/max-depth] line 12:6: Blocks are nested too deeply (5). Maximum allowed is 4.
[go/max-nested-funcs] line 4:53: Too many nested function literals (4). Maximum allowed is 3.
[go/line-length] line 30:121: This line has a length of 131. Maximum allowed is 120.
```

- **`go/max-depth`** (`[go] max_depth`, default 4) counts levels as ESLint's `max-depth` does. Each `if`, `for`, `range`, `switch`, type switch, and `select` opens one level; their `case` clauses belong to it, and an `else if` stays on the level of the `if` it continues. A function literal starts its own count. Every statement past the limit is reported.
- **`go/max-nested-funcs`** (`[go] max_nested_funcs`, default 3) mirrors `max-nested-callbacks`. A function literal passed as a call argument has depth 1 plus the number of such literals around it, so `run(func() { scope(func() { metrics(func() { route(func() {}) }) }) })` reports the literal passed to `route`. Literals that are not arguments, such as `defer func() { … }()` and `go func() { … }()`, neither count nor reset the count.
- **`go/line-length`** (`[go] max_line_length`, default 120) counts runes, and a tab advances to the next multiple of `[ts.format] tab_width` (default 4), so both lanes measure a line alike. The column points at the first character past the limit. It skips lines that cannot reasonably be shortened: `import` lines whose overflow is the import itself (a declaration or comment after it on the same line is still checked), `//go:` directives, comment lines whose overflow is one URL, and lines whose overflow sits inside one string literal or struct tag, followed by nothing but `)`, `]`, `}`, `,` or `;`.

Set a limit to 0 to turn its check off. In check mode the findings point at the file on disk; in format mode, at the formatted text that was written.

All three are on by default since 2.6.0, so a Go repository that passed on 2.5 can fail after upgrading. Raise the limits, or set them to 0, to stay green while the code catches up.

## Output

`text` (default) groups findings by file. `agent` prints one `path[:line[:column]] rule message` line per finding plus a summary line, for agents and `grep`. `json` prints one object with `"schema": 2`:

```text
main.go:7 spacing missing blank line before type definition
web/store.ts:23:38 eslint/eqeqeq Expected === and instead saw ==
fmtkit schema=2 mode=check result=fail files=2 changed=2 violations=5 lint=5 complexity=2 vet=0 errors=0 missing=0
```

| Exit | Meaning                                                          |
| ---- | ---------------------------------------------------------------- |
| 0    | Clean, or `format` fixed everything.                             |
| 1    | Findings remain (including a missing path). Warnings never fail. |
| 2    | Usage or configuration error.                                    |
| 3    | Internal error, such as a missing or incompatible helper.        |

## Development

Needs Rust 1.96+, Go 1.27.4, and Bash. Build output goes to `storage/`.

| Target        | What it does                                          |
| ------------- | ----------------------------------------------------- |
| `make build`  | Build fmtkit and the Go helper.                       |
| `make format` | Format the repository (or `ARGS`) with fmtkit itself. |
| `make check`  | Check the repository (or `ARGS`) without writing.     |
| `make lint`   | rustfmt, clippy, gofmt, and `go vet`, read-only.      |
| `make test`   | Rust and Go tests.                                    |

`scripts/task.sh` also provides `self-check` (format the tree with `--all --no-cache` and fail if anything moves) and `coverage` (90% line coverage for Rust and Go). CI runs lint, tests on Ubuntu and macOS, coverage, smoke tests, the self-check, a benchmark gate (more than 10% slower fails), and nightly fuzzing.

The code lives in `crates/` (cli, engine, discover, config, core, ts, hosts, lint, go, cache, report, testkit) and `go/helper`. Only the engine writes source files. The workspace forbids unsafe code, denies pedantic clippy, and never panics on input. See also [Upgrading oxc](docs/oxc-upgrade.md), [Known upstream issues](docs/known-issues.md), [Type-aware rules](docs/type-aware.md), and `go/helper/proto/PROTOCOL.md`.

**Releases:** a merge to `main` with an untagged version in `Cargo.toml` is tagged and released (archives, Homebrew, Docker) only while the repository variable `RELEASES` is `enabled`.

## License

[MIT](LICENSE)
