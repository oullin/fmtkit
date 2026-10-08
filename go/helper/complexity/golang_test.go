package complexity_test

import (
	"strings"
	"testing"

	"go.ollin.sh/fmtkit/go/helper/complexity"
)

// shapesSource is the shared fixture: one function per construct the two lanes
// have to agree on. The TypeScript lane's shared-constructs test is its twin,
// and the expected numbers in the table below are repeated there verbatim. A
// construct that only one language has (try/catch, the ternary) is scored on
// its own side.
const shapesSource = `package shapes

func ifChain(a, b, c int) int {
	if a > 0 {
		return 1
	}

	if b > 0 {
		return 2
	}

	if c > 0 {
		return 3
	}

	return 0
}

func elseIfLadder(a int) string {
	if a == 1 {
		return "one"
	} else if a == 2 {
		return "two"
	} else if a == 3 {
		return "three"
	} else {
		return "other"
	}
}

func switchFour(a string) int {
	switch a {
	case "a":
		return 1
	case "b":
		return 2
	case "c":
		return 3
	case "d":
		return 4
	default:
		return 0
	}
}

func nestedClosure() func(int) int {
	return func(item int) int {
		if item > 0 {
			return item
		}

		return 0
	}
}

func logicalRun(a, b, c, d bool) bool {
	return a && b && c && d
}

func mixedLogical(a, b, c bool) bool {
	return (a && b) || c
}

func loopWithIf(items []int) int {
	total := 0

	for _, item := range items {
		if item > 0 {
			total += item
		}
	}

	return total
}

func (s *shape) Read() int {
	return 1
}

type shape struct{}
`

// scoreShapes scores the shared fixture under a repository-relative path.
func scoreShapes(t *testing.T) map[string]complexity.Function {
	t.Helper()

	functions, err := complexity.Score("pkg/shapes.go", []byte(shapesSource))

	if err != nil {
		t.Fatalf("score: %v", err)
	}

	byKey := map[string]complexity.Function{}

	for _, fn := range functions {
		byKey[fn.Key] = fn
	}

	return byKey
}

func TestScoreScoresTheSharedShapes(t *testing.T) {
	scored := scoreShapes(t)

	cases := []struct {
		name       string
		cyclomatic int
		cognitive  int
	}{
		{"ifChain", 4, 3},
		{"elseIfLadder", 4, 4},
		{"switchFour", 5, 1},
		{"nestedClosure", 2, 2},
		{"logicalRun", 4, 1},
		{"mixedLogical", 3, 2},
		{"loopWithIf", 3, 3},
	}

	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			fn, ok := scored["pkg/shapes.go#"+tc.name]

			if !ok {
				t.Fatalf("function %q was not scored", tc.name)
			}

			if fn.Name != tc.name {
				t.Errorf("name = %q, want %q", fn.Name, tc.name)
			}

			if fn.Cyclomatic != tc.cyclomatic {
				t.Errorf("cyclomatic = %d, want %d", fn.Cyclomatic, tc.cyclomatic)
			}

			if fn.Cognitive != tc.cognitive {
				t.Errorf("cognitive = %d, want %d", fn.Cognitive, tc.cognitive)
			}
		})
	}
}

func TestScoreQualifiesMethodsWithTheirReceiver(t *testing.T) {
	scored := scoreShapes(t)

	fn, ok := scored["pkg/shapes.go#(*shape).Read"]

	if !ok {
		t.Fatalf("receiver-qualified key missing from %v", keysOf(scored))
	}

	if fn.Name != "(*shape).Read" {
		t.Fatalf("name = %q", fn.Name)
	}
}

func TestScoreReportsTheLineOfEachFunction(t *testing.T) {
	scored := scoreShapes(t)

	if fn := scored["pkg/shapes.go#ifChain"]; fn.Line != 3 {
		t.Errorf("line = %d, want 3", fn.Line)
	}
}

func TestScoreKeepsDeclarationOrder(t *testing.T) {
	functions, err := complexity.Score("pkg/shapes.go", []byte(shapesSource))

	if err != nil {
		t.Fatalf("score: %v", err)
	}

	if len(functions) != 8 || functions[0].Name != "ifChain" || functions[7].Name != "(*shape).Read" {
		t.Fatalf("functions = %#v", functions)
	}
}

func TestScoreSkipsBodilessDeclarations(t *testing.T) {
	functions, err := complexity.Score("asm.go", []byte("package asm\n\nfunc add(a, b int) int\n"))

	if err != nil {
		t.Fatalf("score: %v", err)
	}

	if len(functions) != 0 {
		t.Fatalf("functions = %#v", functions)
	}
}

func TestScoreReportsUnparsableFiles(t *testing.T) {
	_, err := complexity.Score("broken.go", []byte("package shapes\n\nfunc ("))

	if err == nil || !strings.Contains(err.Error(), "broken.go") {
		t.Fatalf("err = %v", err)
	}
}

func keysOf(scored map[string]complexity.Function) []string {
	keys := make([]string, 0, len(scored))

	for key := range scored {
		keys = append(keys, key)
	}

	return keys
}
