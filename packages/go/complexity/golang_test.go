package complexity_test

import (
	"os"
	"path/filepath"
	"testing"

	"go.ollin.sh/fmtkit/complexity"
	formatterconfig "go.ollin.sh/fmtkit/formatter/config"
)

// shapesSource is the shared fixture: one function per construct the two lanes
// have to agree on. Its TypeScript twin is
// packages/ts/sidecar/src/complexity/shared-constructs.test.ts, and the expected
// numbers in the table below are repeated there verbatim. A construct that
// only one language has (try/catch, the ternary) is scored on its own side.
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

// scanShapes writes the shared fixture into a scratch tree and scores it.
func scanShapes(t *testing.T) map[string]complexity.Function {
	t.Helper()

	dir := t.TempDir()

	if err := os.WriteFile(filepath.Join(dir, "shapes.go"), []byte(shapesSource), 0o600); err != nil {
		t.Fatalf("write fixture: %v", err)
	}

	scan := complexity.ScanGo(dir, []string{dir}, formatterconfig.Default())

	if len(scan.Errors) != 0 {
		t.Fatalf("scan errors: %#v", scan.Errors)
	}

	byName := map[string]complexity.Function{}

	for _, fn := range scan.Functions {
		byName[fn.Key] = fn
	}

	return byName
}

func TestScanGoScoresTheSharedShapes(t *testing.T) {
	scored := scanShapes(t)

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
			fn, ok := scored["shapes.go#"+tc.name]

			if !ok {
				t.Fatalf("function %q was not scored", tc.name)
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

func TestScanGoQualifiesMethodsWithTheirReceiver(t *testing.T) {
	scored := scanShapes(t)

	if _, ok := scored["shapes.go#(*shape).Read"]; !ok {
		t.Fatalf("receiver-qualified key missing from %v", keysOf(scored))
	}
}

func TestScanGoReportsTheFileAndLineOfEachFunction(t *testing.T) {
	scored := scanShapes(t)

	fn := scored["shapes.go#ifChain"]

	if fn.File != "shapes.go" {
		t.Errorf("file = %q, want %q", fn.File, "shapes.go")
	}

	if fn.Line != 3 {
		t.Errorf("line = %d, want 3", fn.Line)
	}
}

func TestScanGoSkipsTestFiles(t *testing.T) {
	dir := t.TempDir()

	if err := os.WriteFile(filepath.Join(dir, "shapes_test.go"), []byte(shapesSource), 0o600); err != nil {
		t.Fatalf("write fixture: %v", err)
	}

	scan := complexity.ScanGo(dir, []string{dir}, formatterconfig.Default())

	if len(scan.Functions) != 0 || len(scan.Files) != 0 {
		t.Fatalf("test file was scored: %#v", scan)
	}
}

func TestScanGoReportsUnparsableFiles(t *testing.T) {
	dir := t.TempDir()

	if err := os.WriteFile(filepath.Join(dir, "broken.go"), []byte("package shapes\n\nfunc ("), 0o600); err != nil {
		t.Fatalf("write fixture: %v", err)
	}

	scan := complexity.ScanGo(dir, []string{dir}, formatterconfig.Default())

	if len(scan.Errors) != 1 || scan.Errors[0].File != "broken.go" {
		t.Fatalf("errors = %#v", scan.Errors)
	}
}

func keysOf(scored map[string]complexity.Function) []string {
	keys := make([]string, 0, len(scored))

	for key := range scored {
		keys = append(keys, key)
	}

	return keys
}
