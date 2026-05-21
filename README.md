# go-fmt

[![Go Reference](https://pkg.go.dev/badge/github.com/oullin/go-fmt/packages/driver.svg)](https://pkg.go.dev/github.com/oullin/go-fmt/packages/driver)
[![Go 1.25](https://img.shields.io/badge/go-1.25-00ADD8?logo=go&logoColor=white)](https://go.dev/doc/go1.25)
[![Tests](https://github.com/oullin/go-fmt/actions/workflows/tests.yml/badge.svg)](https://github.com/oullin/go-fmt/actions/workflows/tests.yml)
[![Release](https://github.com/oullin/go-fmt/actions/workflows/release.yml/badge.svg)](https://github.com/oullin/go-fmt/actions/workflows/release.yml)
[![Codecov](https://codecov.io/gh/oullin/go-fmt/graph/badge.svg?branch=main)](https://codecov.io/gh/oullin/go-fmt)
[![Docker](https://img.shields.io/badge/docker-ghcr.io%2Foullin%2Fgo--fmt-2496ED?logo=docker&logoColor=white)](https://github.com/oullin/go-fmt/pkgs/container/go-fmt)

**Rule-driven formatting for Go, runnable as a CLI or a single shared container.**

`go-fmt` fixes layout and structure that `gofmt` does not touch, then finishes with `gofmt` and `goimports`. The result is a single command that can check or rewrite Go code with consistent rule-driven formatting, automatic `go vet` validation, predictable output, and a clean path for local use, CI, or container-based workflows.

**Quick links:** [Quick Start](#quick-start) · [Installation](#installation) · [CLI](#cli) · [Docker](#docker) · [Configuration](#configuration) · [Spacing Rule](#spacing-rule) · [Development](#development)

## Why go-fmt

- Adds formatter rules on top of `gofmt`, not just whitespace cleanup
- Runs rules first, then `gofmt` and `goimports` for a deterministic result
- Runs `go vet ./...` automatically when the current working directory is inside a Go module or workspace
- Works as a local CLI or a shared container via a single host wrapper
- Supports `text`, `json`, and `agent` output modes
- Can also be embedded from the Go engine in `packages/formatter/engine`

## Quick Start

### Recommended: host wrapper

Install the single host wrapper once. One image (`ghcr.io/oullin/go-fmt:latest`) and one shared cache volume (`go-fmt-cache`) are reused across every project on the host — no per-project file, no per-project version drift:

```bash
curl -fsSL -o /tmp/go-fmt-host.sh https://raw.githubusercontent.com/oullin/go-fmt/main/scripts/go-fmt-host.sh
sudo install -m 0755 /tmp/go-fmt-host.sh /usr/local/bin/go-fmt

go-fmt check .
go-fmt format .
go-fmt format ./core ./demo/api
```

The wrapper mounts your project at `/work`, keeps Go build and module caches in the shared Docker volume `go-fmt-cache`, and avoids writing `storage/.cache` into consumer repositories. When the mounted project contains a Go module or workspace, both `check` and `format` also run `go vet ./...` automatically.

Override the image for a session with `GO_FMT_IMAGE=ghcr.io/oullin/go-fmt:v0.0.18 go-fmt …` when pinning a specific release without editing any project files.

### Local install

```bash
go install github.com/oullin/go-fmt/packages/driver/cmd/fmt@latest

fmt version
fmt check .
fmt format .
```

If `fmt` is not on `PATH`, add your Go bin directory:

```bash
export PATH="$(go env GOPATH)/bin:$PATH"
```

## Installation

### Install with Go

Requires Go 1.25 or newer.

```bash
go install github.com/oullin/go-fmt/packages/driver/cmd/fmt@latest
fmt version
```

### Build locally from this repository

Run `make help` first to see the available repository tasks and override variables before choosing a build, test, format, install, or release target.

```bash
make build
./storage/bin/go-fmt version
```

To install from the local source tree:

```bash
make install
fmt version
```

### Run from source

No install is required when working inside this repository:

```bash
./scripts/with-storage-env.sh go -C packages/driver run ./cmd/fmt check .
./scripts/with-storage-env.sh go -C packages/driver run ./cmd/fmt format .
```

### One-off Docker run

For ad-hoc use without installing the wrapper, run the published image directly:

```bash
docker run --rm -v "$PWD":/work -w /work ghcr.io/oullin/go-fmt:latest check .
docker run --rm -v "$PWD":/work -w /work ghcr.io/oullin/go-fmt:latest format .
```

## CLI

The binary is `go-fmt` and it exposes two primary commands:

| Command                    | Purpose                                   |
| -------------------------- | ----------------------------------------- |
| `go-fmt check [paths...]`  | Report violations without writing changes |
| `go-fmt format [paths...]` | Fix violations and write changes to disk  |

If no paths are provided, both commands default to the current directory (`.`). When the current working directory is inside a Go module or workspace, both commands also run `go vet ./...` automatically after file processing. If no module or workspace is present, vet is skipped.

### Flags

Both commands accept the same flags:

| Flag          | Description                                                                            | Default                           |
| ------------- | -------------------------------------------------------------------------------------- | --------------------------------- |
| `--config`    | Path to a `config.yml` file                                                            | Auto-detected in the working tree |
| `--cwd`       | Base path used for config discovery and relative output paths                          | Current working directory         |
| `--format`    | Output format: `text`, `json`, or `agent`                                              | `text`                            |
| `--host-path` | Absolute host path under `HOST_PROJECT_PATH`; intended for the Compose consumer flow   | Disabled unless env is set        |
| `--jobs`      | Max files processed in parallel; `0` uses `runtime.NumCPU()`. Also reads `GO_FMT_JOBS` | `0` (NumCPU)                      |

### Common workflows

```bash
# check the current tree
go-fmt check .

# fix the current tree
go-fmt format .

# check multiple paths
go-fmt check ./core ./demo/api

# format multiple paths
go-fmt format ./core ./demo/api

# check with JSON output
go-fmt check --format json .

# check with agent-friendly output
go-fmt check --format agent .

# check a single file
go-fmt check ./packages/formatter/rules/spacing/spacing.go

# check a host path mounted by the consumer Compose file
go-fmt check --host-path /absolute/host/project/pkg/api
```

Use positional paths when you need to target multiple files or directories. `--host-path` accepts one host path per invocation.

The stand-alone CLI formats Go source only. Repository-local `make format` also runs Oxc formatting for supported non-Go files through the `support` workspace.

## Docker

The public image is published to `ghcr.io/oullin/go-fmt` for `linux/amd64` and `linux/arm64`.

### Host wrapper

The recommended way to invoke the image is through the host wrapper installed in [Quick Start](#quick-start). It mounts the current project at `/work`, reuses the shared `go-fmt-cache` volume, and sets `HOST_PROJECT_PATH` so `--host-path` works without extra configuration.

`--host-path` accepts one absolute host path per invocation and rejects paths outside `HOST_PROJECT_PATH`:

```bash
go-fmt format --host-path "$PWD/pkg/api"
```

Use positional paths such as `./core ./demo/api` when you need multiple targets.

Override the project root with `GO_FMT_PROJECT_DIR` when invoking the wrapper from outside the project you want to format:

```bash
GO_FMT_PROJECT_DIR="$PWD" go-fmt format ./core ./demo/api
```

Remove the shared cache when you want a completely cold start:

```bash
docker volume rm go-fmt-cache
```

### One-off container usage

If you do not want the wrapper installed, run the image directly:

```bash
docker run --rm ghcr.io/oullin/go-fmt:latest
docker run --rm -v "$PWD":/work -w /work ghcr.io/oullin/go-fmt:latest check .
docker run --rm -v "$PWD":/work -w /work ghcr.io/oullin/go-fmt:latest format .
```

## Configuration

`go-fmt` looks for `config.yml` in the working directory. If none is found, built-in defaults apply. You can also point to a file explicitly with `--config path/to/config.yml`.

All fields are optional:

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
| `rules.spacing.enabled` | bool | `true`                           | Enable or disable the spacing rule          |
| `vet.enabled`           | bool | `true`                           | Run or skip automatic `go vet ./...`        |
| `formatters.gofmt`      | bool | `true`                           | Run `gofmt` after formatter rules           |
| `formatters.goimports`  | bool | `true`                           | Run `goimports` after `gofmt`               |
| `exclude`               | list | `.git`, `node_modules`, `vendor` | Directory names to skip during tree walking |
| `not_path`              | list | Empty                            | Substring matches against full file paths   |
| `not_name`              | list | Empty                            | Glob patterns matched against file names    |
| `concurrency`           | int  | `0` (NumCPU)                     | Max files processed in parallel             |

## Spacing Rule

The built-in `spacing` rule is AST-based. It enforces blank-line boundaries and declaration ordering before standard formatters run.

It also inserts a blank line after anonymous-function assignments such as `name := func(...) { ... }`, `name = func(...) { ... }`, and `var name = func(...) { ... }` when another statement follows immediately.

### What it enforces

**Blank line before control flow and jump-style statements**

A blank line is required before `if`, `for`, `range`, `switch`, `select`, `defer`, `return`, `continue`, `break`, `goto`, and `fallthrough` when they are not the first statement in a block.

```go
// before
func run() {
    x := 1
    if x > 0 {
        println("positive")
    }
    return
}

// after
func run() {
    x := 1

    if x > 0 {
        println("positive")
    }

    return
}
```

**Blank line after block statements**

A blank line is required after `if`, `for`, `range`, `switch`, `select`, and `defer` blocks when another statement follows.

```go
// before
func run() {
    if ready {
        start()
    }
    cleanup()
}

// after
func run() {
    if ready {
        start()
    }

    cleanup()
}
```

**Blank lines around standalone `var` declarations**

Standalone `var` declarations are separated from surrounding statements unless they are already grouped with nearby `var` declarations or short assignments.

```go
// before
func run() {
    x := setup()
    var cfg Config
    process(cfg)
}

// after
func run() {
    x := setup()

    var cfg Config

    process(cfg)
}
```

**Blank lines around stdlib sorting calls**

Standalone stdlib sorting calls are separated from surrounding statements with a blank line. This applies to `sort.*(...)` and `slices.Sort*(...)`, including renamed imports.

```go
// before
import stdsort "sort"

func run(values []string) {
    prepare(values)
    stdsort.Strings(values)
    consume(values)
}

// after
import stdsort "sort"

func run(values []string) {
    prepare(values)

    stdsort.Strings(values)

    consume(values)
}
```

**Blank lines around stdlib random calls**

Standalone stdlib random calls are separated from surrounding statements with a blank line. This applies to `rand.*(...)` from `math/rand` and `math/rand/v2`, including renamed imports.

```go
// before
import stdrand "math/rand"

func run() {
    prepare()
    stdrand.Int()
    consume()
}

// after
import stdrand "math/rand"

func run() {
    prepare()

    stdrand.Int()

    consume()
}
```

**Blank lines before top-level `routes.*` calls**

Top-level route registration calls on a `routes` registry are separated with a blank line before each later `routes.Add(...)` or `routes.Group(...)` call.

```go
// before
func defineRoutes() {
    routes.Add("dashboard", "GET", "/dashboard")
    routes.Group("contacts", "/contacts", func(g *wayfinder.Group) {})
}

// after
func defineRoutes() {
    routes.Add("dashboard", "GET", "/dashboard")

    routes.Group("contacts", "/contacts", func(g *wayfinder.Group) {})
}
```

**Blank lines around type declarations**

`type` declarations are separated from surrounding code with a blank line. The formatter reports this as `missing blank line around type definition`.

```go
// before
type Config struct{}
func run() {
    println("ok")
}

// after
type Config struct{}

func run() {
    println("ok")
}
```

**Type declarations at the top of the file**

All `type` definitions are moved above non-type declarations, after the import block.

### Where it applies

The spacing rule inspects statement lists inside:

- Function bodies (`BlockStmt`)
- `case` and `default` clauses (`CaseClause`)
- `select` communication clauses (`CommClause`)

## File Discovery

When given directories, the engine walks them recursively and collects `.go` files. The following are always skipped:

| Skipped                                 | Reason                                 |
| --------------------------------------- | -------------------------------------- |
| Hidden directories (`.foo/`)            | Convention, not source code            |
| `.git/`                                 | Repository metadata                    |
| `vendor/`                               | Vendored dependencies                  |
| `*.gen.go` files                        | Generated code by convention           |
| Files starting with `// Code generated` | Go standard marker for generated files |
| Paths matching `exclude`                | User-defined directory exclusions      |
| Paths matching `not_path`               | User-defined path substring exclusions |
| Files matching `not_name`               | User-defined filename glob exclusions  |

When given individual files, they are used directly and still pass through the same filtering.

## Output Formats

### Text

Human-readable output for local runs:

```text
  Checked 1 file(s).

  main.go
    [spacing] line 5: missing blank line before if statement
    [spacing] line 8: missing blank line before return statement
    ✓ would apply spacing

  Result: fail. 1 changed, 2 violation(s), 0 error(s).
```

### JSON

Structured output for scripts and automation. The CLI emits a single JSON object; it is laid out below for readability:

```json
{
	"result": "fail",
	"files": 1,
	"changed": 1,
	"results": [
		{
			"file": "main.go",
			"applied": ["spacing"],
			"violations": [
				{
					"rule": "spacing",
					"line": 5,
					"message": "missing blank line before if statement"
				},
				{
					"rule": "spacing",
					"line": 8,
					"message": "missing blank line before return statement"
				}
			],
			"changed": true
		}
	]
}
```

### Agent

Compact JSON for AI agents and CI integrations:

```json
{
	"result": "fail",
	"summary": {
		"files": 1,
		"changed": 1,
		"violations": 2
	},
	"changed": [
		{
			"file": "main.go",
			"steps": ["spacing"]
		}
	],
	"violations": [
		{
			"file": "main.go",
			"rule": "spacing",
			"line": 5,
			"message": "missing blank line before if statement"
		},
		{
			"file": "main.go",
			"rule": "spacing",
			"line": 8,
			"message": "missing blank line before return statement"
		}
	]
}
```

## Exit Codes

| Command         | Code | Meaning                             |
| --------------- | ---- | ----------------------------------- |
| `go-fmt check`  | `0`  | No violations found                 |
| `go-fmt check`  | `1`  | Violations or errors detected       |
| `go-fmt format` | `0`  | Formatting applied successfully     |
| `go-fmt format` | `1`  | An error occurred during formatting |

## Development

### Prerequisites

- Go 1.25 or newer
- Node.js 25.8.2 with `pnpm` 10.32.1 for repo-local Make and Turbo workflows
- Docker Desktop or another Docker runtime if you use the published image

Install workspace dependencies before using local repository tasks:

```bash
nvm use
pnpm install
```

Start with `make help` to see the available repository tasks and override variables for local development.

### Common tasks

```bash
make help
pnpm turbo run check --filter=formatter
pnpm turbo run check --filter=support
make format
make build
make release
make test
make test-race
make test-short
make vet
make fmt-source
make install
make clean
```

`make test-race` forces `CGO_ENABLED=1` because the Go race detector requires CGO.

### Make variables

| Variable            | Default                                             | Description                           |
| ------------------- | --------------------------------------------------- | ------------------------------------- |
| `ARGS`              | `.`                                                 | Files or directories to target        |
| `VERSION`           | `git describe ...` or `dev`                         | Build version injected into binaries  |
| `CGO_ENABLED`       | `0`                                                 | CGO setting for build and release     |
| `BUILD_DIR`         | `storage/bin`                                       | Output directory for local binaries   |
| `DIST_DIR`          | `storage/dist`                                      | Output directory for release binaries |
| `RELEASE_PLATFORMS` | `darwin/amd64 darwin/arm64 linux/amd64 linux/arm64` | Platforms built by `make release`     |

### Examples

```bash
make format ARGS=README.md
make fmt-source
make release DIST_DIR=storage/builds
```

Repository-managed caches and generated binaries are expected under `storage/`, with caches rooted at `storage/.cache`.

### Release pipeline

The release workflow:

- runs checks for all workspaces
- runs Go tests with the race detector
- creates the next Git tag when `main` advances
- publishes the GitHub release from that immutable tag
- publishes `ghcr.io/oullin/go-fmt:latest`
- publishes `ghcr.io/oullin/go-fmt:<tag>`
- pushes multi-arch images for `linux/amd64` and `linux/arm64`
- supports retrying a failed publish by manually dispatching the publish workflow for an existing tag

`Release Tag` creates the next version tag with the default `GITHUB_TOKEN`, then invokes `Publish Release` directly as a reusable workflow. Manual retries remain available through `workflow_dispatch` on the publish workflow for an existing tag.

## Package Layout

```text
packages/driver/      Stand-alone Go CLI entrypoint, config loading, report rendering
packages/vet/         Vet planning and automatic go vet execution
packages/formatter/   Formatter planning, engine, rules, and formatters
packages/support/     Oxc-based formatting for supported non-Go file types
```

### Formatting pipeline

```text
source file -> spacing rule -> gofmt -> goimports -> result
```

Each step runs only when enabled in config. If a file is unchanged after the pipeline, it is reported as clean.

### Extensibility

The rule system is designed to be extended. New rules can be created by implementing the `Rule` interface (`Name()` and `Apply()`) and registering them in the rule set before the engine is constructed.
