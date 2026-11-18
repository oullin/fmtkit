package lint_test

import (
	"reflect"
	"strings"
	"testing"

	"go.ollin.sh/fmtkit/go/helper/lint"
)

// at is a finding's rule and position, which is what most tests compare.
type at struct {
	rule   string
	line   int
	column int
}

var defaults = lint.Limits{MaxDepth: 4, MaxNestedFuncs: 3, MaxLineLength: 120, TabWidth: 4}

func check(t *testing.T, src string, limits lint.Limits) []lint.Finding {
	t.Helper()

	findings, err := lint.Check("a.go", []byte(src), limits)

	if err != nil {
		t.Fatalf("check: %v", err)
	}

	return findings
}

func positions(findings []lint.Finding) []at {
	var out []at

	for _, f := range findings {
		out = append(out, at{f.Rule, f.Line, f.Column})
	}

	return out
}

func expect(t *testing.T, src string, limits lint.Limits, want ...at) {
	t.Helper()

	if got := positions(check(t, src, limits)); !reflect.DeepEqual(got, want) {
		t.Fatalf("findings = %v, want %v", got, want)
	}
}

func only(rule string) lint.Limits {
	switch rule {
	case lint.RuleMaxDepth:
		return lint.Limits{MaxDepth: defaults.MaxDepth}
	case lint.RuleMaxNestedFuncs:
		return lint.Limits{MaxNestedFuncs: defaults.MaxNestedFuncs}
	default:
		return lint.Limits{MaxLineLength: defaults.MaxLineLength, TabWidth: defaults.TabWidth}
	}
}

func TestMaxDepthReportsTheFifthLevelAndDeeper(t *testing.T) {
	src := `package a

func f(xs []int, c chan int) {
	for _, x := range xs {
		if x > 0 {
			switch x {
			case 1:
				select {
				case <-c:
					for {
						if x > 2 {
							return
						}
					}
				}
			}
		}
	}
}
`

	expect(t, src, only(lint.RuleMaxDepth), at{lint.RuleMaxDepth, 10, 6}, at{lint.RuleMaxDepth, 11, 7})

	findings := check(t, src, only(lint.RuleMaxDepth))

	if want := "Blocks are nested too deeply (5). Maximum allowed is 4."; findings[0].Message != want {
		t.Fatalf("message = %q, want %q", findings[0].Message, want)
	}
}

func TestMaxDepthCountsAnElseIfChainAsOneLevel(t *testing.T) {
	src := `package a

func f(a, b, c, d int) {
	if a > 0 {
		if b > 0 {
			if c > 0 {
				if d == 1 {
				} else if d == 2 {
				} else if d == 3 {
				} else {
					for {
					}
				}
			}
		}
	}
}
`

	expect(t, src, only(lint.RuleMaxDepth), at{lint.RuleMaxDepth, 11, 6})
}

func TestMaxDepthCountsTypeSwitchesAndNotTheirClauses(t *testing.T) {
	src := `package a

func f(v any) {
	switch v.(type) {
	case int:
		switch {
		case true:
			switch {
			default:
				switch {
				case false:
					switch {
					}
				}
			}
		}
	}
}
`

	expect(t, src, only(lint.RuleMaxDepth), at{lint.RuleMaxDepth, 12, 6})
}

func TestMaxDepthStartsAgainInAFunctionLiteral(t *testing.T) {
	src := `package a

var top = func() {
	if true {
	}
}

func f() {
	if true {
		for {
			switch {
			default:
				fn := func() {
					if true {
						for {
							if true {
								if true {
								}
							}
						}
					}
				}
				_ = fn
			}
		}
	}
}
`

	expect(t, src, only(lint.RuleMaxDepth))
}

func TestMaxNestedFuncsCountsCallArgumentsOnly(t *testing.T) {
	src := `package a

func f() {
	run(func() { scope(func() { metrics(func() { route(func() {}) }) }) })
}

func g() {
	run(func() {
		defer func() {
			scope(func() {
				go func() {
					metrics(func() {})
				}()
			})
		}()
	})
}

func run(fn func())     { fn() }
func scope(fn func())   { fn() }
func metrics(fn func()) { fn() }
func route(fn func())   { fn() }
`

	expect(t, src, only(lint.RuleMaxNestedFuncs), at{lint.RuleMaxNestedFuncs, 4, 53})

	findings := check(t, src, only(lint.RuleMaxNestedFuncs))

	if want := "Too many nested function literals (4). Maximum allowed is 3."; findings[0].Message != want {
		t.Fatalf("message = %q, want %q", findings[0].Message, want)
	}
}

func TestMaxNestedFuncsReportsEveryLiteralPastTheLimit(t *testing.T) {
	src := `package a

func f() {
	a(func() {
		b(1, (func() {
			c(func() {
				d(func() {
					e(func() {})
				})
			})
		}))
	})
}
`

	expect(t, src, lint.Limits{MaxNestedFuncs: 2},
		at{lint.RuleMaxNestedFuncs, 6, 6},
		at{lint.RuleMaxNestedFuncs, 7, 7},
		at{lint.RuleMaxNestedFuncs, 8, 8},
	)
}

func TestLineLengthReportsTheFirstColumnPastTheLimit(t *testing.T) {
	long := "\tx := " + strings.Repeat("a", 120)
	src := "package a\n\nfunc f() {\n" + long + "\n\t_ = x\n}\n"

	findings := check(t, src, only(lint.RuleLineLength))

	if got, want := positions(findings), []at{{lint.RuleLineLength, 4, 118}}; !reflect.DeepEqual(got, want) {
		t.Fatalf("findings = %v, want %v", got, want)
	}

	if want := "This line has a length of 129. Maximum allowed is 120."; findings[0].Message != want {
		t.Fatalf("message = %q, want %q", findings[0].Message, want)
	}
}

func TestLineLengthCountsTabsToTheNextTabStop(t *testing.T) {
	// One tab and 117 bytes: 121 columns at width 4, 125 at width 8.
	line := "\tx := " + strings.Repeat("a", 112)
	src := "package a\n\nfunc f() {\n" + line + "\n\t_ = x\n}\n"

	expect(t, src, lint.Limits{MaxLineLength: 121, TabWidth: 4})
	expect(t, src, lint.Limits{MaxLineLength: 121}, nil...)
	expect(t, src, lint.Limits{MaxLineLength: 121, TabWidth: 8}, at{lint.RuleLineLength, 4, 115})

	// A tab after text only advances to the next stop.
	mid := "\tx := 1" + "\t// " + strings.Repeat("c", 110)

	expect(t, "package a\n\nfunc f() {\n"+mid+"\n\t_ = x\n}\n", lint.Limits{MaxLineLength: 125, TabWidth: 4})
}

func TestLineLengthCountsRunesNotBytes(t *testing.T) {
	src := "package a\n\n// " + strings.Repeat("é", 117) + "\nvar x int\n"

	expect(t, src, only(lint.RuleLineLength))
}

func TestLineLengthSkipsWhatCannotBeShortened(t *testing.T) {
	long := strings.Repeat("x", 130)
	url := "https://example.com/" + strings.Repeat("p", 120)
	src := `package a

import (
	verylong "example.com/` + long + `"
)

import other "example.com/` + long + `"

//go:generate go run ./gen -` + long + `

// See ` + url + `
// See ` + url + ` (archived).
/*
` + url + `
*/

type T struct {
	Field string ` + "`json:\"" + long + "\"`" + `
	Other string ` + "`json:\"" + long + "\"`" + ` // trailing note
}

var a = "` + long + `"

var b = []string{
	"` + long + `",
	"short", "` + long + `",
}

var c = fmt("` + long + `", verylong.X, other.Y)

var d = ` + "`" + `
` + long + `
` + "`" + `

func fmt(string, ...any) string { return "" }
`

	expect(t, src, only(lint.RuleLineLength),
		at{lint.RuleLineLength, 12, 121},
		at{lint.RuleLineLength, 19, 118},
		at{lint.RuleLineLength, 29, 121},
	)
}

func TestLineLengthChecksWhatOnlyLooksExempt(t *testing.T) {
	long := strings.Repeat("x", 130)
	src := `package a

import (
	// ` + strings.Repeat("prose ", 22) + `
	"fmt"
)

var a = ` + "`" + `
//go:` + "`" + ` + ` + long + `

// ://` + long + `
// see example.com/` + long + `

var b = ` + "`" + `
` + strings.Repeat(")", 130) + `
` + "`" + `

var _ = fmt.Sprint
`

	expect(t, src, only(lint.RuleLineLength),
		at{lint.RuleLineLength, 4, 118},
		at{lint.RuleLineLength, 9, 121},
		at{lint.RuleLineLength, 11, 121},
		at{lint.RuleLineLength, 12, 121},
	)
}

func TestFindingsUsePhysicalPositionsUnderLineDirectives(t *testing.T) {
	src := "package a\n\n//line template.go:500\nfunc f(x int) {\n\tif x > 0 {\n\t\tif x > 1 {\n\t\t}\n\t}\n}\n"

	expect(t, src, lint.Limits{MaxDepth: 1}, at{lint.RuleMaxDepth, 6, 3})
}

func TestLineLengthHandlesCarriageReturns(t *testing.T) {
	line := strings.Repeat("a", 117)
	raw := "var y = `" + strings.Repeat("b", 100) + "\r\n" + strings.Repeat("b", 130) + "`\r\n"
	src := "package a\r\n\r\nvar x = " + line + "\r\n" + raw

	expect(t, src, only(lint.RuleLineLength), at{lint.RuleLineLength, 3, 121})
}

func TestCheckSortsFindingsAndSkipsWhenEveryLimitIsOff(t *testing.T) {
	src := "package a\n\nfunc f() {\n\tif true { if true { if true { if true { if true { run(func() { run(func() { run(func() { run(func() {}) }) }) }) } } } } }\n}\n\nfunc run(fn func()) { fn() }\n"

	got := positions(check(t, src, defaults))
	want := []at{{lint.RuleMaxDepth, 4, 42}, {lint.RuleMaxNestedFuncs, 4, 95}, {lint.RuleLineLength, 4, 118}}

	if !reflect.DeepEqual(got, want) {
		t.Fatalf("findings = %v, want %v", got, want)
	}

	findings, err := lint.Check("a.go", []byte("not go"), lint.Limits{})

	if err != nil || findings != nil {
		t.Fatalf("all off = %v, %v; want nothing, no parse", findings, err)
	}

	if _, err := lint.Check("a.go", []byte("not go"), defaults); err == nil {
		t.Fatal("want a parse error")
	}
}
