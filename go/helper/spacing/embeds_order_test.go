package spacing

import (
	"slices"
	"testing"
)

// TestEmbedViolationsAreSortedByLine guards the deliberate 2.0 fix: the
// detached go:embed matches come from a map, so the violations must be put in
// line order before they are reported.
func TestEmbedViolationsAreSortedByLine(t *testing.T) {
	src := "package sample\n\nimport \"embed\"\n\n" +
		"//go:embed a.txt\n\nvar aFS embed.FS\n\n" +
		"//go:embed b.txt\n\nvar bFS embed.FS\n\n" +
		"//go:embed c.txt\n\nvar cFS embed.FS\n\n" +
		"//go:embed d.txt\n\nvar dFS embed.FS\n"

	for range 20 {
		ctx, err := newFileContext("sample.go", []byte(src))

		if err != nil {
			t.Fatalf("parse: %v", err)
		}

		violations := newEmbedDirectiveRepairer(ctx).analyze("sample.go")
		lines := make([]int, 0, len(violations))

		for _, violation := range violations {
			lines = append(lines, violation.Line)
		}

		if !slices.Equal(lines, []int{7, 11, 15, 19}) {
			t.Fatalf("lines = %v, want [7 11 15 19]", lines)
		}
	}
}
