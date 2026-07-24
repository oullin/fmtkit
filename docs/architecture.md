# Architecture

fmtkit is one binary with two halves: a Go driver that owns the CLI, file
discovery, reporting, and orchestration, and a bun-compiled TypeScript sidecar
that owns the TS/Vue formatting passes. This document explains how the pieces
fit, the contracts between them, and the design rules the codebase follows.

## Design rules

- **Behavior lives on types.** Go logic belongs to structs with methods that
  share state through their owner (parse context, tree handles, printers) —
  free functions are reserved for genuinely stateless leaf predicates.
  TypeScript code lives behind classes with real instances and
  constructor-injected dependencies.
- **Sanctioned exceptions (TS).** Only these may be static or free:
  entrypoint `main()` bootstraps (a `main` plus a run-as-main guard, nothing
  else), the documented `Result`/`ok`/`err` helpers in `kernel/result.ts`,
  value types with factory statics (Zod DTOs' `parse`/`from`, `SourceDocument.of`,
  `IterationBudget.once`), and `Error` subclasses.
- **Parse, don't validate (TS).** Untrusted data crosses a boundary once,
  through a frozen Zod-backed DTO (`*CliDto`, `ParsedSourceDto`). No `typeof`
  narrowing in source. The deep AST is the one documented relaxation: only
  node envelopes are schema-validated; descendants are trusted Oxc output.
- **The wire is frozen.** Every value crossing the Go↔TS process boundary is
  defined exactly once per side (Go: `driver/internal/sidecarproto`; TS: the
  CLI DTOs) and covered by golden tests. Changing one requires changing both
  sides in the same PR.
- **The repo formats itself.** `make format-all` must leave the tree
  unchanged. Write class members in the formatter's order — properties, then
  constructor, then methods, blank lines between members — or the self-check
  will reorder them for you.

## Go (`packages/go`, module `go.ollin.sh/fmtkit`)

### Public library

| Package                   | Role                                                                                                                                  |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| `formatter`               | Facade: `Check`/`Format`/`CheckFiles`/`FormatFiles`                                                                                   |
| `formatter/engine`        | `Engine` runs `Formatter` implementations concurrently, produces `Report`                                                             |
| `formatter/config`        | The single source of truth for formatter configuration and defaults                                                                   |
| `formatter/rules/spacing` | The spacing rule. Internally: `fileContext` (parse once) shared by `blankLineInserter`, `typeOrderRewriter`, `embedDirectiveRepairer` |
| `vet`                     | `go vet` wrapper with an injectable toolchain for tests                                                                               |
| `driver/config`           | CLI config: embeds `formatter/config.Config` (`mapstructure:",squash"`) plus the vet toggle; `config.yml` schema is a public contract |
| `driver/report`           | Typed `Mode`/`Format` values, `Renderer{Root, Mode}`; the JSON/agent output shapes are a public contract                              |

### Driver internals (`driver/internal/...`)

| Package          | Role                                                                                                                                                           |
| ---------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `command`        | `Command` + `Set`: the one dispatch table. Both binaries and the umbrella `go` subcommand are `Set`s built by `app`                                            |
| `app`            | Composition root only: builds the command `Set`s, constructs the pipeline `Step`s, resolves color once                                                         |
| `gotool`         | The Go check/format use case: `ParseInvocation` → `Invocation`, `Execute(Request)` → `Outcome` (typed result; owns exit policy via `Outcome.ExitCode`)         |
| `pipeline`       | Generic mechanism: `Step`/`Result`/`Detail` + the section/tee/quiet-failure loop. Steps compute summaries from typed results — never by scraping rendered text |
| `console`        | Terminal presentation: `DetectColor` (the only NO_COLOR/FORCE_COLOR read) + `Printer`                                                                          |
| `gitfiles`       | `Tree`: git-based file discovery, `Selection`, `IntersectChanged`                                                                                              |
| `filetypes`      | `Filter`: extension taxonomy (formattable/lintable)                                                                                                            |
| `prettierignore` | `Matcher`: full `.prettierignore` gitignore semantics                                                                                                          |
| `sourcefiles`    | `Collector{Tree, Selection, Filter}`: composition of the three above                                                                                           |
| `sidecarproto`   | The typed Go↔TS seam (see below)                                                                                                                               |
| `tsruntime`      | `Assets` (extracted toolchain lifecycle), `Invoker` (spawns the sidecar via `sidecarproto`), `PrettierMigration`                                               |
| `embedded`       | `go:embed` of the staged sidecar per platform; `bin/` must stay a child of this package (staging writes there)                                                 |

## TypeScript sidecar (`packages/ts/sidecar/src`)

| Directory    | Role                                                                                                                                                                                                                                                                                                                |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `kernel/`    | `result.ts` (sanctioned helpers), tagged errors, concurrency pool                                                                                                                                                                                                                                                   |
| `syntax/`    | `SourceDocument` (frozen value object: text + coordinate queries), `SourceParser` (the Zod parse boundary), `AstReader`, `Edit` + `EditApplier`, `node-schema`                                                                                                                                                      |
| `hosts/`     | Embedded-language handling: `EmbeddedBlockSplitter` + `VueScriptScanner`/`MarkdownFenceScanner`, `FileTargetPolicy`                                                                                                                                                                                                 |
| `passes/`    | One class per rule, all implementing `FormattingPass { name; computeEdits(document): Edit[] }`. Policies (`StatementSpacingPolicy`, `ClassMemberPolicy`, `VueReactivityIdioms`) hold the layout knowledge; `drizzle/` holds the vocabulary/scanner/classifier/writer collaborators                                  |
| `pipeline/`  | `PassPipeline`/`PipelineStep`/`IterationBudget` (fixed-point loops are declared here, not hidden in passes); `PipelineFactory` — **the only place pass sequences are named**; `FileFormatter` (host-aware transform), `SourceFileEditor` (read→transform→compare→atomic write), `FormatPipeline`, `SyntaxValidator` |
| `io/`        | `SourceFiles`/`ProcessRunner` ports + Node adapters                                                                                                                                                                                                                                                                 |
| `cli/`       | `CliCommand` contract, `CompositionRoot.production()` (the DI wiring point), command classes, reporters, and the argv DTOs. Entry files are `main()` shims                                                                                                                                                          |
| `sidecar.ts` | The wire entry: dispatches `pipeline`/`oxfmt`/`oxlint` modes                                                                                                                                                                                                                                                        |

Adding a pass: implement `FormattingPass`, register it in `PipelineFactory` —
nothing else changes. The file-set schedule (segment → oxfmt → fluent →
segment fixed-point → validate) lives once, in `FormatAllCommand`.

Module resolution uses wildcard subpath imports (`#sidecar/*` → `./src/*.ts`)
declared in both `sidecar/package.json` and `sidecar/src/package.json` (the
staging script copies `src/` with the inner map). All imports must use the
alias — `alias-specifiers.test.ts` enforces it. The import graph is
cycle-free.

## The Go↔TS seam

The driver spawns the staged `fmtkit-ts-sidecar` executable. Everything on
the wire — the executable name, `.oxfmtrc.json`/`.oxlintrc.json`, the
`pipeline|oxfmt|oxlint` modes, the argv flags, the `FMTKIT_*`/`OX*_BIN`
override env vars, and the stdout summary prefixes the driver parses back —
is defined in `driver/internal/sidecarproto` on the Go side and consumed by
`sidecar.ts` + the `cli/` DTOs on the TS side. Golden argv tests and the
frozen-constants test are the drift tripwire.

Assets flow: `packages/ts/infra/stage-ts-assets.sh` bun-compiles the sidecar
and stages it (plus the oxfmt/oxlint/oxc-parser napi bindings and the bundled
configs) into `driver/internal/embedded/bin/<os>_<arch>/`, which `go:embed`
picks up only under the `fmtkit_sidecar` build tag. Dev builds carry no
assets and use `FMTKIT_SUPPORT_DIR` (see `infra/task.sh`).

## Verification

Every change must keep green: `go test ./...` + golangci-lint (Go),
typecheck + `test:coverage` (TS, ≥90% lines; Go gated packages ≥85%),
`infra/test-binary-smoke.sh` (the only exercise of the embed path), and
`make format-all` with a clean tree afterwards. Behavior-sensitive layers are
pinned by goldens: pipeline stderr transcripts, report text/json/agent
renders, CLI usage/exit codes for both binaries, and the spacing corpus
(`testdata/corpus`). If a golden fails, the code is wrong — goldens are never
regenerated to make a refactor pass.
