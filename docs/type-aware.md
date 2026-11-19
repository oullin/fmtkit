# Type-aware rules

Type-aware rules are oxc's tsgolint rules, such as `typescript/no-floating-promises` and `typescript/await-thenable`. They need the TypeScript type checker. A rule of this kind decides about a call in one file from a type declared in another file, so fmtkit's single-file lint cannot run it. fmtkit runs these rules through [tsrs](https://www.npmjs.com/package/tsrs), a native TypeScript type checker that speaks the protocol oxlint uses for its own type-aware backend. No Node.js runs: fmtkit starts the native binary directly.

## Turning them on

These rules are off unless you name them. The bundled policy enables none of them, so a repository without type-aware settings never needs tsrs.

```toml
[lint]
type_aware = "nkzw"        # the ten rules @nkzw/oxlint-config 3.0 enables, with its options
# tsrs = "node_modules/@tsrs/darwin-arm64/tsrs"   # only if fmtkit cannot find it

[lint.rules]
"typescript/no-floating-promises" = "warn"   # or name rules one at a time
"typescript/no-misused-promises" = ["error", { checksVoidReturn = false }]
```

- **Naming a rule.** A type-aware rule named under `[lint.rules]` (with any of its spellings: `typescript/`, `@typescript-eslint/`, or the bare name) turns it on for every file. Such a rule takes at most one option.
- **The `nkzw` preset.** `type_aware = "nkzw"` turns on the ten type-aware rules that `@nkzw/oxlint-config` 3.0 enables, with its options, as listed in [`type-aware-nkzw.json`](../crates/lint/policy/type-aware-nkzw.json). As upstream does, it turns `no-floating-promises` on for TypeScript files only. A `[lint.rules]` entry for one of these rules replaces the preset's setting and applies to every file. The preset exists because those ten rules are exactly what the bundled policy leaves out of nkzw's configuration: one line restores the whole upstream configuration, and copying ten entries and their options by hand would drift from upstream.
- **Categories.** A category such as `correctness` also covers some type-aware rules (for example `typescript/no-base-to-string`), but a category never turns one on. oxlint without `--type-aware` behaves the same way. Only a rule you name, or one the preset names, runs.

`[[exempt]]` entries, `[lint] ignore`, and `oxlint-disable` and `eslint-disable` directives apply to type-aware findings exactly as they apply to other findings. Findings are reported as `typescript/<rule>` at the configured severity.

## Installing tsrs

```sh
npm i -D tsrs
```

npm installs the native binary for your platform at `node_modules/@tsrs/<platform>/tsrs`. fmtkit looks for tsrs in this order:

1. `FMTKIT_TSRS`;
2. `[lint] tsrs`, relative to the repository root;
3. beside the fmtkit executable (as invoked, then with symlinks resolved);
4. `node_modules/@tsrs/<platform>/tsrs` in the repository root or any parent directory;
5. `PATH`, skipping interpreter scripts such as npm's `node_modules/.bin/tsrs`, which is a Node.js launcher.

If a type-aware rule is on and tsrs cannot be found, the run stops before it writes anything and exits with code 2. The message says how to install tsrs. A path named by `FMTKIT_TSRS` or `[lint] tsrs` that is not an executable file is also an error, rather than a reason to keep looking. If tsrs itself fails, the run exits with code 3 and shows the end of tsrs's error output.

## How a run works

Each run makes one `tsrs --lint headless` call. It sends every TypeScript and JavaScript file in scope that `[lint] ignore` does not skip, together with each file's rules and their options. tsrs builds the programs from the repository's `tsconfig.json` files. It reports findings only for the files fmtkit sent, so the default scope (the changed files) checks the changed files against the whole program. In `check` mode, tsrs runs beside the formatting and lint lanes. In `format` mode, it runs after the files are written, so its positions match the files on disk.

## Limitations

- **No cache.** A type-aware finding in one file depends on every file its program reads: other sources, `.d.ts` files, generated and ignored files, the tsconfig chain, and `node_modules`. A cache key that covered all of that would have to read and hash those files on every run. tsrs therefore runs on every run that has a script file in scope and a type-aware rule on, including runs that `fmtkit serve` answers. The per-file cache still skips formatting and in-process linting for unchanged files. Because nothing is cached, edits to another file, to a tsconfig, or to `node_modules` are always seen, with or without `serve --watch`.
- **No fixes.** `format` does not apply tsrs's fixes. fmtkit applies fixes in memory, one file at a time, and lints each file again after a fix. A type-aware fix would need a new tsrs run after each round. Of nkzw's ten rules, only `return-await` and `no-duplicate-type-constituents` offer safe fixes.
- **No stdin.** `--stdin-filepath` formats stdin and does not lint it, so type-aware rules do not run there either.
- **No Vue.** The protocol has no Vue support, so `<script>` blocks in `.vue` files are not sent.
- **Unused directives.** A directive that names a type-aware rule is judged after tsrs runs. A name for a type-aware rule that is on for that file is unused when it suppressed no tsrs finding, and `unused-directive` reports it without a fix. A name for a type-aware rule that is off is never reported, because another linter may read it. A directive with no rule names (such as `// eslint-disable-next-line`) is judged by the in-process rules only. When a directive names both kinds of rule and none of its ordinary names suppressed anything, it is reported as a whole comment, without a fix.
- **tsrs is in beta.** fmtkit is tested against tsrs `7.1.0-beta.tsrs3`. Its protocol is the one oxlint uses for its own type-aware backend (version 2). A tsrs release that changes that protocol incompatibly fails the run with exit code 3 rather than reporting nothing.
