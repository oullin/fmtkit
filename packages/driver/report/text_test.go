package report

import (
	"bytes"
	"strings"
	"testing"

	"github.com/fatih/color"
	formatterengine "github.com/oullin/fmtkit/packages/formatter/engine"
	"github.com/oullin/fmtkit/packages/vet"
)

// renderTextPlain renders without ANSI escapes so substring asserts are
// stable. color.NoColor is global state, so these tests must not run in
// parallel.
func renderTextPlain(t *testing.T, cwd, mode string, report Combined) string {
	t.Helper()

	previous := color.NoColor
	color.NoColor = true

	t.Cleanup(func() { color.NoColor = previous })

	var out bytes.Buffer

	if err := RenderText(&out, cwd, mode, report); err != nil {
		t.Fatalf("render text: %v", err)
	}

	return out.String()
}

func assertContainsAll(t *testing.T, output string, wants []string) {
	t.Helper()

	for _, want := range wants {
		if !strings.Contains(output, want) {
			t.Fatalf("expected output to contain %q, got:\n%s", want, output)
		}
	}
}

func TestRenderTextCheckModeFailure(t *testing.T) {
	output := renderTextPlain(t, "/work", "check", sampleCombinedReport())

	assertContainsAll(t, output, []string{
		"Formatter",
		"Checked 2 file(s).",
		"sample.go",
		"[spacing] line 7: after if statement",
		"✓ would apply spacing, gofmt",
		"! parse error",
		"walk.go",
		"! walk failed",
		"Result: fail. 1 changed, 1 violation(s), 2 error(s).",
		"Vet",
		"module-a",
		"! automatic go vet ./... failed:",
		"Result: fail. 1 error(s).",
	})
}

func TestRenderTextFormatModeVerbs(t *testing.T) {
	output := renderTextPlain(t, "/work", "format", sampleCombinedReport())

	assertContainsAll(t, output, []string{
		"Formatted 2 file(s).",
		"✓ applied spacing, gofmt",
	})
}

func TestRenderTextEmptyReport(t *testing.T) {
	output := renderTextPlain(t, "/work", "check", Combined{})

	assertContainsAll(t, output, []string{
		"No Go files found.",
		"Skipped automatic go vet ./... because no Go module or workspace was detected.",
		"Result: skipped. 0 error(s).",
	})
}

func TestRenderTextVetSkippedWithoutToolchain(t *testing.T) {
	report := Combined{Vet: vet.Report{Skipped: true}}

	output := renderTextPlain(t, "/work", "check", report)

	assertContainsAll(t, output, []string{
		"Skipped automatic go vet ./... because the Go toolchain is not available.",
	})
}

func TestRenderTextAllPass(t *testing.T) {
	report := Combined{
		Formatter: formatterengine.Report{
			Result: "pass",
			Files:  1,
			Results: []formatterengine.FileResult{
				{File: "/work/clean.go"},
			},
		},
		Vet: vet.Report{Root: "/work"},
	}

	output := renderTextPlain(t, "/work", "check", report)

	assertContainsAll(t, output, []string{
		"Checked 1 file(s).",
		"Result: pass. 0 changed, 0 violation(s), 0 error(s).",
		"go vet ./... passed.",
		"Result: pass. 0 error(s).",
	})

	if strings.Contains(output, "clean.go") {
		t.Fatalf("clean files should not be listed, got:\n%s", output)
	}
}
