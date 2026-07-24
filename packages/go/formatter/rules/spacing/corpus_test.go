package spacing

import (
	"flag"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

var updateCorpus = flag.Bool("update", false, "rewrite spacing corpus golden files")

// TestSpacingCorpus runs each testdata/corpus/*.input fixture through
// New().Apply and asserts the rewritten source matches its .golden pair byte
// for byte. The corpus characterizes the current spacing behavior across
// statement gaps, selector-call setup spacing, type-declaration spacing, type
// ordering, embed-directive repair/collapse, and import aliases so later
// refactor stages cannot silently change the output. The fixtures use a
// non-.go extension so the repo's own format-all walk leaves them untouched.
// Regenerate with `go test ./formatter/rules/spacing -run TestSpacingCorpus -update`.
func TestSpacingCorpus(t *testing.T) {
	inputs, err := filepath.Glob(filepath.Join("testdata", "corpus", "*.input"))

	if err != nil {
		t.Fatalf("glob corpus: %v", err)
	}

	if len(inputs) == 0 {
		t.Fatal("no corpus fixtures found")
	}

	for _, input := range inputs {
		input := input
		name := strings.TrimSuffix(filepath.Base(input), ".input")

		t.Run(name, func(t *testing.T) {
			src, err := os.ReadFile(input)

			if err != nil {
				t.Fatalf("read input: %v", err)
			}

			_, formatted, err := New().Apply(input, src)

			if err != nil {
				t.Fatalf("apply: %v", err)
			}

			golden := strings.TrimSuffix(input, ".input") + ".golden"

			if *updateCorpus {
				if err := os.WriteFile(golden, formatted, 0o644); err != nil {
					t.Fatalf("update golden: %v", err)
				}

				return
			}

			want, err := os.ReadFile(golden)

			if err != nil {
				t.Fatalf("read golden: %v", err)
			}

			if string(formatted) != string(want) {
				t.Fatalf("output mismatch for %s\n--- got ---\n%s\n--- want ---\n%s", name, formatted, want)
			}
		})
	}
}
