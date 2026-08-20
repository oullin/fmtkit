# fmtkit

[![Go Reference](https://pkg.go.dev/badge/go.ollin.sh/fmtkit/driver.svg)](https://pkg.go.dev/go.ollin.sh/fmtkit/driver)
[![Go 1.26.5](https://img.shields.io/badge/go-1.26.5-00ADD8?logo=go&logoColor=white)](https://go.dev/doc/go1.26)
[![Tests](https://github.com/oullin/fmtkit/actions/workflows/tests.yml/badge.svg)](https://github.com/oullin/fmtkit/actions/workflows/tests.yml)
[![Release](https://github.com/oullin/fmtkit/actions/workflows/release.yml/badge.svg)](https://github.com/oullin/fmtkit/actions/workflows/release.yml)

One formatter for a Go + TypeScript repository. `fmtkit` enforces the layout rules `gofmt` and `oxfmt` leave alone, blank lines around control flow, declaration ordering, class member order; then hands off to the standard formatters for the final pass.

## What it is

A single self-contained binary that formats both halves of a full-stack repo:

- **Go** — an AST-based spacing rule, then `gofmt` and `goimports`, plus an automatic `go vet ./...`.
- **TypeScript / Vue** — `oxlint --fix`, then `oxfmt`, then structural passes for blank lines, class member order, and fluent chains. Also formats the embedded TS blocks in Markdown and HTML.

The TS toolchain is compiled with Bun and embedded in the binary, so there is **no Node.js requirement** and nothing to `npm install`. One download, one command, both languages.

If you only want the Go half, `fmtkit-go` is a separate `go install`-able CLI, and the engine is importable as a library.

## Why

`gofmt` is deliberately conservative: it normalizes indentation and alignment, but it will never tell you that a `return` should be preceded by a blank line, or that your `type` declarations belong at the top of the file. `oxfmt` is the same story on the TS side, they own whitespace within a statement, not the rhythm between statements.

That leaves a whole category of "style" that lives in review comments and team wikis, gets applied inconsistently, and produces diff noise when someone finally cleans it up. `fmtkit` moves those rules into the formatter, where they get applied the same way every time and stop being a thing people argue about.

It is deliberately opinionated. There is one spacing rule with one shape, and the knobs are for turning things off, not for tuning them.

## Who it's for

- Teams with a **Go backend and a TS/Vue frontend in one repo** who are tired of running two toolchains with two config surfaces and two CI steps.
- Anyone who wants **more structure than `gofmt` provides** and would rather not hand-maintain it.
- **CI pipelines** that want a formatting gate with no daemon and no Node.js on the runner — a plain binary by default, or an image from GHCR if your CI is container-shaped.
- **AI coding agents and scripts**, via the `json` and `agent` output modes.

It is probably _not_ for you if you want a configurable style engine — fmtkit has opinions and only a few dials.

## Install

Every route ships the same binary and produces identical output. Pick whichever fits.

### Homebrew (recommended)

```bash
brew tap oullin/fmtkit
brew install --cask fmtkit
fmtkit format .
```

The binary embeds the TS toolchain (oxfmt, oxlint, oxc-parser and the support scripts) and extracts it to your user cache directory on first run.

### Linux / GitHub Releases

Homebrew casks are macOS-only. On Linux, grab the same binary directly:

```bash
tag=$(curl -fsSLI -o /dev/null -w '%{url_effective}' https://github.com/oullin/fmtkit/releases/latest | sed 's#.*/##')
curl -fsSL "https://github.com/oullin/fmtkit/releases/download/${tag}/fmtkit_${tag#v}_linux_amd64.tar.gz" | tar -xz fmtkit
sudo install -m 0755 fmtkit /usr/local/bin/fmtkit
```

Archives are published for `darwin`/`linux` × `amd64`/`arm64` with a `checksums.txt`; swap `linux_amd64` for your platform. The snippet resolves the [latest release](https://github.com/oullin/fmtkit/releases/latest) rather than naming a version, so it does not go stale. But **for CI, pin `tag` to a known release** so a new upstream version can't change your build.

### Docker

Every release also ships as a multi-arch image (`linux/amd64` + `linux/arm64`) at [ghcr.io/oullin/fmtkit](https://github.com/oullin/fmtkit/pkgs/container/fmtkit), for container-shaped CI and for machines where installing a binary is inconvenient:

```bash
docker run --rm -u "$(id -u):$(id -g)" -v "$PWD":/work ghcr.io/oullin/fmtkit:latest format .
```

The container expects your repository bind-mounted at `/work`. `-u` keeps rewritten files owned by you rather than root. The image carries everything the pipeline needs — `git`, a Go toolchain (goimports and the automatic `go vet` pass need the `go` command), and the TS toolchain pre-extracted so there is no first-run cost. **For CI, pin a version tag** (`ghcr.io/oullin/fmtkit:vX.Y.Z`) instead of `latest`.

#### Windows and WSL

There are no native Windows binaries; on Windows, the Docker image is the supported route.

- **WSL2** is a regular glibc Linux: use the [Linux install](#linux--github-releases) or the `docker run` command above unchanged. Keep the repository in the WSL filesystem (not `/mnt/c/...`) — bind mounts from the Windows drive are slow.
- **Docker Desktop from PowerShell**: same image, Windows-shaped syntax — and skip `-u`, which is a Unix-ism that NTFS bind mounts don't need:

    ```powershell
    docker run --rm -v "${PWD}:/work" ghcr.io/oullin/fmtkit:latest format .
    ```

- **Line endings**: fmtkit writes LF (oxfmt's default; gofmt always does). A checkout made with `core.autocrlf=true` will show every line as changed on first format — set `core.autocrlf` to `false` (or `input`) and let `.gitattributes` own line endings.

### Go install (Go-only CLI)

```bash
go install go.ollin.sh/fmtkit/driver/cmd/fmtkit-go@latest
fmtkit-go check .
```

This gives you `fmtkit-go`, the Go formatter alone; no TS/Vue support. Good for Go-only projects and for contributors.

If it isn't on your `PATH` afterward: `export PATH="$(go env GOPATH)/bin:$PATH"`.

## Quickstart

```bash
fmtkit format .          # everything you changed, both languages
fmtkit format --go .     # Go only
fmtkit format --ts .     # TS/Vue only
fmtkit format-all        # the entire repository
fmtkit check .           # report Go violations, write nothing
fmtkit lint .            # report TS/Vue lint violations, write nothing
```

In CI, use `format-all` (or `check`) — see [`format` vs `format-all`](#format-vs-format-all) for why the scope matters.

## What it does to your code

### Go

Given this:

```go
func run(items []string) error {
	total := 0
	type result struct{ n int }
	for _, it := range items {
		total += len(it)
	}
	if total == 0 {
		return fmt.Errorf("empty")
	}
	r := result{n: total}
	return nil
}
```

`fmtkit format` produces:

```go
func run(items []string) error {
	total := 0

	type result struct{ n int }

	for _, it := range items {
		total += len(it)
	}

	if total == 0 {
		return fmt.Errorf("empty")
	}

	r := result{n: total}

	return nil
}
```

The spacing rule in summary:

- Blank lines **before and after control flow** — `if`, `for`, `range`, `switch`, `select`, `defer`, `return`, `break`, `continue`, `goto`, `fallthrough`.
- Separates standalone `var` declarations from surrounding statements when they aren't already grouped.
- Blank lines around standalone stdlib `sort.*` / `slices.Sort*` and `rand.*` calls, and after `t.Helper()`.
- Separates `type` declarations from their neighbors, and **hoists top-level `type` definitions** to the top of the file, after imports.
- Blank line after anonymous-function assignments, and between top-level `routes.Add` / `routes.Group` calls.

Full catalogue with before/after for every variant: [docs/spacing.md](docs/spacing.md).

### TypeScript / Vue

The TS lane runs `oxlint --fix` for safe lint fixes, then `oxfmt`, then these structural passes:

| Pass                     | What it does                                                                   |
| ------------------------ | ------------------------------------------------------------------------------ |
| `BlankLinePass`          | The statement-spacing rules, mirroring the Go side.                            |
| `ClassReorderPass`       | Reorders class members into a stable shape (properties, constructor, methods). |
| `DeclarationReorderPass` | Reorders declarations, only where provably side-effect safe.                   |
| `FluentChainPass`        | Splits fluent call chains so each link starts on its own line.                 |
| `ExpandedCallPass`       | Expands structurally complex call arguments into stable multiline layouts.     |
| `BodyWrapPass`           | Braces unbraced statement bodies.                                              |

### What is never touched

When given directories, the engine walks recursively and always skips:

| Skipped                             | Reason                               |
| ----------------------------------- | ------------------------------------ |
| Hidden directories                  | Convention, not source code.         |
| `.git/`, `vendor/`                  | Repository and dependency metadata.  |
| `*.gen.go`                          | Generated code by convention.        |
| Files starting `// Code generated`  | Go's standard generated-file marker. |
| `.gitignore`d paths                 | Not yours to format.                 |
| `exclude` / `not_path` / `not_name` | Your own exclusions (see below).     |

## Commands

### `fmtkit` (the full binary)

| Command                                     | What it does                                         |
| ------------------------------------------- | ---------------------------------------------------- |
| `format [--ts] [--go] [--quiet] [paths...]` | Format files changed vs `HEAD`, plus untracked ones. |
| `format-all [--ts] [--go] [--quiet]`        | Format every non-ignored file in the repo.           |
| `ts [paths...]`                             | TS/Vue formatting only.                              |
| `lint [paths...]`                           | Report TS/Vue lint violations. Never writes.         |
| `check [args...]`                           | Run the Go formatter in check mode.                  |
| `go <check\|format\|sources\|version>`      | The Go formatter CLI.                                |
| `version`, `help`                           | The usual.                                           |

No language flag means all lanes, TS before Go.

`format` applies oxlint's safe fixes first, then the formatting passes normalize whatever oxlint rewrote. Standalone `lint` only reports; it never edits your files.

### `format` vs `format-all`

**`format` covers what you changed; `format-all` covers everything.**

`format` covers files that diverge from `HEAD`, modified (staged or not) and untracked, so an everyday format stays proportional to your diff rather than your repo.

`format-all` covers every non-ignored file, and **is what a CI gate wants**: a changed-file scope would pass vacuously on a fresh checkout, where nothing is modified.

Both skip `.gitignore`d files, and both need a git working tree.

This applies to every step, with two wrinkles: the Go formatter keeps its own walk (so `config.yml`'s `exclude` / `not_path` / `not_name` and generated-file detection always apply) and `format` then narrows that to what git reports as changed; and `go vet` is unscoped either way, because it analyses whole packages, not files.

### `fmtkit-go` (the Go-only CLI)

| Command                                       | What it does                                                         |
| --------------------------------------------- | -------------------------------------------------------------------- |
| `check [paths...]`                            | Reports violations without writing.                                  |
| `format [paths...]`                           | Rewrites files in place.                                             |
| `sources [--include-declarations] [paths...]` | Prints the collected file list, NUL-separated. Plumbing for scripts. |

Both `check` and `format` default to `.`, and both run `go vet ./...` automatically when the working directory is inside a Go module or workspace.

| Flag       | Default | Description                                                              |
| ---------- | ------- | ------------------------------------------------------------------------ |
| `--config` | auto    | Path to a `config.yml`. Auto-detected if omitted.                        |
| `--cwd`    | `.`     | Base path for config discovery and relative output paths.                |
| `--format` | `text`  | Output mode: `text`, `json`, or `agent`.                                 |
| `--jobs`   | `0`     | Max files in parallel; `0` uses `runtime.NumCPU()`. Reads `FMTKIT_JOBS`. |

```bash
fmtkit-go check .
fmtkit-go format ./core ./demo/api
fmtkit-go check --format json .
fmtkit-go check ./packages/go/formatter/rules/spacing/spacing.go
```

## Configuration

### Go (`config.yml`)

`fmtkit` looks for `config.yml` in the working directory; without one, the defaults below apply. Point at a specific file with `--config`.

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

### TS/Vue (`.oxfmtrc.json`)

The binary ships a bundled `.oxfmtrc.json` (tabs, single quotes, trailing commas, 200-column width) applied by default, so you get a consistent style with zero setup. Resolution is by precedence, first match wins:

1. **`FMTKIT_OXFMTRC`** — an explicit path.
2. **A project-local `.oxfmtrc.*`** (`.json`, `.jsonc`, `.ts`, `.js`, …) in the directory being formatted. The bundled default is skipped and oxfmt uses yours.
3. **Your Prettier config.** If the directory has a Prettier config (`.prettierrc*`, `prettier.config.*`, or a `"prettier"` key in `package.json`) but no oxfmt config, fmtkit translates it via `oxfmt --migrate=prettier`, so a Prettier-configured project formats consistently with no extra setup. The translation is cached by the Prettier config's content hash, so it runs once and re-runs only when that config changes. If a config can't be translated (a JS config importing project-local modules, say), fmtkit warns on stderr and falls back to the bundled default rather than failing the run.
4. **The bundled default.**

To opt out of the Prettier-derived step, drop in your own `.oxfmtrc.*` — it takes precedence.

### Ignoring files (`.prettierignore`)

`oxfmt` already honors `.prettierignore` and `.gitignore` in its own step. fmtkit extends that to the rest of the TS/Vue pipeline — the structural passes and `oxlint --fix` — by filtering ignored paths out of the file set it collects, so an ignored file is untouched by every lane.

The matcher follows gitignore syntax: comments, negation, leading-`/` anchoring, trailing-`/` directories, and the `*`, `?`, `[…]`, and `**` wildcards. The Go formatter is unaffected — `.prettierignore` governs only the TS/Vue/HTML/Markdown lanes.

## Output formats

**`text`** — for humans:

```text
Formatter

  Checked 1 file(s).

  main.go
    [spacing] line 7: missing blank line before type definition
    [spacing] line 11: missing blank line before if statement
    ✓ would apply spacing

  Result: fail. 1 changed, 2 violation(s), 0 error(s).

Vet

  Result: ok. 0 error(s).
```

**`json`** — for scripts. Emitted as a single line; shown here expanded:

```json
{
	"result": "fail",
	"formatter": {
		"result": "fail",
		"files": 1,
		"changed": 1,
		"results": [
			{
				"file": "main.go",
				"applied": ["spacing"],
				"violations": [{ "rule": "spacing", "line": 7, "message": "missing blank line before type definition" }],
				"changed": true
			}
		]
	},
	"vet": { "status": "skipped" }
}
```

**`agent`** — indented JSON, grouped for CI and AI tools:

```json
{
	"result": "fail",
	"formatter": {
		"result": "fail",
		"summary": { "files": 1, "changed": 1, "violations": 1 },
		"changed": [{ "file": "main.go", "steps": ["spacing"] }],
		"violations": [{ "file": "main.go", "rule": "spacing", "line": 7, "message": "missing blank line before type definition" }]
	},
	"vet": { "status": "skipped" }
}
```

The `json` and `agent` shapes are a public contract, pinned by golden tests.

## Exit codes

| Command  | Code | Meaning                              |
| -------- | ---- | ------------------------------------ |
| `check`  | `0`  | No violations found.                 |
| `check`  | `1`  | Violations or errors detected.       |
| `format` | `0`  | Formatting applied successfully.     |
| `format` | `1`  | An error occurred during formatting. |

Note that `format` exits `0` when it _fixes_ violations — it only fails on a genuine error. Use `check` for gates.

## Development

You'll need Go 1.26.5+, [Bun](https://bun.com) (to compile the TS sidecar), and Vite+ (which manages the Node.js runtime and pnpm version the workspace declares).

```bash
curl -fsSL https://vite.plus -o install-vp.sh
sh install-vp.sh
vp install
```

Day-to-day tasks:

```bash
vp run build         # build the local fmtkit-go binary into storage/bin
vp run check         # package checks across the workspace
vp run test          # all package tests
vp run test-race     # tests with the race detector (forces CGO_ENABLED=1)
vp run test:binary   # build the self-contained binary and smoke test it
vp run vet           # go vet across the Go module packages
vp run install-cli   # install fmtkit-go from the local source tree
vp run release       # cross-platform binaries into storage/dist
```

### fmtkit formats itself

fmtkit formats its own source with the binary it ships, so the development loop and the release exercise the same Go orchestrator and the same Bun-compiled sidecar. The `Makefile` is the shortest way in:

```bash
make format                # format the repo (ARGS defaults to ".")
make format ARGS=--ts      # only the TS/Vue half
make format-all            # the whole repository
make check                 # Go formatter in check mode
make version               # the version the working tree builds as
```

The first run stages the host TS toolchain into `packages/go/driver/internal/typescript/embedded/bin/<os>_<arch>/` (needs Bun, takes a few seconds). Later runs reuse it and re-stage only when the support scripts, the tool pins, or the `.oxfmtrc.json` / `.oxlintrc.json` configs change. The inner loop is then a plain incremental `go build`.

That loop points `FMTKIT_SUPPORT_DIR` at the staged assets rather than embedding them, which keeps it fast. The embedded-asset path a release actually uses is covered by `vp run test:binary`.

## How the code is organized

fmtkit is one binary with two halves:

- A **Go driver** (`packages/go`) that owns the CLI, finds files, formats Go, runs `go vet`, renders reports, and orchestrates the run.
- A **TypeScript sidecar** (`packages/ts/sidecar`), compiled with Bun and embedded in the binary, that formats TS/Vue and the embedded blocks in Markdown/HTML.

The driver runs the sidecar as a child process. Everything crossing that boundary. The executable name, modes, flags, env vars, and the summary lines the driver reads back is defined once per side (`driver/internal/typescript/proto` in Go, the `cli/` DTOs in TS) and pinned by tests. **Change one side, and you change the other in the same PR.**

### Go side (`packages/go`, module `go.ollin.sh/fmtkit`)

The importable library:

| Package                   | What it does                                                                                                                          |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| `formatter`               | Public entry points: `Check`, `Format`, `CheckFiles`, `FormatFiles`.                                                                  |
| `formatter/engine`        | Runs the formatters over files concurrently and builds the `Report`.                                                                  |
| `formatter/config`        | Single source of truth for formatter settings and defaults.                                                                           |
| `formatter/rules/spacing` | The spacing rule. Parses each file once, then three types do the work: blank-line insertion, type reordering, embed-directive repair. |
| `vet`                     | Wraps `go vet` behind an injectable toolchain so tests can fake it.                                                                   |
| `driver/config`           | CLI config. Embeds the formatter config and adds the vet toggle; the `config.yml` schema is a public contract.                        |
| `driver/report`           | Typed output modes and the renderer; the JSON/agent shapes are a public contract.                                                     |

The CLI internals (`driver/internal/...`), one job each: `command` holds the dispatch table both binaries share; `app` wires things together, registering language lanes with `toolchain` — the registry that turns `--ts`/`--go` into an ordered set of lanes; `pipeline` runs generic steps whose summaries come from typed results (nothing scrapes rendered text); `console` owns terminal colors and printing; `gitfiles` owns git-backed file selection.

Each language then owns its behavior in its own package. `golang` is the Go check/format use case (returning a typed `Outcome`) plus its format step. `typescript` builds the TS/Vue lint and format steps and splits its machinery across subpackages — `typescript/runtime` extracts and spawns the sidecar, `typescript/proto` is the frozen wire protocol, `typescript/filetypes` and `typescript/prettierignore` each own one kind of file selection composed by `typescript/sourcefiles`, and `typescript/embedded` holds the `go:embed` assets (its `bin/` folder is where staging writes — **do not move it**).

### TS side (`packages/ts/sidecar/src`)

| Directory   | What it does                                                                                                                                                     |
| ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `kernel/`   | `Result` helpers, error types, the concurrency pool.                                                                                                             |
| `syntax/`   | Parsing and editing: `SourceDocument` (an immutable file value), `SourceParser` (the Zod boundary), `AstReader`, `EditApplier`.                                  |
| `hosts/`    | Pulls TS out of `.vue`/`.md`/`.html` files and puts it back.                                                                                                     |
| `passes/`   | One class per formatting rule. Every pass implements the same small interface: `computeEdits(document)` returns edits. Policy classes hold the layout knowledge. |
| `pipeline/` | Runs passes in order. `PipelineFactory` is the only place a pass sequence is defined; loops and fixed points are declared there, not hidden inside passes.       |
| `io/`       | File and process access behind ports, with Node adapters.                                                                                                        |
| `cli/`      | The commands, the DTOs that parse argv, and `CompositionRoot` — the one place everything gets constructed. Entry files are just `main()` shims.                  |

**Adding a TS pass:** write a class implementing `FormattingPass`, register it in `PipelineFactory`. Nothing else changes.
**Adding a Go rule:** implement the `Rule` interface (`Name()`, `Apply()`) and register it before the engine is built.

### Ground rules

- **Logic lives on types.** Go logic belongs to structs with methods; free functions are for small stateless helpers only. TS code lives in classes with real instances and constructor-injected dependencies — the only exceptions are `main()` entry shims, the `Result` helpers, value types with factory statics (the Zod DTOs, `SourceDocument.of`), and error classes.
- **Parse, don't validate.** Outside data enters through a Zod-backed DTO exactly once. No `typeof` checks in TS source.
- **The wire is frozen.** The Go↔TS protocol values never change casually; golden tests on both sides fail loudly if they drift.
- **The repo formats itself.** `make format-all` must leave the tree unchanged. Write class members in the formatter's order (properties, constructor, methods) or the self-check will reorder them for you.
- **Golden are never regenerated to make a change pass.** Pipeline transcripts, report renders, CLI usage/exit codes, and the spacing corpus are pinned byte-for-byte; if a golden fails, the code is wrong.

The Go pipeline runs `source → spacing rule → gofmt → goimports`, skipping any stage disabled in config.

## License

[MIT](LICENSE)
