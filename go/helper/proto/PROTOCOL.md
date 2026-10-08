# fmtkit ↔ fmtkit-go-helper protocol

`fmtkit` starts one `fmtkit-go-helper` per run and talks to it over the
helper's stdin (requests) and stdout (replies). The helper's stderr is free-form
diagnostics; fmtkit shows its tail only when the helper dies.

Protocol version: **1** (`proto.Version` in Go, `proto::PROTOCOL_VERSION` in
`crates/go`). Bump it on any change to this document's layouts.

## Frames

Every message is one frame:

```
u32 length | u8 kind | u32 id | payload
```

- All integers are little-endian.
- `length` counts everything after itself: `5 + len(payload)`. A length below 5
  or above 256 MiB (`268435456`) is malformed.
- `id` pairs a reply with its request. fmtkit numbers requests from 1 and
  wraps, skipping 0; `Hello` and `Shutdown` use id 0.

| kind | name     | direction        | payload          |
|------|----------|------------------|------------------|
| 1    | Hello    | both, first      | `Hello`          |
| 2    | Process  | fmtkit → helper  | `ProcessRequest` |
| 3    | Reply    | helper → fmtkit  | `ProcessReply`   |
| 4    | Shutdown | fmtkit → helper  | empty            |

## Payload encoding

Payloads are a fixed sequence of fields, with no tags and no padding:

- `u32`: 4 bytes, little-endian.
- `bool`: one byte, `0` or `1`; any other value is malformed.
- `bytes`: `u32` length, then that many bytes.
- `string`: same as `bytes`; UTF-8 by convention (fmtkit decodes lossily).
- `list<T>`: `u32` count, then that many `T`.

A payload must be consumed exactly: missing bytes, trailing bytes, and a list
count the remaining bytes cannot hold are all malformed.

### Hello

```
u32    proto     protocol version
string version   fmtkit release ("2.0.0"), or "dev" for an unstamped build
```

### ProcessRequest

```
string rel              repository-relative path, forward slashes
string abs              absolute path (goimports resolution only)
bytes  source           the file's current contents
bool   spacing          run the spacing rule
bool   gofmt            run go/format
bool   goimports        run goimports
bool   resolve_imports  let goimports add and remove imports
bool   complexity       score the final text
```

### ProcessReply

```
string               error        empty when the file was processed
bytes                output       the final text (the input when nothing changed or on error)
list<string>         applied      steps that changed the text, in order
list<Violation>      violations   spacing findings against the input
list<Score>          complexity   one per function declaration with a body
```

```
Violation: string rule | u32 line | u32 column | string message      (1-based; 0 = none)
Score:     string key | string name | u32 line | u32 cyclomatic | u32 cognitive
```

`key` is `<rel>#<name>`; `name` is the function name or the
receiver-qualified method name, `(*T).Method`.

## Session

1. fmtkit writes `Hello{PROTOCOL_VERSION, fmtkit version}`.
2. The helper replies with its own `Hello` and never judges compatibility.
   fmtkit does: the protocol versions must be equal, and the fmtkit versions
   must be equal unless the helper reports `dev` or fmtkit is a debug build.
   Anything else is a version error and fmtkit stops the helper.
3. fmtkit writes `Process` frames at any time, from any thread. The helper
   decodes them on one reader goroutine, processes them on `GOMAXPROCS`
   workers, and writes `Reply` frames from one writer goroutine **in
   completion order**, so replies arrive out of order.
4. fmtkit writes `Shutdown` (or closes stdin). The helper stops reading,
   answers every request it already accepted, flushes, and exits 0.

## Steps and errors

The helper runs the selected steps in this order, each on the previous one's
output: `spacing`, `gofmt`, `goimports`, then `complexity` on the final text.
A step's name is appended to `applied` when it changed the text.

- `goimports` is format-only (`imports.Options{FormatOnly: true, Comments:
  true, TabIndent: true, TabWidth: 8}`, no filename) unless `resolve_imports`
  is set; then it resolves imports as the goimports CLI does, using `abs` (or
  `rel` when `abs` is empty) as the filename. Resolution is the only thing that
  reads the filesystem.
- A failing step stops the pipeline. `error` is `"<step>: <cause>"`, `output`
  is the original source, `applied` and `violations` keep what earlier steps
  reported, and `complexity` is empty.
- A panic while processing a request becomes `error = "internal error: ..."`
  for that request only; the stack goes to stderr.

## Failures

A malformed frame, an unexpected kind, or a closed stdout is fatal to whichever
side sees it. The helper writes the reason to stderr and exits 1; fmtkit fails
every outstanding and later request with a crash or protocol error, and the run
exits 3.

## Golden frames

`testdata/*.bin` hold one encoded frame each. `go test ./proto` and
`cargo test -p fmtkit-go` both encode the same values and compare them byte for
byte, then decode the files back. Regenerate with
`go test ./proto -run TestGoldenFrames -update` and update the Rust values to
match.

| file              | kind     | id         | contents                                         |
|-------------------|----------|------------|--------------------------------------------------|
| `hello.bin`       | Hello    | 0          | proto 1, version `2.0.0`                         |
| `process.bin`     | Process  | 7          | `pkg/a.go`, spacing + gofmt + complexity         |
| `reply.bin`       | Reply    | 7          | two applied steps, two violations, two scores    |
| `reply_error.bin` | Reply    | 4294967295 | a gofmt error and nothing else                   |
| `shutdown.bin`    | Shutdown | 0          | empty                                            |
