# go-fmt

[![Go Reference](https://pkg.go.dev/badge/github.com/oullin/go-fmt/packages/driver.svg)](https://pkg.go.dev/github.com/oullin/go-fmt/packages/driver)
[![Go 1.26](https://img.shields.io/badge/go-1.26-00ADD8?logo=go&logoColor=white)](https://go.dev/doc/go1.26)
[![Tests](https://github.com/oullin/go-fmt/actions/workflows/tests.yml/badge.svg)](https://github.com/oullin/go-fmt/actions/workflows/tests.yml)
[![Release](https://github.com/oullin/go-fmt/actions/workflows/release.yml/badge.svg)](https://github.com/oullin/go-fmt/actions/workflows/release.yml)
[![Codecov](https://codecov.io/gh/oullin/go-fmt/graph/badge.svg?branch=main)](https://app.codecov.io/github/oullin/go-fmt)
[![Docker](https://img.shields.io/badge/docker-ghcr.io%2Foullin%2Fgo--fmt-2496ED?logo=docker&logoColor=white)](https://github.com/oullin/go-fmt/pkgs/container/go-fmt)

`go-fmt` is a rule-driven formatter for Go. It enforces layout and structure that `gofmt` leaves alone — blank lines around control flow, type hoisting, declaration grouping — then hands off to `gofmt` and `goimports` for the final pass. You can run it as a local CLI or as a shared Docker image; either way the output is the same.

## At a glance

- AST-based spacing rules, then `gofmt` and `goimports`, in one deterministic pipeline.
- Runs `go vet ./...` automatically when invoked inside a Go module or workspace.
- Three output modes: `text` for humans, `json` for scripts, `agent` for CI and AI tools.
- One CLI binary (`fmt-go`) or one Docker image with a thin host wrapper.
- Engine in [`packages/formatter/engine`](packages/formatter/engine) is importable from Go.

## Install

Two ways to run it. Pick the one that fits your workflow — both produce identical output.

**With Go** (good for local hacking and contributors):

```bash
go install github.com/oullin/go-fmt/packages/driver/cmd/fmt-go@latest
fmt-go check .
fmt-go format .
```

**With Docker** (good for CI and pinning one version across many projects):

```bash
curl -fsSL -o /tmp/go-fmt-host.sh https://raw.githubusercontent.com/oullin/go-fmt/main/scripts/go-fmt-host.sh
sudo install -m 0755 /tmp/go-fmt-host.sh /usr/local/bin/go-fmt
go-fmt format .
```

The Docker wrapper mounts the current directory at `/work` and reuses a shared `go-fmt-cache` volume across projects, so you do not need a per-project Dockerfile or version pin. Pin a specific release for a single run with `GO_FMT_IMAGE=ghcr.io/oullin/go-fmt:v0.0.18 go-fmt …`, or swap in a flavour tag (`latest-go`, `latest-node-ts`, `latest-full`) when you only need one formatter layer.

If `fmt-go` is not on your `PATH` after `go install`, add the Go bin directory: `export PATH="$(go env GOPATH)/bin:$PATH"`.

## Usage

| Command             | What it does                        |
| ------------------- | ----------------------------------- |
| `check [paths...]`  | Reports violations without writing. |
| `format [paths...]` | Rewrites files in place.            |

Both default to `.` when no paths are given. Both run `go vet ./...` automatically when the working directory is inside a Go module or workspace.

| Flag          | Default | Description                                                              |
| ------------- | ------- | ------------------------------------------------------------------------ |
| `--config`    | auto    | Path to a `config.yml`. Auto-detected if omitted.                        |
| `--cwd`       | `.`     | Base path for config discovery and relative output paths.                |
| `--format`    | `text`  | Output mode: `text`, `json`, or `agent`.                                 |
| `--jobs`      | `0`     | Max files in parallel; `0` uses `runtime.NumCPU()`. Reads `GO_FMT_JOBS`. |
| `--host-path` | off     | Absolute host path under `HOST_PROJECT_PATH` (Docker host-mount flow).   |

A handful of common invocations:

```bash
fmt-go check .
fmt-go format ./core ./demo/api
fmt-go check --format json .
fmt-go check --format agent .
fmt-go check ./packages/formatter/rules/spacing/spacing.go
```

## Configuration

`go-fmt` looks for `config.yml` in the working directory; if none is found, the defaults below apply. Point at a specific file with `--config`.

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

The TS/Vue layer runs [`oxfmt`](https://www.npmjs.com/package/oxfmt) over your sources. The images ship a bundled `.oxfmtrc.json` (tabs, single quotes, trailing commas, 200-column width) that is applied by default, so you get the same style out of the box without any setup.

A project-local oxfmt config takes precedence: if the directory being formatted contains its own `.oxfmtrc.*` (`.json`, `.jsonc`, `.ts`, `.js`, …), the bundled default is skipped and oxfmt uses yours. Override the bundled path explicitly with the `GO_FMT_OXFMTRC` environment variable, matching the other `GO_FMT_*` knobs.

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

## Docker images

Published to `ghcr.io/oullin/go-fmt` for `linux/amd64` and `linux/arm64`.

| Tag                               | Contents                            | Entrypoint |
| --------------------------------- | ----------------------------------- | ---------- |
| `latest`, `<tag>`                 | Full Go + TS/Vue formatter (alias). | `fmt-all`  |
| `latest-full`, `<tag>-full`       | Full Go + TS/Vue formatter.         | `fmt-all`  |
| `latest-go`, `<tag>-go`           | Go formatter CLI only.              | `fmt-go`   |
| `latest-node-ts`, `<tag>-node-ts` | TS/Vue formatter only.              | `fmt-ts`   |

## Exit codes

| Command  | Code | Meaning                              |
| -------- | ---- | ------------------------------------ |
| `check`  | `0`  | No violations found.                 |
| `check`  | `1`  | Violations or errors detected.       |
| `format` | `0`  | Formatting applied successfully.     |
| `format` | `1`  | An error occurred during formatting. |

## Development

You will need Go 1.26+, Node.js 25.8.2 with pnpm 10.33.0, and a Docker runtime if you plan to touch the published images. Install the workspace once with `nvm use && pnpm install`.

```bash
make help          # see every target and override variable
make build         # build the local fmt-go binary into storage/bin
make test          # run all package tests
make test-race     # tests with the race detector (forces CGO_ENABLED=1)
make vet           # run go vet across the workspace
make format        # run the Dockerised formatter against ARGS (default ".")
make format-all    # run the formatter against the whole repository
make install       # install fmt-go from the local source tree
make release       # build cross-platform binaries into storage/dist
```

Package layout:

```text
packages/driver/      Stand-alone Go CLI, config loading, report rendering
packages/vet/         Vet planning and automatic go vet execution
packages/formatter/   Formatter planning, engine, rules, and formatters
packages/devx/        Oxc-based formatting for supported non-Go file types
```

The pipeline runs `source → spacing rule → gofmt → goimports`, skipping any stage disabled in config. New rules can be added by implementing the `Rule` interface (`Name()`, `Apply()`) and registering them with the rule set before the engine is constructed.
