# fmtkit

[![Tests](https://github.com/oullin/fmtkit/actions/workflows/tests.yml/badge.svg)](https://github.com/oullin/fmtkit/actions/workflows/tests.yml)
[![Release](https://github.com/oullin/fmtkit/actions/workflows/release.yml/badge.svg)](https://github.com/oullin/fmtkit/actions/workflows/release.yml)

fmtkit is one formatter and one gate for a repository that mixes Go with TypeScript, JavaScript, Vue, HTML, and Markdown. It runs the standard formatters, adds the layout rules they leave alone, lints, scores function complexity, and runs `go vet`. One command reports everything. One command fixes what can be fixed.

This README describes fmtkit 2.0, a rewrite in Rust. Users of the 0.x releases should read [Migrating to 2.0](docs/migrating-to-2.0.md) first.

## What it is

fmtkit is a Rust binary, `fmtkit`, with one small companion, `fmtkit-go-helper`. The two ship together and must stay together.

- **Go.** The helper applies fmtkit's spacing rule, then `gofmt`, then `goimports`, then scores complexity. fmtkit then runs `go vet`.
- **Scripts.** TypeScript and JavaScript (`.ts`, `.tsx`, `.mts`, `.cts`, `.js`, `.jsx`, `.mjs`, `.cjs`) go through Oxlint fixes, the oxc formatter, and fmtkit's structural passes.
- **Hosts.** Vue single-file components, HTML, and Markdown are formatted as whole documents. The scripts, styles, and code fences inside them are formatted too.

The TypeScript side links oxc directly, pinned to `oxlint_v1.86.0` (Oxlint 1.86.0 and oxfmt 0.71.0). It needs no Node.js, no `node_modules`, and no network. The Go side needs no Go toolchain to format. It needs `go` on `PATH` only for `go vet` and for import resolution.

## Why

Standard formatters stop at token layout. They do not insert a blank line before a `return`, order class members, or keep imports grouped. Teams then enforce those rules in review, by hand, and inconsistently. fmtkit enforces them in the formatter, so review can discuss behaviour instead.

fmtkit also replaces a chain of tools with one command. Formatting, lint, complexity, and `go vet` share one file list, one cache, one report, and one exit code. CI and coding agents read one result.

## Who it is for

fmtkit is for teams that keep Go and TypeScript in one repository and want one opinion about layout. It suits teams that accept fmtkit's defaults. It does not suit teams that need Prettier configuration parity or arbitrary ESLint plugins.

## Install

Install fmtkit with Homebrew, a release archive, or Docker. Every channel ships `fmtkit` and `fmtkit-go-helper` together.

### Homebrew

The release workflow publishes a formula to the `oullin/homebrew-fmtkit` tap.

```sh
brew install oullin/fmtkit/fmtkit
```

The formula installs the helper under the formula's `share/fmtkit` directory. fmtkit finds it there.

### Release archives

Each GitHub release carries one archive per target. The targets are `aarch64-apple-darwin`, `x86_64-apple-darwin`, `aarch64-unknown-linux-gnu`, and `x86_64-unknown-linux-gnu`. The archive is named `fmtkit-<target>.tar.xz`. It holds both binaries in one top-level directory.

Place both binaries in the same directory on your `PATH`. fmtkit looks for the helper beside itself.

### Docker

The release workflow publishes `ghcr.io/oullin/fmtkit` for `linux/amd64` and `linux/arm64`. Each release is tagged `v<version>` and `latest`. The image contains fmtkit, the helper, and Go 1.27.1 for `go vet`. It does not contain Git, and it does not need it: fmtkit reads the repository itself.

```sh
docker run --rm -u "$(id -u):$(id -g)" -v "$PWD:/work" ghcr.io/oullin/fmtkit:latest check --all
```

The `-u` flag keeps written files owned by you. The working directory is `/work`. Without arguments, the image prints the help.

### From source

Building from source needs Rust 1.96 or newer and Go 1.27.1.

```sh
git clone https://github.com/oullin/fmtkit.git
cd fmtkit
make build
```

`make build` writes `storage/target/release/fmtkit` and `storage/go-helper/fmtkit-go-helper`. Copy both into one directory on your `PATH`.

### Finding the helper

fmtkit stops with exit code 3 when it cannot find a compatible helper. It searches in this order:

1. `FMTKIT_GO_HELPER`, when set. A path that is not an executable file is an error. There is no fallback.
2. The directory of `fmtkit` as invoked.
3. The directory of `fmtkit` after symlinks are resolved.
4. `../share/fmtkit/` relative to the resolved `fmtkit`. This is the Homebrew layout.
5. `PATH`.

A helper is compatible when it speaks the same protocol and reports the same version. A helper built with the version `dev` passes the version check.

## Quickstart

Run fmtkit from anywhere inside a Git repository. It works on the files you changed.

```sh
fmtkit check          # report on changed files; write nothing
fmtkit format         # fix changed files in place, then report what is left
fmtkit check --all    # report on every file in the repository
fmtkit format --all   # fix every file in the repository
```

A file counts as changed when it differs from `HEAD` in the work tree or the index, or when it is untracked and not ignored. A clean tree therefore checks nothing until you add `--all`.

Use `check --all` in CI. It fails when any file would change and when any finding remains.

## What it does to your code

fmtkit rewrites layout and applies safe lint fixes. It does not change behaviour on purpose. In scripts, every pass is checked: a file whose syntax tree changes in a way the pass did not intend is left untouched and reported as an error.

### Go

Go files go through four steps, in this order:

1. **Spacing.** fmtkit's own rule inserts blank lines around control flow, jump statements, block statements, standalone `var` declarations, and type declarations. It moves type declarations to the top of the file and repairs misplaced `//go:embed` directives. The [spacing rule reference](docs/spacing.md) lists every case with examples.
2. **gofmt.** The standard Go formatter.
3. **goimports.** By default it only groups and sorts imports. It does not add or remove them. Set `resolve_imports = true` under `[go]`, or pass `--resolve-imports`, to let it resolve imports. That mode reads the package directory and the module cache, and it is slow.
4. **Complexity.** The helper scores every function after formatting. See [Complexity](#complexity).

This is real input and output for the spacing rule:

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
	_ = r
	return nil
}
```

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
	_ = r

	return nil
}
```

After formatting, fmtkit runs `go vet` and reports its findings under the rule `go/vet`. With `--all` and no paths, it vets `./...` in every Go module of the repository, except modules inside ignored or `[files] exclude` directories. Otherwise it vets the packages of the Go files in scope. fmtkit skips `go vet` when `go` is not on `PATH`, when `[go] vet = false`, under `--ts`, or when no Go file is in scope, and the report says why. `go vet` analyses whole packages, so it also reads Go files that `[files] exclude` hides from formatting.

### TypeScript and JavaScript

Scripts go through Oxlint, then a fixed schedule of passes:

1. **Lint fixes.** Oxlint applies its safe fixes, in up to 10 rounds. The remaining findings are reported.
2. **Body wrap.** Unbraced `if`, `else`, `with`, and loop bodies get braces. An `else if` stays as it is.
3. **Class reorder.** Class members are ordered as properties, then constructors, then methods.
4. **Declaration reorder.** In a run of consecutive imports or `const` declarations, single-line declarations come first. Multiline ones follow, separated by blank lines.
5. **Blank lines.** Blank lines are inserted between statements where the statement-spacing policy requires them.
6. **oxfmt.** The oxc formatter lays out the code with the options under `[ts.format]`.
7. **Fluent chains.** A chain of two or more calls puts each link on its own line.
8. **Drizzle queries.** In modules that import `drizzle-orm`, query arguments get a fixed layout.
9. **Expanded calls.** A call with an argument that is itself a call, an object, or an array puts one argument on each line.

The schedule repeats until the file stops changing. A file that still changes after three extra rounds is reported as an `idempotency` error and is left untouched.

This is real input and output for a class:

```diff
 export class Store {
-    load(id: string) { return id }
-    private items: string[] = []
-    constructor() {}
+	private items: Array<string> = [];
+
+	constructor() {}
+
+	load(id: string) {
+		return id;
+	}
 }
```

The members were reordered, the body was expanded, and indentation changed to tabs. `string[]` became `Array<string>` through the bundled `typescript/array-type` fix.

This is real input and output for a chain and an unbraced body:

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

### Lint

Oxlint runs on scripts and on Vue files with the bundled policy. The bundled policy enables 197 rules: core ESLint rules, rules from the `typescript`, `oxc`, `unicorn`, `import`, and `react` plugins, and fmtkit's native `perfectionist/*`, `@nkzw/*`, and `no-only-tests/*` rules. fmtkit also implements `anti-slop/*` rules. They are not in the bundled policy; enable them by name.

A rule set to `error` fails the run. A rule set to `warn` is reported but does not fail it. Configure the policy under `[lint]`; see [Configuration](#configuration).

### Vue, HTML, and Markdown

Hosts are formatted as whole documents, close to Prettier's defaults.

- Vue and HTML go through markup_fmt. The `<script>` blocks go through the full script pipeline. The `<style>` blocks go through the oxc CSS formatter.
- Markdown goes through the oxc Markdown formatter. Prose is not rewrapped. Fences tagged `ts`, `typescript`, `tsx`, `mts`, `cts`, `js`, `javascript`, `jsx`, `mjs`, or `cjs` are formatted as scripts. Fences tagged `css`, `postcss`, `scss`, or `less` are formatted as CSS. Every other fence is kept as written.
- Template literals that hold another language are formatted too, as oxc recognises them: `css`, `styled.x`, `gql`, `html`, and `md` tags, among others. A template that does not format is left as written. Angular templates are always left as written.
- Line endings become `\n`, and a non-empty document ends with one.

A host document repeats formatting until it stops changing, with three extra rounds at most.

## What it never touches

fmtkit leaves these files and regions exactly as written:

- Files ignored by `.gitignore` or `.ignore`, and files matched by `[files] exclude`.
- Anything under a `.git`, `node_modules`, or `vendor` directory, at any depth.
- Symbolic links.
- Declaration files: `.d.ts`, `.d.mts`, and `.d.cts`.
- Generated Go files: `*.gen.go`, and any Go or script file with a `// Code generated ... DO NOT EDIT.` line before its first line of code.
- File types it does not format, such as JSON, YAML, TOML, and standalone CSS.
- Vue custom blocks, `<template lang="pug">`, and scripts or styles in Sass, Stylus, or CoffeeScript.
- Markdown fences in languages other than scripts and CSS.
- Tailwind class order.
- Any file that fails to parse, fails a pass, or does not reach a fixed point. fmtkit reports the error and does not write the file.

## Commands

fmtkit has two working commands. `format` writes; `check` only reports.

| Command          | What it does                                                        |
| ---------------- | ------------------------------------------------------------------- |
| `fmtkit format`  | Rewrite files in place, then report what is left to fix by hand.    |
| `fmtkit check`   | Report what `format` would change and every finding. Write nothing. |
| `fmtkit version` | Print the version, for example `fmtkit 2.0.0`.                      |
| `fmtkit help`    | Print the help. `fmtkit <command> --help` prints a command's flags. |

Both `format` and `check` take the same flags:

| Flag                      | Meaning                                                                                  |
| ------------------------- | ---------------------------------------------------------------------------------------- |
| `[PATHS]...`              | Files or directories to cover, changed or not.                                           |
| `--all`                   | Cover every file, not only the changed ones.                                             |
| `--go`                    | Only the Go lane.                                                                        |
| `--ts`                    | Only the TypeScript lane: scripts, Vue, HTML, and Markdown.                              |
| `-j`, `--jobs <N>`        | Worker threads; 0 means one per CPU.                                                     |
| `--no-cache`              | Neither read nor update the cache.                                                       |
| `--format <FORMAT>`       | `text` (the default), `json`, or `agent`.                                                |
| `-q`, `--quiet`           | Print only findings and the summary line.                                                |
| `--color <WHEN>`          | `auto` (the default), `always`, or `never`.                                              |
| `--stdin-filepath <PATH>` | Read a source from stdin as if it were at `PATH`; see [Standard input](#standard-input). |
| `--resolve-imports`       | Let goimports add and remove imports.                                                    |

### Scope

The scope decides which files fmtkit reads.

- **Root.** The root is the enclosing Git work tree. Outside Git, the root is the current directory.
- **Default.** Inside Git, the scope is the changed files. Outside Git, the scope is every file that `.gitignore` and `.ignore` do not exclude.
- **`--all`.** Every tracked file and every untracked file that is not ignored.
- **Paths.** Paths are relative to the current directory. A named file, or every file under a named directory, is covered whether or not it changed. `fmtkit check main.go` checks `main.go` even on a clean checkout.
- **Lanes.** `--go` and `--ts` restrict the run to one lane. They cannot be combined.

A path outside the root is a usage error. A path that does not exist is reported under `missing` and fails the run.

### Standard input

`--stdin-filepath` formats one source for an editor. The path chooses the language. With `format`, the formatted text goes to stdout. With `check`, nothing goes to stdout, and the exit code is 1 when formatting would change the source. Standard input is formatted only: it is not scored for complexity, and lint findings that fixes cannot remove are not reported.

```sh
printf 'const a = {b:1}\n' | fmtkit format --stdin-filepath web/x.ts
```

```text
const a = { b: 1 };
```

A syntax error or an unsupported file type exits with code 1.

### Jobs

fmtkit takes the first of these that is set: `--jobs`, `FMTKIT_JOBS`, and `jobs` in `fmtkit.toml`. A value of 0, or none at all, means one worker per CPU. `FMTKIT_JOBS` must be a whole number.

### Cache

fmtkit caches outcomes so unchanged files are not processed again. `format` caches a file only when it had nothing to write. `check` writes nothing, so it also caches files that formatting would change. A cache entry depends on the file's bytes, its path, the fmtkit version, the configuration, and the mode.

fmtkit also records each cached file's size, inode, and modification and change times. When those still match, it reuses the outcome without opening the file, as git does with its index. A file modified less than two seconds before a run is read in full, because some file systems record times only to the nearest two seconds.

A clean `go vet` run is cached per module. The entry depends on the `go` executable, the environment that `go` reads (every `GO*` and `CGO_*` variable, the C toolchain variables, `HOME`, and the `go env -w` file), the packages vetted, and the size, inode, and modification and change times of every file in the module. That includes files git ignores. It leaves out the directories `go` skips (`testdata`, and names starting with `.` or `_`), `node_modules`, and nested modules. While none of these change, fmtkit skips `go vet` for that module and reports the earlier pass. A module that reads code from elsewhere on disk, through a `go.work` workspace or a `replace` with a local path, is always vetted. Failed runs are never cached, and neither is a run over a file modified less than two seconds earlier.

The cache lives in `<cache dir>/fmtkit/v2/`. The cache directory is `~/Library/Caches` on macOS and `$XDG_CACHE_HOME` or `~/.cache` on Linux. `FMTKIT_CACHE_DIR` replaces the whole `<cache dir>/fmtkit/v2` prefix. `--no-cache` bypasses the cache for one run.

### Environment variables

| Variable           | Effect                                                                        |
| ------------------ | ----------------------------------------------------------------------------- |
| `FMTKIT_CONFIG`    | Path of the configuration file. A relative path is resolved against the root. |
| `FMTKIT_JOBS`      | Worker threads, when `--jobs` is not given.                                   |
| `FMTKIT_CACHE_DIR` | Directory of the cache.                                                       |
| `FMTKIT_GO_HELPER` | Path of `fmtkit-go-helper`.                                                   |
| `NO_COLOR`         | Disables colour when `--color` is `auto`.                                     |

## Complexity

fmtkit scores every function with two numbers and fails the run when either exceeds its limit. Cyclomatic complexity counts the paths through a function. Cognitive complexity weighs how hard the function is to read, and it charges more for nesting.

| Number     | Default limit | Go scorer      | Script scorer |
| ---------- | ------------- | -------------- | ------------- |
| Cyclomatic | 15            | gocyclo 0.6.0  | fmtkit        |
| Cognitive  | 20            | gocognit 1.2.1 | fmtkit        |

A function breaches a limit when its score is strictly greater than the limit. A limit of 0 disables that number.

fmtkit scores `.go`, `.ts`, `.tsx`, `.mts`, `.cts`, `.js`, and `.jsx` files. It does not score `.mjs`, `.cjs`, Vue, or test files. Test files are `*_test.go` and any file whose name contains `.test.` or `.spec.`.

A finding names the function by its key and gives its line in the file on disk. After `format`, that is the formatted text; under `check`, it is the text as written, the same text that lint findings refer to:

```text
[complexity/cognitive] line 7: web/store.ts#grade scores 37 (limit 20)
[complexity/cyclomatic] line 7: web/store.ts#grade scores 22 (limit 15)
```

### Function keys

A key is the file path relative to the root, then `#`, then the function name.

- A Go method includes its receiver: `store/store.go#(*Store).Get`.
- A class method includes its class: `web/store.ts#Store.get size`.
- A name that occurs more than once in a file gets its line: `web/handlers.ts#handler:6`.
- An anonymous function adds its cognitive score to the named function around it. It keeps its own cyclomatic score. The key reports the worse of the two.

### The allow list

The allow list exempts named functions from both limits. Each entry needs a reason, so the exemption stays honest.

```toml
[[complexity.allow]]
key = "web/store.ts#grade"
reason = "One branch per grade band; becomes a lookup table."
```

An entry that matches no function is reported under `complexity/allow` and fails the run:

```text
[complexity/allow] allow entry "web/store.ts#gone" matches no function
```

fmtkit judges an entry only when its file is in scope or no longer exists. Treat the list as a baseline that only shrinks.

The bundled lint policy also enables `eslint/complexity` with a maximum of 12. That rule is separate from fmtkit's limits. Turn it off with `"eslint/complexity" = "off"` under `[lint.rules]` if you want one source of truth.

## Configuration

fmtkit reads one file, `fmtkit.toml`, at the repository root. `FMTKIT_CONFIG` names another file. Every key is optional, and an unknown key is an error.

This file sets every default and adds one lint rule and one allow entry:

```toml
jobs = 0

[files]
exclude = ["node_modules/", "vendor/"]

[go]
spacing = true
gofmt = true
goimports = true
resolve_imports = false
vet = true

[ts.format]
use_tabs = true
tab_width = 4
print_width = 200
single_quote = true
semi = true
trailing_comma = "all"
arrow_parens = "always"

[lint]
bundled = true
ignore = []

[lint.rules]
eqeqeq = ["error", "always"]
"anti-slop/no-object-parameters" = "warn"

[complexity]
cyclomatic = 15
cognitive = 20

[[complexity.allow]]
key = "web/store.ts#grade"
reason = "One branch per grade band; becomes a lookup table."
```

### `jobs`

The number of worker threads. 0 means one per CPU.

### `[files]`

`exclude` lists gitignore-style patterns, relative to the root. They apply to both lanes, on top of `.gitignore`. A pattern without a slash matches at any depth. Setting `exclude` replaces the default list, so keep `node_modules/` and `vendor/` if you need them.

### `[go]`

| Key               | Default | Effect                                |
| ----------------- | ------- | ------------------------------------- |
| `spacing`         | `true`  | Apply the spacing rule.               |
| `gofmt`           | `true`  | Apply gofmt.                          |
| `goimports`       | `true`  | Apply goimports.                      |
| `resolve_imports` | `false` | Let goimports add and remove imports. |
| `vet`             | `true`  | Run `go vet`.                         |

### `[ts.format]`

These options drive the oxc formatter for scripts, Vue, HTML, Markdown, and CSS.

| Key              | Default    | Values                     |
| ---------------- | ---------- | -------------------------- |
| `use_tabs`       | `true`     | `true` or `false`          |
| `tab_width`      | `4`        | 1 or more                  |
| `print_width`    | `200`      | 1 or more                  |
| `single_quote`   | `true`     | `true` or `false`          |
| `semi`           | `true`     | `true` or `false`          |
| `trailing_comma` | `"all"`    | `"all"`, `"es5"`, `"none"` |
| `arrow_parens`   | `"always"` | `"always"`, `"avoid"`      |

### `[lint]`

- `bundled` keeps the bundled policy. Set it to `false` to start from no rules.
- `ignore` lists gitignore-style patterns of files that lint skips. fmtkit still formats those files.
- `rules` maps a rule name to a setting.

A setting is a severity, or an array of a severity followed by the rule's options. The severities are `"off"` or `"allow"`, `"warn"`, and `"error"` or `"deny"`. A rule may be named with or without its plugin: `eqeqeq` and `eslint/eqeqeq` are the same rule. An unknown rule is a configuration error.

A setting replaces the bundled setting completely, options included. `"typescript/array-type" = "warn"` drops the bundled `generic` option and falls back to the rule's own default. Repeat the options when you only want to change the severity.

### `[complexity]`

`cyclomatic` and `cognitive` set the limits. `allow` holds the allow list. Each entry has a `key`, which must contain `#`, and a `reason`. Duplicate keys are an error. See [Complexity](#complexity).

## Output formats

fmtkit prints one report in one of three formats. All three carry the same findings in the same order.

### Text

Text is the default. It groups findings by file. A progress bar appears on stderr only when stderr is a terminal and `--quiet` is not set.

```text
  Checked 2 file(s).

  main.go
    [spacing] line 7: missing blank line before type definition
    [spacing] line 8: missing blank line before range loop
    [spacing] line 11: missing blank line before if statement
    [spacing] line 14: missing blank line after if statement
    [spacing] line 16: missing blank line before return statement
    ✓ would apply spacing

  web/store.ts
    [typescript/array-type] line 3:20: Array type using 'string[]' is forbidden. Use 'Array<string>' instead.
    [complexity/cognitive] line 7: web/store.ts#grade scores 37 (limit 20)
    [complexity/cyclomatic] line 7: web/store.ts#grade scores 22 (limit 15)
    [eslint/complexity] line 7:8: function `grade` has a complexity of 22. Maximum allowed is 12.
    [unicorn/catch-error-name] line 19:70: The catch parameter "e" should be named "cause"
    [unicorn/prefer-optional-catch-binding] line 19:70: Prefer omitting the catch binding parameter if it is unused
    [eslint/eqeqeq] line 23:38: Expected === and instead saw ==
    ✓ would apply lint, class-reorder, blank-lines, oxfmt

  go vet passed on 1 target(s).

  Result: fail. 2 file(s), 2 changed, 5 violation(s), 5 lint, 2 complexity, 0 vet, 0 error(s).
```

### JSON

`--format json` prints one pretty-printed object with `"schema": 2`. The `files` array lists only files with something to report.

```json
{
  "schema": 2,
  "mode": "check",
  "result": "fail",
  "summary": {
    "files": 2,
    "changed": 1,
    "violations": 1,
    "lint": 1,
    "complexity": 0,
    "vet": 0,
    "errors": 0,
    "missing": 0
  },
  "files": [
    {
      "file": "main.go",
      "lang": "go",
      "changed": true,
      "applied": [
        "spacing"
      ],
      "violations": [
        {
          "rule": "spacing",
          "line": 5,
          "message": "missing blank line before if statement",
          "severity": "error"
        }
      ]
    },
    {
      "file": "web/same.ts",
      "lang": "ts",
      "lint": [
        {
          "rule": "eslint/eqeqeq",
          "line": 1,
          "column": 47,
          "message": "Expected === and instead saw ==",
          "severity": "error"
        }
      ]
    }
  ],
  "complexity": [],
  "vet": {
    "status": "pass",
    "targets": [
      "./..."
    ],
    "errors": []
  },
  "missing": []
}
```

`result` is `pass`, `fixed`, or `fail`. `vet.status` is `pass`, `fail`, or `skipped`; a skipped run adds a `reason`.

### Agent

`--format agent` prints one line per finding and a final summary line. It suits coding agents and `grep`. Each line is `path[:line[:column]] rule message`. Warnings carry a `warning:` prefix on the message.

```text
main.go:7 spacing missing blank line before type definition
main.go:8 spacing missing blank line before range loop
main.go:11 spacing missing blank line before if statement
main.go:14 spacing missing blank line after if statement
main.go:16 spacing missing blank line before return statement
main.go format would apply spacing
web/store.ts:3:20 typescript/array-type Array type using 'string[]' is forbidden. Use 'Array<string>' instead.
web/store.ts:7 complexity/cognitive web/store.ts#grade scores 37 (limit 20)
web/store.ts:7 complexity/cyclomatic web/store.ts#grade scores 22 (limit 15)
web/store.ts:7:8 eslint/complexity function `grade` has a complexity of 22. Maximum allowed is 12.
web/store.ts:19:70 unicorn/catch-error-name The catch parameter "e" should be named "cause"
web/store.ts:19:70 unicorn/prefer-optional-catch-binding Prefer omitting the catch binding parameter if it is unused
web/store.ts:23:38 eslint/eqeqeq Expected === and instead saw ==
web/store.ts format would apply lint, class-reorder, blank-lines, oxfmt
fmtkit schema=2 mode=check result=fail files=2 changed=2 violations=5 lint=5 complexity=2 vet=0 errors=0 missing=0
```

A `go vet` finding looks like this:

```text
vet.go:6:14 go/vet fmt.Printf format %d has arg "x" of wrong type string
```

## Exit codes

The exit code tells CI what happened. Only 0 is a success.

| Code | Meaning                                                                                                                                         |
| ---- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| 0    | Nothing to report, or `format` fixed everything it found.                                                                                       |
| 1    | Findings remain: a file `check` would change, a lint finding, a violation, a complexity breach, a vet error, or a file error.                   |
| 2    | Usage or configuration error: a bad flag, an invalid `fmtkit.toml`, an unknown lint rule, an invalid `FMTKIT_JOBS`, or a path outside the root. |
| 3    | Internal error: the helper is missing, incompatible, or crashed, or the report could not be written.                                            |

`format` exits 0 after it writes changes, as long as nothing remains to fix by hand. Lint warnings are reported but never change the exit code. A path that does not exist exits 1.

## Development

Development needs Rust 1.96 or newer, Go 1.27.1, and Bash. Every build artifact lives under `storage/`.

| Target            | What it does                                               |
| ----------------- | ---------------------------------------------------------- |
| `make build`      | Build fmtkit and the Go helper into `storage/`.            |
| `make format`     | Format `ARGS`; the default `.` means the whole repository. |
| `make format-all` | Format the whole repository.                               |
| `make check`      | Check the whole repository, or `ARGS`, without writing.    |
| `make lint`       | Run rustfmt, clippy, gofmt, and `go vet`, all read-only.   |
| `make test`       | Run the Rust and Go test suites.                           |
| `make version`    | Print the version the working tree builds as.              |

`scripts/task.sh` backs every target. It also provides `self-check` and `coverage`. The coverage gate requires 90 percent line coverage for Rust and for Go.

Continuous integration runs these workflows:

- `tests.yml` runs lint, tests on Ubuntu and macOS, coverage, a binary smoke test, a Docker smoke test, and the self-check.
- `bench.yml` compares a pull request with its base on a large Go and TypeScript corpus. It fails when the pull request is more than 10 percent slower.
- `fuzz.yml` runs every night. It fuzzes the edit set, the Go protocol, the configuration, the script pipeline, and the host formatters for 10 minutes each.

### Releases

The version in `Cargo.toml` is the release version. When a commit lands on `main` with a version that has no tag, `tag.yml` tags it `v<version>` and starts `release.yml`. The release builds the archives with cargo-dist, publishes the Homebrew formula, and publishes the Docker image. A version must be strict semver and must sort above the latest tag. Tagging runs only while the repository variable `RELEASES` is set to `enabled` (`gh variable set RELEASES --body enabled`); without it, merges release nothing.

## fmtkit formats itself

This repository is formatted and checked by the fmtkit it builds. `make format` and `make check` build fmtkit from the working tree and run it on the repository. The repository's own `fmtkit.toml` excludes the test inputs, because formatting them would change what the tests test.

CI runs `scripts/task.sh self-check`. It formats the clean tree with `--all --no-cache` and fails when any file moves.

## How the code is organised

The workspace has twelve crates and one Go module. Only the engine reads and writes source files.

| Path              | Role                                                                                  |
| ----------------- | ------------------------------------------------------------------------------------- |
| `crates/cli`      | The `fmtkit` binary: argument parsing, exit codes, and the progress bar.              |
| `crates/engine`   | The run: scope, lanes, the worker pool, complexity, `go vet`, and every file write.   |
| `crates/discover` | The repository root, the changed set, ignore rules, and file classification.          |
| `crates/config`   | `fmtkit.toml` and its validation.                                                     |
| `crates/core`     | Shared types: languages, findings, edits, and line indexes.                           |
| `crates/ts`       | The script pipeline: the structural passes and the oxc formatter.                     |
| `crates/hosts`    | Vue, HTML, and Markdown documents, and CSS inside them.                               |
| `crates/lint`     | Oxlint, the bundled policy, and the native rules.                                     |
| `crates/go`       | The helper process and `go vet`.                                                      |
| `crates/cache`    | The outcome cache.                                                                    |
| `crates/report`   | The text, JSON, and agent reports.                                                    |
| `crates/testkit`  | Test support.                                                                         |
| `go/helper`       | `fmtkit-go-helper`: spacing, gofmt, goimports, and complexity, over stdin and stdout. |
| `fixtures`        | Shared test inputs, such as the complexity fixtures.                                  |
| `fuzz`            | Fuzz targets, seeds, and dictionaries.                                                |

The helper protocol is documented in `go/helper/proto/PROTOCOL.md`. The oxc pin and its upgrade checklist are in [Upgrading oxc](docs/oxc-upgrade.md). Known defects in upstream formatters are in [Known upstream issues](docs/known-issues.md).

## Ground rules

The code follows a short list of Rust rules. Reviews enforce them, and the compiler enforces most of them.

- **No unsafe code.** The workspace sets `unsafe_code = "forbid"`.
- **No ignored results.** `unused_must_use` is denied.
- **Pedantic clippy.** `clippy::pedantic` is denied, with a few documented exceptions.
- **No panics on input.** Library code returns errors. `expect` appears only on invariants, constant patterns, and the bundled policy. Release builds abort on panic.
- **Edits never overlap.** Passes describe changes as edits. Applying overlapping edits is an error. A pass that finds nested targets edits the outer one and leaves the inner one for the next round.
- **One writer.** Only the engine touches source files. Writes go to a temporary file that is renamed into place, and they keep the file's permissions. The helper never writes files.
- **Deterministic output.** Reports are sorted, so two runs over the same tree print the same report.
- **Checked passes.** Every reparse is compared with the original syntax tree. A pass that changes meaning leaves the file untouched.

## License

fmtkit is released under the [MIT License](LICENSE).
