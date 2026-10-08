# Complexity fixtures

These fixtures are shared by both language lanes. Each source file has a
`<file>.json` beside it: the scores it must produce, in report order (by line,
then by name).

```json
{ "key": "<file>#<name>", "line": 1, "cyclomatic": 4, "cognitive": 3 }
```

The key prefix is the fixture's file name, so the fixture is scored with that
name as its repository-relative path.

| File | Scored by | Test |
|---|---|---|
| `shapes.ts` | `crates/ts` (`complexity::score_program`) | `the_fixtures_match_their_expected_scores` in `crates/ts/src/complexity/tests.rs` |
| `shapes.go` | the Go helper (gocyclo / gocognit) | the helper's complexity tests |
| `same-name.ts` | `crates/ts` | same Rust test; it pins the 2.0 key fix described below |

## Where the numbers come from

- **`shapes.ts`** is v1's `SHARED_CONSTRUCTS` from
  `packages/ts/sidecar/src/complexity/shared-constructs.test.ts`. The v1
  template literal began with a newline, which is dropped here, so every line
  is one less than in that test: the test asserts `ifChain` on line 2, and the
  fixture puts it on line 1.
- **`shapes.go`** is v1's `shapesSource` from
  `packages/go/complexity/golang_test.go`, byte for byte.
- **Scores for the seven shared functions** (`ifChain`, `elseIfLadder`,
  `switchFour`, `nestedClosure`, `logicalRun`, `mixedLogical`, `loopWithIf`)
  are the `SHARED_SCORES` table that both v1 tests assert verbatim.
- **Not asserted by any v1 test:**
  - `(*shape).Read` in `shapes.go`. v1 only checked that its key exists. Its
    1 / 0 was traced by hand (gocyclo starts at 1 and the body has no branch;
    gocognit has nothing to count).
  - Every line other than `ifChain`'s. These were read off the sources.
  - Both of the above were then confirmed by running v1 itself on these
    files: `complexity.ScanGo` for Go, and the sidecar's `ComplexityScanner`
    on oxc-parser 0.152.0 for TypeScript.

## The 2.0 key fix

v1 gives two named functions that share a name, on different lines, separate
keys: the first is `name` and the second is `name:<line>`
(`ComplexityScanner.#keyFor`). An anonymous function reports under the key of
the named function around it. But v1 recorded that owner by its **name**, not
its **key**: `FunctionSiteCollector.#visitFunction` passes the site's `name` on
as `owner`. `#keyFor` returns an anonymous site's name unchanged. So every
callback inside the second function folded its cyclomatic number into the
*first* function's key.

Cognitive scores were unaffected, because each named function's cognitive score
already includes everything nested in it.

In 2.0 an anonymous function reports under the key of the function around it.

`same-name.ts`:

```ts
const first = {
	handler(): void {},
};

const second = {
	handler(rows: number[]): number[] {
		return rows.filter((row) => row > 0 && row < 10);
	},
};
```

| Key | Line | v1 (cyclomatic / cognitive) | 2.0 (cyclomatic / cognitive) |
|---|---|---|---|
| `same-name.ts#handler` | 2 | **2** / 0 | 1 / 0 |
| `same-name.ts#handler:6` | 6 | **1** / 1 | 2 / 1 |

In v1, the arrow callback's 2 (one, plus `&&`) raised the empty first
`handler` and left the second `handler` at 1.

The same fix applies when the second function is nested inside the first
(`function run() { function run() { … } }`). In v1 the inner function's
callbacks raised the outer `run`; in 2.0 they raise `run:2`.

## Parity check

Beyond these fixtures, the Rust scorer was compared with v1 on 6,660 script
files: the oxc repository at the pinned tag, zod, and this repository's v1
TypeScript. The comparison covered 24,314 functions. The two agreed on every
name, line, and score in 5,760 files. They also agreed on which 895 files fail
to parse. The 5 files that differed were all the key fix above.
