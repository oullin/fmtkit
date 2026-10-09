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

# From source (Rust 1.96+, Go 1.27.1)
git clone https://github.com/oullin/fmtkit.git && cd fmtkit && make build
```

Release archives (`fmtkit-<target>.tar.xz`) cover `aarch64`/`x86_64` on macOS and Linux. Put both binaries in one directory on your `PATH`; `make build` writes them to `storage/target/release/fmtkit` and `storage/go-helper/fmtkit-go-helper`.

fmtkit finds the helper through `FMTKIT_GO_HELPER`, then beside itself (as invoked, then with symlinks resolved), then in `../share/fmtkit/` (Homebrew), then on `PATH`. A missing or incompatible helper exits with code 3.

No Node.js is needed. `go` on `PATH` is needed only for `go vet` and `--resolve-imports`.

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

**Go:** fmtkit's [spacing rule](docs/spacing.md) (blank lines around control flow and declarations, types moved to the top, `//go:embed` repair), then `gofmt`, then `goimports` (grouping and sorting only, unless `resolve_imports` is on), then complexity scoring, then `go vet` on the packages in scope (`./...` per module under `--all`).

**TypeScript and JavaScript:** Oxlint fixes, braces on unbraced bodies, class members ordered (properties, constructor, methods), short declarations before multiline ones, statement blank lines, the oxc formatter, one call per line in fluent chains, a fixed layout for Drizzle queries, and one argument per line in calls with nested calls, objects, or arrays. The schedule repeats until the file stops changing.

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

**Lint:** Oxlint (pinned to `oxlint_v1.86.0`) with a bundled policy of 197 rules, including native `perfectionist/*`, `@nkzw/*`, and `no-only-tests/*` rules. The `anti-slop/*` rules are available but off by default. `error` fails the run; `warn` does not.

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

| Flag                      | Meaning                                                  |
| ------------------------- | -------------------------------------------------------- |
| `[PATHS]...`              | Cover these files or directories, changed or not.        |
| `--all`                   | Cover every tracked and untracked, non-ignored file.     |
| `--go` / `--ts`           | Run only one lane (`--ts` includes Vue, HTML, Markdown). |
| `-j`, `--jobs <N>`        | Worker threads; 0 means one per CPU.                     |
| `--no-cache`              | Skip the cache.                                          |
| `--format <FORMAT>`       | `text` (default), `json`, or `agent`.                    |
| `-q`, `--quiet`           | Print only findings and the summary.                     |
| `--color <WHEN>`          | `auto` (default), `always`, or `never`.                  |
| `--stdin-filepath <PATH>` | Format stdin as if it were at `PATH`, for editors.       |
| `--resolve-imports`       | Let goimports add and remove imports (slow).             |

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
| `NO_COLOR`         | Disable colour under `--color auto`.                       |

### Cache

fmtkit caches each file's outcome, keyed by its bytes, path, the fmtkit version, the configuration, and the mode. Like git's index, it trusts a file's size, inode, and times and skips unchanged files without reading them. It also remembers directory listings and clean `go vet` runs per module. `--no-cache` bypasses all of it for one run.

### Serve

`fmtkit serve` keeps a repository's cache, linter, workers, and git state loaded. While it runs, `format` and `check` in that repository hand their work to it and finish in a few milliseconds. Results are identical to a local run.

```sh
fmtkit serve           # in its own terminal
fmtkit serve --watch   # also watch the tree (kqueue on macOS, inotify on Linux)
```

- With `--watch`, each run looks only at what changed since the previous one. A flood of changes or dropped kernel events makes the next run look at everything. If the watch hits the open-file limit (macOS) or `fs.inotify.max_user_watches` (Linux), the server logs why and continues without it.
- The server runs one request at a time, and refuses requests from another fmtkit version or a shell whose relevant environment differs (`FMTKIT_*`, `GIT_*`, `GO*`, `CGO_*`, `HOME`, `PATH`, `XDG_*`). Refused runs simply run locally.
- It stops when its executable is replaced. `--no-cache` and `--stdin-filepath` always run locally.
- It listens on an owner-only Unix socket beside the cache. A second server for the same repository exits with code 2. Not available on Windows.

## Configuration

fmtkit reads `fmtkit.toml` at the repository root (or `FMTKIT_CONFIG`). Every key is optional; unknown keys are errors. These are the defaults:

```toml
jobs = 0                              # 0 = one worker per CPU

[files]
exclude = ["node_modules/", "vendor/"] # gitignore-style; replaces the default list

[go]
spacing = true
gofmt = true
goimports = true
resolve_imports = false
vet = true

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

[lint.rules]
eqeqeq = ["error", "always"]          # severity, or [severity, options...]
"anti-slop/no-object-parameters" = "warn"

[complexity]
cyclomatic = 15                       # 0 disables
cognitive = 20
```

A rule setting replaces the bundled one entirely, options included, so repeat the options when you only change the severity. Severities are `off`/`allow`, `warn`, and `error`/`deny`.

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

Needs Rust 1.96+, Go 1.27.1, and Bash. Build output goes to `storage/`.

| Target        | What it does                                          |
| ------------- | ----------------------------------------------------- |
| `make build`  | Build fmtkit and the Go helper.                       |
| `make format` | Format the repository (or `ARGS`) with fmtkit itself. |
| `make check`  | Check the repository (or `ARGS`) without writing.     |
| `make lint`   | rustfmt, clippy, gofmt, and `go vet`, read-only.      |
| `make test`   | Rust and Go tests.                                    |

`scripts/task.sh` also provides `self-check` (format the tree with `--all --no-cache` and fail if anything moves) and `coverage` (90% line coverage for Rust and Go). CI runs lint, tests on Ubuntu and macOS, coverage, smoke tests, the self-check, a benchmark gate (more than 10% slower fails), and nightly fuzzing.

The code lives in `crates/` (cli, engine, discover, config, core, ts, hosts, lint, go, cache, report, testkit) and `go/helper`. Only the engine writes source files. The workspace forbids unsafe code, denies pedantic clippy, and never panics on input. See also [Upgrading oxc](docs/oxc-upgrade.md), [Known upstream issues](docs/known-issues.md), and `go/helper/proto/PROTOCOL.md`.

**Releases:** a merge to `main` with an untagged version in `Cargo.toml` is tagged and released (archives, Homebrew, Docker) only while the repository variable `RELEASES` is `enabled`.

## License

[MIT](LICENSE)
