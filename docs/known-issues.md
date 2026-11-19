# Known upstream issues

The fuzz targets in `fuzz/` found the defects below in the formatters that fmtkit builds on. Each one lives in a dependency rather than in fmtkit, so fmtkit contains or refuses its effect instead of fixing it. The versions are:

- `oxc_formatter`, `oxc_formatter_css`, and `oxc_formatter_markdown` from the oxc tag `oxlint_v1.94.0` (the release that shipped oxfmt 0.79.0);
- `oxc-css-parser` 0.0.22, the parser behind `oxc_formatter_css`;
- `markup_fmt` 0.28.0.

This document writes every input as a Rust string literal. In a saved fuzz input, the first byte selects the dialect or host (see `fuzz/fuzz_targets/`); the literal is the rest of the file. Unless a section says otherwise, the outputs use fmtkit's default `[ts.format]` options (tabs, single quotes, a print width of 200).

Re-run the regression tests after every oxc or markup_fmt upgrade. `upstream_oxfmt_bugs_stay_refused` in `crates/ts/src/tests.rs` and `fuzz_findings_that_never_settle_are_refused` in `crates/hosts/src/tests.rs` fail when an upstream fix lands, as does `a_hyphen_before_a_space_in_a_style_fence_panics_upstream` in `crates/hosts/src/tests.rs`; remove the entry from this document when that happens.

## How fmtkit contains these issues

- **A formatter prints a different program.** After every step, the TypeScript pipeline re-parses the output and compares its [fingerprint](../crates/ts/src/fingerprint.rs) with the input's. A mismatch returns `TsError::Invariant` and leaves the file as written. The `ts_pipeline` fuzz target accepts an invariant error whose step is `oxfmt` for this reason; an invariant error from any other step is a bug in fmtkit and still fails the target.
- **A formatter does not reach a fixed point in one run.** `fmtkit_ts::format_source` and `fmtkit_hosts::format_host` format a changed output again, at most three more times, until a round changes nothing. Output that still changes after that returns an error whose step is `idempotency` and leaves the file as written. A file that the first round does not change costs exactly one round.
- **A formatter or parser panics.** The engine catches the panic for that file alone. The file fails with an `internal error, please report it` message and is left as written, and the run continues with every other file.

## oxc_formatter

### Two line comments merge into one

- Input (`.ts`): `"a//\n<a//"`
- Observed: `"a < a; // //\n"`. The formatter moves the first comment to the end of the statement and appends the second comment to it as text. The same happens when the continuation operator is `.`, `^`, `*`, `-`, `/`, or `?` instead of `<`.
- Expected: output that keeps two separate line comments.
- fmtkit: the fingerprint counts comments, so the invariant check refuses the output.

### Parentheses turn `await` into a call in a file of unambiguous module kind

- Input (`.tsx`): `"await 1e3.a"`. The same happens with `"await 1.5.a"` and `"await 1..a"`.
- Observed: `"await (1e3).a;\n"`, and `"await(1e3).a;\n"` on a second run. oxc reads `.tsx` files with the unambiguous module kind, in which `await (1e3).a` is a call of a function named `await`, not an await expression. In a module (`.mts`) both texts parse to the same program, which a fingerprint test asserts.
- Expected: output that keeps the await expression, for example `"await 1e3.a;\n"`.
- fmtkit: the invariant check refuses the output.

### A carriage return in a template without a cooked value fails a debug assertion

- Input (`.ts`): ``"a`\r\\1`"`` (a lone carriage return, then the invalid escape `\1` in a tagged template). The same happens with ``"a`\r\n\\1`"``.
- Observed: debug builds panic in `oxc_formatter_core/src/write/builders.rs:282` with "The content ... contains an unsupported `\r` line terminator character". The formatter prints the raw text of a template without a cooked value through a text builder that rejects `\r`. Release builds compile the assertion out.
- Expected: ``"a`\n\\1`;\n"``. The formatter already writes `\n` for a carriage return in every template that has a cooked value.
- fmtkit: before the formatter runs, `template_line_breaks` in `crates/ts/src/format.rs` rewrites `\r\n` and a lone `\r` inside template elements to `\n`. ECMAScript reads both as `\n` in the cooked and in the raw value of a template, so the rewrite preserves the program, and the fingerprint hashes template raw text the same way. Text outside template elements does not change.

### A line comment inside a comparison needs two runs

- Input (`.ts`): `"a<a//\n<a>"`
- Observed: the first run prints `"a <\n\ta<a>; //\n"`, and the second run prints `"a < a<a>; //\n"`.
- Expected: the second output from the first run.
- fmtkit: the fixed-point loop absorbs the extra run.

### A line comment after `await` duplicates the next line

- Input (`.ts`): `"await//\nawait b"`, which parses as the single statement `await (await b)`.
- Observed: `"await //\nawait b;\nawait b;\n"`, which holds two statements. Every further run adds one more `await b;` line.
- Expected: output that holds one statement, for example `"await //\nawait b;\n"`.
- fmtkit: the fingerprint counts statements, so the invariant check refuses the output.

## oxc_formatter_css

### An unclosed parenthesis in a declaration value gains a semicolon on every run

- Input (CSS): `"{r:(}"`. The input `"{r:({}}"` behaves the same way. fmtkit reaches both through a Markdown fence such as `"```css\n{r:(}"`.
- Observed: the first run prints `"{\n\tr: (;\n}\n"`, the second run prints `"{\n\tr: (;;\n}\n"`, and every further run adds one more `;`.
- Expected: a syntax error, or output that a second run leaves unchanged.
- fmtkit: `format_host` returns the `idempotency` error and leaves the document as written.

## oxc-css-parser

### A hyphen before a character that cannot start an identifier panics

- Input (CSS): `".- "`. fmtkit reaches it through the Markdown fence `"```css\n.- o"` and through the HTML element `"<style>.- o</style>"`.
- Observed: the tokenizer panics in `scan_ident_sequence` at `src/tokenizer/mod.rs` with "internal error: entered unreachable code". The function expects an identifier character, an escape, or the end of the input after a leading hyphen, and treats every other character as unreachable.
- Expected: a syntax error, which fmtkit already handles by keeping the style as written.
- fmtkit: the engine catches a panic per file, so the document fails with "internal error, please report it: internal error: entered unreachable code", is left as written, and every other file is still processed. The release profile unwinds rather than aborts for this reason. The panic message itself still reaches standard error. The `hosts` fuzz target calls the formatter directly and reports the panic as a crash.

## oxc_formatter_markdown

### A lazy line after a list item paragraph drifts right on every run

- Input: `"- a\n\n  {{\n}}"`
- Observed: each run indents the lazy line `}}` by two more spaces: `"- a\n\n    {{\n  }}\n"`, then `"- a\n\n    {{\n    }}\n"`, then `"- a\n\n    {{\n      }}\n"`, without end.
- Expected: output that a second run leaves unchanged.
- fmtkit: `format_host` returns the `idempotency` error and leaves the document as written.

### A math block splits on the second run

- Input: `"$$$\n$$"`
- Observed: the first run prints `"$$\n$$\n$$\n"`, and the second run prints `"$$\n$$\n\n$$\n$$\n"`.
- Expected: one stable output that keeps the structure of the input.
- fmtkit: the fixed-point loop settles on the second output.

### A form feed survives one run

- Input: `"a\u{c}\r\u{c}"`. The input `"a\n\n\u{c}"` behaves the same way.
- Observed: the first run prints `"a\u{c}\n"` (and `"a\n\n"` for the second input); the second run prints `"a\n"`.
- Expected: `"a\n"` from the first run.
- fmtkit: the fixed-point loop settles on the second output.

### An indented lazy line turns a code span into a table

- Input: ``"- `a\r|-\n`"``. fmtkit reads the lone carriage return as a line break, as CommonMark does.
- Observed: the first run indents the lazy lines into the list item (``"- `a\n  |-\n  `\n"``). The second run reads `|-` as a table delimiter row and prints a table: ``"- | `a  |\n  | --- |\n  | `   |\n"``.
- Expected: output that keeps the document structure and that a second run leaves unchanged.
- fmtkit: the fixed-point loop settles on the table. The document changes meaning.

### A lazy continuation of a nested block quote is rewritten

- Input: `"- a\n  >a\na"`. The last line is a lazy continuation of the block quote paragraph, whose text is `a a`.
- Observed: the first run prints `"- a\n    > a\n\t\t> a\n"`, and the second run joins the lines into `"- a\n    > a > a\n"`, whose paragraph text is `a > a`.
- Expected: output that keeps the paragraph text and that a second run leaves unchanged.
- fmtkit: the fixed-point loop settles on the second output. The document changes meaning.

### A setext underline inside a block quote gains an empty line

- Input: ``">`a\n--\n`"``
- Observed: the first run prints ``"> `a\n> --\n> `\n"``, and the second run inserts an empty quoted line: ``"> `a\n> --\n>\n> `\n"``.
- Expected: one stable output.
- fmtkit: the fixed-point loop settles on the second output.

## markup_fmt

### An unformatted multi-line Vue interpolation drifts right on every run

- Input (`.vue`): ``"{{`\n`\n\0}"``. The input ``"{{{;}\n`\n`}"`` behaves the same way. (``"{{a\n`\n}}"`` drifted the same way under markup_fmt 0.27; 0.28 reports it as a syntax error, and fmtkit keeps it as written.)
- Observed: the expression does not parse, so fmtkit hands it back to markup_fmt as written. Each run then indents one line of the template literal by one more tab. `reflow_with_indent` in `src/printer.rs` removes the indentation that `detect_indent` in `src/helpers.rs` measures, but `detect_indent` also measures the lines inside a template literal, which `reflow_with_indent` keeps as written. A template line at column 0 makes the measured indentation 0, so no run removes the indentation that the previous run added.
- Expected: output that a second run leaves unchanged.
- fmtkit: `format_host` returns the `idempotency` error and leaves the document as written. The TypeScript pipeline prints an interpolation that parses, and that output settles.

### A whitespace-only line keeps its whitespace for one run

- Input (`.html`): `"<style>a\n \na</style>"`. The Vue input `"{{a\n \n=}"` behaves the same way.
- Observed: the first run keeps the line that holds one space (`"<style>\n\ta\n \n\ta\n</style>\n"`), and the second run empties it (`"<style>\n\ta\n\n\ta\n</style>\n"`). `reflow_with_indent` prints a whitespace-only line as written when the measured indentation is 0.
- Expected: the second output from the first run.
- fmtkit: the fixed-point loop absorbs the extra run.

### A template literal inside an unformatted style drifts right on every run

- Input (`.html`): ``"<style>s\n`\nd</style>"``
- Observed: the style does not parse as CSS, so fmtkit hands it back to markup_fmt as written. Each run indents the backtick line by one more tab: ``"<style>\n\ts\n\t`\nd\n</style>\n"``, then ``"<style>\n\ts\n\t\t`\nd\n</style>\n"``, and so on. `reflow_with_indent` tracks template literals in style content as it does in script content, which is the mechanism of the Vue interpolation drift above.
- Expected: output that a second run leaves unchanged.
- fmtkit: `format_host` returns the `idempotency` error and leaves the document as written.

## fmtkit limitations

These are gaps in fmtkit itself rather than in a dependency.

### `import/export` does not follow `export *`

- Input: a module that re-exports two modules with `export * from './a'` and `export * from './b'`, where both declare the same export name.
- Observed: no finding. oxlint 1.94.0 reports `import/export` here, because its CLI builds a module graph and resolves every `export *`. fmtkit lints each file on its own, without a module graph or a resolver, so the rule sees only the names that the file itself declares.
- Expected: the duplicate export is reported.
- fmtkit: the rule still reports duplicates within a single file. Cross-module duplicates surface at type-check time (`tsc` reports TS2308).

### Type-aware rules are not cached and have no fixes

- Input: a type-aware rule (such as `typescript/no-floating-promises`) turned on under `[lint.rules]` or with `[lint] type_aware = "nkzw"`.
- Observed: tsrs runs on every run that has a script file in scope, including runs that `fmtkit serve` answers; `format` does not apply tsrs's fixes; `--stdin-filepath` and `.vue` files are not checked by these rules.
- Expected: `format` applies the safe fixes of `return-await` and `no-duplicate-type-constituents`, as `oxlint --type-aware --fix` does.
- fmtkit: see [Type-aware rules](type-aware.md#limitations) for why each of these holds.
