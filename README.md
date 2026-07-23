# fmtkit

[![Go Reference](https://pkg.go.dev/badge/go.ollin.sh/fmtkit/driver.svg)](https://pkg.go.dev/go.ollin.sh/fmtkit/driver)
[![Go 1.26.4](https://img.shields.io/badge/go-1.26.4-00ADD8?logo=go&logoColor=white)](https://go.dev/doc/go1.26)
[![Tests](https://github.com/oullin/fmtkit/actions/workflows/tests.yml/badge.svg)](https://github.com/oullin/fmtkit/actions/workflows/tests.yml)
[![Release](https://github.com/oullin/fmtkit/actions/workflows/release.yml/badge.svg)](https://github.com/oullin/fmtkit/actions/workflows/release.yml)
[![Codecov](https://codecov.io/gh/oullin/fmtkit/graph/badge.svg?branch=main)](https://app.codecov.io/github/oullin/fmtkit)

`fmtkit` is a rule-driven formatter for Go. It enforces layout and structure that `gofmt` leaves alone — blank lines around control flow, type hoisting, declaration grouping — then hands off to `gofmt` and `goimports` for the final pass.

## At a glance

- AST-based spacing rules, then `gofmt` and `goimports`, in one deterministic pipeline.
- Runs `go vet ./...` automatically when invoked inside a Go module or workspace.
- Three output modes: `text` for humans, `json` for scripts, `agent` for CI and AI tools.
- One self-contained `fmtkit` binary (Homebrew or GitHub Releases), or a Go-only CLI (`fmtkit-go`).
- Engine in [`packages/go/formatter/engine`](packages/go/formatter/engine) is importable from Go.

## Install

Two ways to run it. Pick the one that fits your workflow — both produce identical output.

**With Homebrew** (recommended: one self-contained binary with the full TS/Vue + Go pipeline — no Node.js required):

```bash
brew tap oullin/fmtkit
brew install --cask fmtkit
fmtkit format .
```

The binary embeds the TS toolchain (oxfmt, oxlint, oxc-parser and the support scripts, compiled with Bun) and extracts it to your user cache directory on first run. Homebrew casks are macOS-only; on Linux, download the same binary from GitHub Releases instead:

```bash
tag=$(curl -fsSLI -o /dev/null -w '%{url_effective}' https://github.com/oullin/fmtkit/releases/latest | sed 's#.*/##')
curl -fsSL "https://github.com/oullin/fmtkit/releases/download/${tag}/fmtkit_${tag#v}_linux_amd64.tar.gz" | tar -xz fmtkit
sudo install -m 0755 fmtkit /usr/local/bin/fmtkit
```

Archives are published for `darwin`/`linux` × `amd64`/`arm64` with a `checksums.txt`; swap `linux_amd64` for your platform. The snippet resolves the [latest release](https://github.com/oullin/fmtkit/releases/latest) rather than naming a version, so it does not go stale. For CI, pin `tag` to a known release instead.

**With Go** (good for local hacking and contributors):

```bash
go install go.ollin.sh/fmtkit/driver/cmd/fmtkit-go@latest
fmtkit-go check .
fmtkit-go format .
```

If `fmtkit-go` is not on your `PATH` after `go install`, add the Go bin directory: `export PATH="$(go env GOPATH)/bin:$PATH"`.

For CI, download the release binary for the runner's platform and pin the tag — it needs no daemon, no image pull, and no Node.js.

## Usage

The distributed `fmtkit` binary runs the whole pipeline (TS/Vue lint, TS/Vue formatting, Go formatting) with `format` / `format-all`, and narrows it with step flags:

```bash
fmtkit format .          # every step, over the working tree's changes
fmtkit format --ts .     # TS/Vue lint + formatting only
fmtkit format --go .     # Go formatting only
fmtkit format-all --quiet
```

`format` applies oxlint's safe fixes (`oxlint --fix`) first, then the formatting
passes normalize whatever oxlint rewrote. Standalone `fmtkit lint` only reports
violations; it never edits your files.

**`format` covers what you changed; `format-all` covers everything.** `format`
covers the files that diverge from HEAD — modified (staged or not) and
untracked — so an everyday format stays proportional to your diff rather than
the repo.
`format-all` covers every non-ignored file, and is what a CI gate wants: a
changed-file scope would pass vacuously on a fresh checkout, where nothing is
modified. Both skip anything `.gitignore`d, and both need a git working tree.

This applies to every step. The TS/Vue steps collect through git directly; the
Go formatter keeps its own walk (so `config.yml`'s `exclude` / `not_path` /
`not_name` and generated-file detection always apply) and `format` then narrows
that to what git reports as changed. `go vet` is unscoped either way — it
analyses whole packages, not files.

`ts`, `lint`, `go <subcommand>`, `check`, `version`, and `help` are also available; `fmtkit help` lists them.

The `fmtkit-go` CLI (the Go-only formatter published via `go install`) accepts:

| Command             | What it does                        |
| ------------------- | ----------------------------------- |
| `check [paths...]`  | Reports violations without writing. |
| `format [paths...]` | Rewrites files in place.            |

Both default to `.` when no paths are given. Both run `go vet ./...` automatically when the working directory is inside a Go module or workspace.

| Flag       | Default | Description                                                              |
| ---------- | ------- | ------------------------------------------------------------------------ |
| `--config` | auto    | Path to a `config.yml`. Auto-detected if omitted.                        |
| `--cwd`    | `.`     | Base path for config discovery and relative output paths.                |
| `--format` | `text`  | Output mode: `text`, `json`, or `agent`.                                 |
| `--jobs`   | `0`     | Max files in parallel; `0` uses `runtime.NumCPU()`. Reads `FMTKIT_JOBS`. |

A handful of common invocations:

```bash
fmtkit-go check .
fmtkit-go format ./core ./demo/api
fmtkit-go check --format json .
fmtkit-go check --format agent .
fmtkit-go check ./packages/go/formatter/rules/spacing/spacing.go
```

## Configuration

`fmtkit` looks for `config.yml` in the working directory; if none is found, the defaults below apply. Point at a specific file with `--config`.

```yaml
rules:
    spacing:
        enabled: true

vet:
    enabled: true

formatters:
    gofmt: true
    goimports: true

exclude:
    - .git
    - node_modules
    - vendor

not_path:
    - third_party/generated

not_name:
    - '*.pb.go'

concurrency: 0
```

| Field                   | Type | Default                          | Description                                 |
| ----------------------- | ---- | -------------------------------- | ------------------------------------------- |
| `rules.spacing.enabled` | bool | `true`                           | Enables the spacing rule.                   |
| `vet.enabled`           | bool | `true`                           | Runs `go vet ./...` after formatting.       |
| `formatters.gofmt`      | bool | `true`                           | Runs `gofmt` after the rules.               |
| `formatters.goimports`  | bool | `true`                           | Runs `goimports` after `gofmt`.             |
| `exclude`               | list | `.git`, `node_modules`, `vendor` | Directory names skipped during traversal.   |
| `not_path`              | list | empty                            | Substrings matched against full file paths. |
| `not_name`              | list | empty                            | Globs matched against file names.           |
| `concurrency`           | int  | `0`                              | Max files in parallel (`0` = `NumCPU`).     |

### TS/Vue formatting (`.oxfmtrc.json`)

The TS/Vue layer runs [`oxfmt`](https://www.npmjs.com/package/oxfmt) over your sources, then applies project-specific syntax passes for blank lines and fluent builder chains. The binary ships a bundled `.oxfmtrc.json` (tabs, single quotes, trailing commas, 200-column width) that is applied by default, so you get the same style out of the box without any setup.

The config is resolved by precedence, first match wins:

1. `FMTKIT_OXFMTRC` — an explicit path, matching the other `FMTKIT_*` knobs.
2. A project-local `.oxfmtrc.*` (`.json`, `.jsonc`, `.ts`, `.js`, …) in the directory being formatted: the bundled default is skipped and oxfmt uses yours.
3. A config derived from your Prettier setup: if the directory has a Prettier config (`.prettierrc*`, `prettier.config.*`, or a `"prettier"` key in `package.json`) but no oxfmt config, fmtkit translates it via `oxfmt --migrate=prettier` so a Prettier-configured project formats consistently with no extra setup. The translation is cached by the Prettier config's content hash, so it runs once and re-runs only when that config changes. If a config cannot be translated (a JS config importing project-local modules, say), fmtkit warns on stderr and falls back to the bundled default rather than failing the run.
4. The bundled default.

To opt out of the Prettier-derived step, drop in your own `.oxfmtrc.*`, which takes precedence over it.

### Ignoring files (`.prettierignore`)

`oxfmt` already honors `.prettierignore` (and `.gitignore`) in its own step. fmtkit extends that to the rest of the TS/Vue pipeline — the blank-line and fluent-chain passes and `oxlint --fix` — by filtering `.prettierignore`d paths out of the file set it collects, so an ignored file is left untouched by every lane. The matcher follows gitignore syntax (comments, negation, leading-`/` anchoring, trailing-`/` directories, and the `*`, `?`, `[…]`, and `**` wildcards). The Go formatter is unaffected: `.prettierignore` governs only the TS/Vue/HTML/Markdown lanes.

## What it formats

The built-in spacing rule, in summary:

- Inserts blank lines before and after control flow (`if`, `for`, `switch`, `select`, `defer`, `return`, `break`, `continue`, `goto`, `fallthrough`).
- Separates standalone `var` declarations from surrounding statements when they are not already grouped.
- Adds blank lines around standalone stdlib `sort.*` / `slices.Sort*` and `rand.*` calls, and after `t.Helper()`.
- Separates `type` declarations from neighbours and hoists all `type` definitions to the top of the file, after imports.
- Adds a blank line after anonymous-function assignments and between top-level `routes.Add` / `routes.Group` calls.

Full catalogue with before/after examples: [docs/spacing.md](docs/spacing.md).

When given directories, the engine walks recursively for `.go` files and always skips:

| Skipped                             | Reason                               |
| ----------------------------------- | ------------------------------------ |
| Hidden directories                  | Convention, not source code.         |
| `.git/`, `vendor/`                  | Repository and dependency metadata.  |
| `*.gen.go`                          | Generated code by convention.        |
| Files starting `// Code generated`  | Go's standard generated-file marker. |
| `exclude` / `not_path` / `not_name` | User-defined exclusions.             |

## Output formats

**Text** — for local runs:

```text
  Checked 1 file(s).

  main.go
    [spacing] line 5: missing blank line before if statement
    ✓ would apply spacing

  Result: fail. 1 changed, 1 violation(s), 0 error(s).
```

**JSON** — for scripts and automation:

```json
{
	"result": "fail",
	"files": 1,
	"changed": 1,
	"results": [
		{
			"file": "main.go",
			"applied": ["spacing"],
			"violations": [{ "rule": "spacing", "line": 5, "message": "missing blank line before if statement" }],
			"changed": true
		}
	]
}
```

**Agent** — compact JSON for CI and AI tools:

```json
{
	"result": "fail",
	"summary": { "files": 1, "changed": 1, "violations": 1 },
	"changed": [{ "file": "main.go", "steps": ["spacing"] }],
	"violations": [{ "file": "main.go", "rule": "spacing", "line": 5, "message": "missing blank line before if statement" }]
}
```

## Exit codes

| Command  | Code | Meaning                              |
| -------- | ---- | ------------------------------------ |
| `check`  | `0`  | No violations found.                 |
| `check`  | `1`  | Violations or errors detected.       |
| `format` | `0`  | Formatting applied successfully.     |
| `format` | `1`  | An error occurred during formatting. |

## Development

You will need Go 1.26.4+, Vite+, and [Bun](https://bun.com) (used to compile the TS sidecar the binary embeds). Vite+ manages the project Node.js runtime and pnpm version declared by the workspace.

```bash
curl -fsSL https://vite.plus -o install-vp.sh
sh install-vp.sh
vp install
```

Use Vite+ tasks for day-to-day development:

```bash
vp run build         # build the local fmtkit-go binary into storage/bin
vp run check         # run package checks across the workspace
vp run test          # run all package tests
vp run test-race     # tests with the race detector (forces CGO_ENABLED=1)
vp run test:binary   # build the self-contained binary and smoke test it
vp run vet           # run go vet across the Go module packages
vp run format -- .   # format this repo with fmtkit's own binary
vp run install-cli   # install fmtkit-go from the local source tree
vp run release       # build cross-platform binaries into storage/dist
```

### Formatting fmtkit with fmtkit

fmtkit formats itself with the binary it ships, so the development loop and the
release exercise the same Go orchestrator and the same Bun-compiled TS sidecar.
The root `Makefile` is the shortest way in:

```bash
make format                # format the repo (ARGS defaults to ".")
make format ARGS=--ts      # only the TS/Vue half
make format-all            # the whole repository
make check                 # Go formatter in check mode
```

The first run stages the host TS toolchain assets into
`packages/go/driver/internal/embedded/bin/<os>_<arch>/` (this needs Bun and takes a
few seconds); later runs reuse them and re-stage only when the support scripts,
the tool pins, or the `.oxfmtrc.json` / `.oxlintrc.json` configs change. The
inner loop is then a plain incremental `go build`.

That loop points `FMTKIT_SUPPORT_DIR` at the staged assets rather than embedding
them, which keeps it fast. The embedded-asset path a release actually uses is
covered by `vp run test:binary`.

Package layout:

```text
packages/go/              The Go module (go.ollin.sh/fmtkit)
packages/go/driver/       Stand-alone Go CLI, config loading, report rendering
packages/go/vet/          Vet planning and automatic go vet execution
packages/go/formatter/    Formatter planning, engine, rules, and formatters
packages/go/infra/        Go-toolchain task runner
packages/ts/sidecar/      Oxc-based formatting for supported non-Go file types
packages/ts/infra/        Staging for the bun-compiled TS toolchain assets
infra/                    Repo-wide tasks, shared shell lib, release scripts
```

The pipeline runs `source → spacing rule → gofmt → goimports`, skipping any stage disabled in config. New rules can be added by implementing the `Rule` interface (`Name()`, `Apply()`) and registering them with the rule set before the engine is constructed.
