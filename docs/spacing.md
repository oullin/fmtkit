# Spacing rule reference

This is the canonical reference for the built-in `spacing` rule that `go-fmt` runs before `gofmt` and `goimports`. The rule is AST-based: it enforces blank-line boundaries and declaration ordering so the standard formatters have a consistent shape to work with.

The README summarises the rule; this page lists every variant with before/after examples.

## Where it applies

The spacing rule inspects statement lists inside:

- Function bodies (`BlockStmt`)
- `case` and `default` clauses (`CaseClause`)
- `select` communication clauses (`CommClause`)

It also inserts a blank line after anonymous-function assignments such as `name := func(...) { ... }`, `name = func(...) { ... }`, and `var name = func(...) { ... }` when another statement follows immediately.

## Blank line before control flow and jump-style statements

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

## Blank line after block statements

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

## Blank lines around standalone `var` declarations

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

## Blank lines around stdlib sorting calls

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

## Blank lines around stdlib random calls

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

## Blank line after `t.Helper()`

Standalone `t.Helper()` calls are followed by a blank line when another statement follows immediately.

```go
// before
func helper(t *testing.T) {
    t.Helper()
    value := 1
}

// after
func helper(t *testing.T) {
    t.Helper()

    value := 1
}
```

## Blank lines before top-level `routes.*` calls

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

## Blank lines around type declarations

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

## Type declarations at the top of the file

All `type` definitions are moved above non-type declarations, after the import block.
