package complexity_test

import (
	"encoding/json"
	"os"
	"path/filepath"
	"slices"
	"testing"

	"go.ollin.sh/fmtkit/go/helper/complexity"
)

// fixtureScore is one entry of a fixtures/complexity/<file>.json table.
type fixtureScore struct {
	Key        string `json:"key"`
	Line       int    `json:"line"`
	Cyclomatic int    `json:"cyclomatic"`
	Cognitive  int    `json:"cognitive"`
}

// TestScoreMatchesTheCrossLanguageFixture scores the shared Go fixture under
// its own file name, as fixtures/complexity/README.md prescribes, and expects
// the JSON table exactly: same entries, same report order (by line).
func TestScoreMatchesTheCrossLanguageFixture(t *testing.T) {
	dir := filepath.Join("..", "..", "..", "fixtures", "complexity")
	src, err := os.ReadFile(filepath.Join(dir, "shapes.go"))

	if err != nil {
		t.Fatalf("read fixture: %v", err)
	}

	raw, err := os.ReadFile(filepath.Join(dir, "shapes.go.json"))

	if err != nil {
		t.Fatalf("read expected scores: %v", err)
	}

	var want []fixtureScore

	if err := json.Unmarshal(raw, &want); err != nil {
		t.Fatalf("decode expected scores: %v", err)
	}

	functions, err := complexity.Score("shapes.go", src)

	if err != nil {
		t.Fatalf("score: %v", err)
	}

	got := make([]fixtureScore, 0, len(functions))

	for _, fn := range functions {
		got = append(got, fixtureScore{Key: fn.Key, Line: fn.Line, Cyclomatic: fn.Cyclomatic, Cognitive: fn.Cognitive})
	}

	if !slices.Equal(got, want) {
		t.Fatalf("scores differ from shapes.go.json\n got: %+v\nwant: %+v", got, want)
	}
}
