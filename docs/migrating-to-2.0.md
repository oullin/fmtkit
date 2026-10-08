# Migrating to fmtkit 2.0

fmtkit 2.0 is a rewrite in Rust, and it is not backwards compatible. It replaces the 0.x releases, the last of which is v0.13.5. Upgrading takes five steps: install 2.0, translate the configuration into `fmtkit.toml`, update every script and CI job that calls fmtkit, re-baseline the complexity allow list, and reformat the repository once in its own commit.

## Upgrade checklist

Follow these steps in order. Each step links to its section.

1. Remove the 0.x install and install 2.0. See [Install changes](#install-changes).
2. Write `fmtkit.toml` at the repository root from `config.yml`, `.oxlintrc*`, `.oxfmtrc*`, and `.prettierignore`. Then delete those files. See [Configuration mapping](#configuration-mapping).
3. Rewrite every fmtkit command in scripts, hooks, and CI. See [CLI mapping](#cli-mapping).
4. Update every consumer of the JSON or agent report, and every check of the exit code. See [Reports and exit codes](#reports-and-exit-codes).
5. Re-baseline the complexity allow list. See [Re-baselining the complexity allow list](#re-baselining-the-complexity-allow-list).
6. Run `fmtkit format --all` and commit the result on its own. See [One-time reformat](#one-time-reformat).

## What changed at a glance

Everything a user touches changed. The formatting rules themselves mostly did not.

| Area           | 0.x                                                                          | 2.0                                                                   |
| -------------- | ---------------------------------------------------------------------------- | --------------------------------------------------------------------- |
| Implementation | Go driver with an embedded TypeScript toolchain built by Bun                 | Rust binary with a small Go helper                                    |
| Binaries       | `fmtkit` and the Go-only `fmtkit-go`                                         | `fmtkit` and `fmtkit-go-helper`, always shipped together              |
| Commands       | `format`, `format-all`, `ts`, `lint`, `check`, `complexity`, `go`            | `format` and `check`                                                  |
| Default scope  | Changed files for `format`; every file for `check`, `lint`, and `complexity` | Changed files for every command without paths; `--all` for every file |
| Configuration  | `config.yml`, Oxlint configs, oxfmt or Prettier configs, `.prettierignore`   | One `fmtkit.toml` at the repository root                              |
| Reports        | JSON without a schema field; agent output as indented JSON                   | JSON with `"schema": 2`; agent output as one line per finding         |
| Exit codes     | 0 and 1 per command, and 2 for an unknown `fmtkit` subcommand                | 0, 1, 2, and 3, the same for both commands                            |
| Node.js        | Needed for import-based Oxlint configs                                       | Never needed                                                          |
| Releases       | Tags derived from conventional commits                                       | Tags follow the version in `Cargo.toml`                               |

## Install changes

Every install channel changed shape. Remove the 0.x install first.

- **Homebrew.** 0.x shipped a cask. 2.0 ships a formula in the same tap. Run `brew uninstall --cask fmtkit`, then `brew install oullin/fmtkit/fmtkit`.
- **Release archives.** 0.x published `fmtkit_<version>_<os>_<arch>.tar.gz` with one binary. 2.0 publishes `fmtkit-<target>.tar.xz`, for example `fmtkit-x86_64-unknown-linux-gnu.tar.xz`. The archive holds `fmtkit` and `fmtkit-go-helper` in one top-level directory. Install both into the same directory.
- **Docker.** The image is still `ghcr.io/oullin/fmtkit`, tagged `v<version>` and `latest`. It still mounts the repository at `/work`. It no longer contains Git, because fmtkit reads the repository itself. It still contains Go for `go vet`.
- **Go install.** `go install go.ollin.sh/fmtkit/driver/cmd/fmtkit-go@latest` is retired. There is no Go-only binary in 2.0.

fmtkit 2.0 needs its helper. When it cannot find a compatible `fmtkit-go-helper`, it exits with code 3. The [README](../README.md#finding-the-helper) lists where it looks.

## CLI mapping

2.0 has two commands. `format` writes and reports. `check` only reports. Each one covers every lane, lint, complexity, and `go vet`.

| 0.x                                        | 2.0                                       | Notes                                                    |
| ------------------------------------------ | ----------------------------------------- | -------------------------------------------------------- |
| `fmtkit format [paths]`                    | `fmtkit format [paths]`                   | Same scope: changed files.                               |
| `fmtkit format --go` or `--ts`             | `fmtkit format --go` or `--ts`            | The two flags can no longer be combined.                 |
| `fmtkit format-all`                        | `fmtkit format --all`                     |                                                          |
| `fmtkit ts [paths]`                        | `fmtkit format --ts --all [paths]`        |                                                          |
| `fmtkit lint [paths]`                      | `fmtkit check --ts --all [paths]`         | Also reports formatting. There is no lint-only mode.     |
| `fmtkit check [paths]`                     | `fmtkit check --go --all [paths]`         | 0.x `check` was Go only and covered every file.          |
| `fmtkit complexity [--go\|--ts] [paths]`   | `fmtkit check --all [--go\|--ts] [paths]` | There is no complexity-only mode.                        |
| `fmtkit go check` or `fmtkit-go check`     | `fmtkit check --go --all`                 |                                                          |
| `fmtkit go format` or `fmtkit-go format`   | `fmtkit format --go --all`                |                                                          |
| `fmtkit go sources` or `fmtkit-go sources` | None                                      | Retired.                                                 |
| `fmtkit --version` or `fmtkit -version`    | `fmtkit version`                          | `--version` is now a usage error with exit code 2.       |
| `--config <file>`                          | `FMTKIT_CONFIG=<file>`                    | A relative path is resolved against the repository root. |
| `--cwd <dir>`                              | None                                      | Run fmtkit from inside the repository instead.           |
| `--jobs <n>`, `FMTKIT_JOBS`                | `--jobs <n>`, `FMTKIT_JOBS`               | 0 still means one worker per CPU.                        |
| `--format text\|json\|agent`               | `--format text\|json\|agent`              | The JSON and agent shapes changed.                       |
| `--quiet`                                  | `--quiet`                                 |                                                          |

### Scope

Three scope rules changed. They decide which files a CI job actually checks.

- **Every command defaults to the changed files.** In 0.x, only `format` was limited to changed files. `check`, `lint`, and `complexity` covered every file. In 2.0, `fmtkit check` without paths covers only files that differ from `HEAD`, plus untracked files. A clean checkout therefore passes vacuously. CI must pass `--all`. Named paths still cover every file under them, changed or not, as 0.x `check` did, and a path that does not exist still fails the run.
- **Git is optional.** 0.x `format` needed a Git work tree. Outside Git, 2.0 covers every file that `.gitignore` and `.ignore` do not exclude.
- **Hidden directories are covered.** 0.x skipped every hidden directory. 2.0 walks them and relies on `.gitignore`, `.ignore`, and `[files] exclude` instead. Only `.git`, `node_modules`, and `vendor` are always skipped. Exclude any hidden directory you do not want formatted.

## Configuration mapping

2.0 reads one file: `fmtkit.toml` at the repository root. 0.x looked for `config.yml` in the working directory. An unknown key in `fmtkit.toml` is an error, so a mistranslated key fails loudly.

### From `config.yml`

| 0.x key                 | 2.0 key                   | Notes                                                                        |
| ----------------------- | ------------------------- | ---------------------------------------------------------------------------- |
| `rules.spacing.enabled` | `[go] spacing`            |                                                                              |
| `vet.enabled`           | `[go] vet`                |                                                                              |
| `formatters.gofmt`      | `[go] gofmt`              |                                                                              |
| `formatters.goimports`  | `[go] goimports`          | goimports no longer resolves imports by default. See `[go] resolve_imports`. |
| `exclude`               | `[files] exclude`         | A directory name `generated` becomes the pattern `"generated/"`.             |
| `not_path`              | `[files] exclude`         | A substring becomes a gitignore-style pattern. See the note below.           |
| `not_name`              | `[files] exclude`         | A glob such as `*.pb.go` stays `"*.pb.go"`.                                  |
| `concurrency`           | `jobs`                    | 0 still means one worker per CPU.                                            |
| `complexity.cyclomatic` | `[complexity] cyclomatic` |                                                                              |
| `complexity.cognitive`  | `[complexity] cognitive`  |                                                                              |
| `complexity.allow`      | `[[complexity.allow]]`    | Same `key` and `reason`. Re-baseline the keys.                               |

`[files] exclude` follows `.gitignore` syntax. A pattern with a slash at its start or in its middle is anchored to the repository root. `"third_party/generated/"` therefore excludes `third_party/generated/` but not `a/third_party/generated/`. 0.x `not_path` matched a substring anywhere in the path. Write `"**/third_party/generated/"` to keep that behaviour.

`[files] exclude` now applies to both lanes. Setting it replaces the default list, so keep `"node_modules/"` and `"vendor/"` in it.

### From Oxlint configuration

| 0.x                                                            | 2.0             | Notes                                                    |
| -------------------------------------------------------------- | --------------- | -------------------------------------------------------- |
| `rules` in `.oxlintrc`, `.oxlintrc.json`, or `.oxlintrc.jsonc` | `[lint.rules]`  | Same rule names, severities, and option arrays.          |
| `ignorePatterns`                                               | `[lint] ignore` | Ignored files are still formatted.                       |
| `oxlint.config.ts` and `oxlint.config.mts`                     | `[lint.rules]`  | Copy the effective rule settings by hand.                |
| Nested configs in subdirectories                               | None            | Rules apply to the whole repository.                     |
| `FMTKIT_OXLINTRC`                                              | `[lint.rules]`  |                                                          |
| `plugins`, `jsPlugins`, `extends`, `overrides`                 | None            | Only the bundled plugins and native rules are available. |

A setting in `[lint.rules]` replaces the bundled setting completely, options included. A bare severity is therefore easy to get wrong. `"typescript/array-type" = "warn"` drops the bundled `generic` option. Write `"typescript/array-type" = ["warn", "generic"]` to change only the severity.

One lint behaviour differs: `[lint] bundled = false` starts from no rules at all. As in 0.x, only rules set to `error` fail the run; warnings are reported.

### From oxfmt and Prettier configuration

2.0 reads no oxfmt or Prettier file. The defaults under `[ts.format]` equal the bundled `.oxfmtrc.json` of 0.x, so a repository that used the bundled style needs no section at all.

| `.oxfmtrc.json` key | `[ts.format]` key |
| ------------------- | ----------------- |
| `useTabs`           | `use_tabs`        |
| `tabWidth`          | `tab_width`       |
| `printWidth`        | `print_width`     |
| `singleQuote`       | `single_quote`    |
| `semi`              | `semi`            |
| `trailingComma`     | `trailing_comma`  |
| `arrowParens`       | `arrow_parens`    |

No other formatter option exists. 0.x translated a Prettier configuration automatically when no oxfmt configuration was present. 2.0 does not. A repository that relied on a Prettier configuration will see its style change to the fmtkit defaults, except for the seven options above.

### From `.prettierignore`

Move its patterns into `[files] exclude`. They then apply to the Go lane too.

### A translated example

This `config.yml`:

```yaml
exclude:
    - generated
not_path:
    - third_party/generated
not_name:
    - '*.pb.go'
concurrency: 4
complexity:
    cyclomatic: 15
    cognitive: 20
    allow:
        - key: 'internal/envcfg/envcfg.go#(*Source).Read'
          reason: 'One err check per field; becomes a table in the next pass.'
```

becomes this `fmtkit.toml`:

```toml
jobs = 4

[files]
exclude = ["node_modules/", "vendor/", "generated/", "**/third_party/generated/", "*.pb.go"]

[complexity]
cyclomatic = 15
cognitive = 20

[[complexity.allow]]
key = "internal/envcfg/envcfg.go#(*Source).Read"
reason = "One err check per field; becomes a table in the next pass."
```

## Reports and exit codes

The machine-readable reports changed shape. Every consumer must be updated.

### JSON

The 0.x JSON had no schema field and printed on one line. 2.0 prints pretty JSON with `"schema": 2`.

| 0.x                                    | 2.0                                                                                                    |
| -------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| None                                   | `schema`, always `2`                                                                                   |
| None                                   | `mode`: `format` or `check`                                                                            |
| `result`                               | `result`: `pass`, `fixed`, or `fail`                                                                   |
| `formatter.files`, `formatter.changed` | `summary`, with counts of files, changes, violations, lint, complexity, vet, errors, and missing paths |
| `formatter.results[]`                  | `files[]`, listing only files with something to report                                                 |
| `formatter.results[].applied`          | `files[].applied`                                                                                      |
| `formatter.results[].violations`       | `files[].violations`, each with a `severity`                                                           |
| None                                   | `files[].lang`, `files[].lint`, and `files[].error`                                                    |
| `complexity`                           | `complexity[]`                                                                                         |
| `vet.status`                           | `vet.status`, plus `reason`, `targets`, and `errors`                                                   |
| None                                   | `missing[]`, the paths that do not exist                                                               |

### Agent

0.x printed the agent report as indented JSON. 2.0 prints one line per finding and a summary line. Each line has the shape `path[:line[:column]] rule message`.

```text
main.go:5 spacing missing blank line before if statement
main.go format would apply spacing
web/same.ts:1:47 eslint/eqeqeq Expected === and instead saw ==
fmtkit schema=2 mode=check result=fail files=2 changed=1 violations=1 lint=1 complexity=0 vet=0 errors=0 missing=0
```

### Text

0.x printed a `Formatter` section and a `Vet` section, each with its own result. 2.0 prints one report with one result line.

### Exit codes

Both commands share one set of exit codes in 2.0.

| Code | Meaning                                                   |
| ---- | --------------------------------------------------------- |
| 0    | Nothing to report, or `format` fixed everything it found. |
| 1    | Findings remain, or a named path does not exist.          |
| 2    | Usage or configuration error.                             |
| 3    | Internal error, such as a missing or incompatible helper. |

The 0.x README stated that `format` failed only on a genuine error. In 2.0, `format` exits 1 when anything remains that it cannot fix: a lint finding, a complexity breach, a `go vet` error, or a file error. A pre-commit hook that runs `format` now blocks on those findings.

## Retired

These 0.x features have no 2.0 equivalent:

- The `fmtkit-go` binary and its `go install` route.
- The `format-all`, `ts`, `lint`, `complexity`, and `go` commands.
- `go sources` and its `--include-declarations` flag.
- The `--config` and `--cwd` flags, and the `--version` and `-version` flags.
- `config.yml`, `.oxlintrc*`, `oxlint.config.ts`, `oxlint.config.mts`, nested Oxlint configs, and `FMTKIT_OXLINTRC`.
- `.oxfmtrc*`, `FMTKIT_OXFMTRC`, and the automatic translation of Prettier configuration.
- `.prettierignore`.
- `FMTKIT_SUPPORT_DIR` and the embedded TypeScript toolchain that was extracted to the user cache on first run.
- Node.js support for import-based Oxlint configs, and custom Oxlint JS plugins.
- The Go packages that 0.x published under `go.ollin.sh/fmtkit`, such as `go.ollin.sh/fmtkit/driver`. The 2.0 helper module is internal.
- The Homebrew cask and the `.tar.gz` release archives.

## Formatting style changes

Most formatting is unchanged. These changes can move code on the first 2.0 run:

- **goimports no longer resolves imports.** 0.x let goimports add and remove imports. 2.0 only groups and sorts them. Set `[go] resolve_imports = true` or pass `--resolve-imports` to restore the 0.x behaviour.
- **Scripts are formatted in process.** 2.0 links oxc at `oxlint_v1.86.0`, the release of Oxlint 1.86.0 and oxfmt 0.71.0, instead of running oxfmt through an embedded Node toolchain. The `[ts.format]` defaults equal the 0.x bundled style.
- **Hosts use new formatters.** Vue and HTML go through markup_fmt. Markdown goes through the oxc Markdown formatter. CSS inside them goes through the oxc CSS formatter. Edge cases can lay out differently from 0.x.
- **Project formatter configs are ignored.** A repository that relied on its own oxfmt or Prettier configuration moves to the fmtkit defaults.
- **Hidden directories are formatted.** Files under hidden directories are now in scope unless they are ignored.
- **Unparseable files are errors.** A script that oxc cannot parse is reported under the rule `syntax` and left untouched.

## Defects fixed deliberately

2.0 fixes these 0.x defects on purpose. Each fix can change output or findings.

- **Overlapping edits.** 0.x applied text edits without rejecting overlaps, and its class and declaration reordering did not filter overlapping edits. 2.0 rejects overlapping edits. A pass that meets nested targets edits the outer one and leaves the inner one to the next round.
- **Check mode stopped early.** In check mode, the 0.x TypeScript pipeline stopped at the first pass that reported changes, so later passes went unreported. 2.0 runs the whole pipeline and reports every step that would apply.
- **Complexity key of an anonymous function.** When a function name occurs more than once in a file, its key carries its line, such as `handler:6`. 0.x reported an anonymous function inside it under `handler`. 2.0 reports it under `handler:6`.
- **Order of `//go:embed` findings.** 2.0 reports misplaced `//go:embed` directives in source order.

## Releases follow `Cargo.toml`

The version in `Cargo.toml` is the release version. 0.x derived each tag from conventional commit messages. In 2.0, a commit on `main` whose version has no tag yet is tagged `v<version>`, and that tag starts the release. Commit messages no longer decide the version.

Pin a version tag in CI, such as `ghcr.io/oullin/fmtkit:v2.0.0`.

## Re-baselining the complexity allow list

Expect the allow list to need a new baseline. Scores and keys match 0.x except for the anonymous function fix above, but the key prefixes can change.

- **Keys are relative to the repository root.** 0.x computed Go keys relative to the working directory. A repository that ran fmtkit from a subdirectory must rewrite those keys.
- **Moved files change keys.** fmtkit's own repository reset its list to `allow = []`, because the rewrite moved every file in it.

Re-baseline in four steps:

1. Translate the allow list into `[[complexity.allow]]` entries.
2. Run `fmtkit check --all --format agent`.
3. Fix or remove every `complexity/allow` line. Each one names an entry that matches no function.
4. Add an entry, with a reason, for each `complexity/cyclomatic` or `complexity/cognitive` breach that you cannot fix now.

The bundled lint policy also enables `eslint/complexity` with a maximum of 12. It reports separately from the allow list. Set `"eslint/complexity" = "off"` under `[lint.rules]` if you want the allow list to be the only complexity gate.

## One-time reformat

Reformat the whole repository once, in a commit of its own. This keeps formatting churn out of review and out of `git blame`.

```sh
fmtkit format --all
git add -A
git commit -m "Reformat with fmtkit 2.0"
fmtkit check --all
```

`format` exits 1 when findings remain that it cannot fix. Commit the reformat anyway, then fix the findings in later commits. Add the commit to `.git-blame-ignore-revs` if your team uses one.

## Not covered by this guide

These points could not be confirmed from the repository and are left out:

- The exact layout differences between the 0.x and 2.0 host formatters for Vue, HTML, and Markdown.
- Whether the Homebrew formula installs on Linux, and the checksum files that cargo-dist publishes.
- Whether 0.x sorted Tailwind classes. 2.0 does not.
