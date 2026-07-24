package golang

import (
	"fmt"
	"testing"

	"go.ollin.sh/fmtkit/driver/internal/pipeline"
	"go.ollin.sh/fmtkit/driver/internal/toolchain"
	report "go.ollin.sh/fmtkit/driver/report"
	formatterengine "go.ollin.sh/fmtkit/formatter/engine"
	"go.ollin.sh/fmtkit/vet"
)

func detailStrings(details []pipeline.Detail) []string {
	out := make([]string, 0, len(details))

	for _, d := range details {
		out = append(out, d.Label+"|"+d.Value)
	}

	return out
}

func assertDetails(t *testing.T, got []pipeline.Detail, want ...string) {
	t.Helper()

	if g := fmt.Sprint(detailStrings(got)); g != fmt.Sprint(want) {
		t.Fatalf("details mismatch\n--- got ---\n%s\n--- want ---\n%s", g, fmt.Sprint(want))
	}
}

// outcomeFor builds a Go outcome for the given mode, formatter report, and vet
// report.
func outcomeFor(mode report.Mode, fm formatterengine.Report, vt vet.Report) Outcome {
	return Outcome{Mode: mode, Combined: report.Combined{Formatter: fm, Vet: vt}}
}

func TestFormatDetailsPass(t *testing.T) {
	outcome := outcomeFor(
		report.ModeFormat,
		formatterengine.Report{Result: formatterengine.ResultPass, Files: 2},
		vet.Report{Root: "/work"},
	)

	assertDetails(t, formatDetails(outcome),
		"fmtkit|Formatted 2 file(s).",
		"result|pass. 0 changed, 0 violation(s), 0 error(s).",
		"vet|go vet ./... passed.",
		"vet result|pass. 0 error(s).",
	)
}

func TestFormatDetailsCheckModeVerb(t *testing.T) {
	outcome := outcomeFor(
		report.ModeCheck,
		formatterengine.Report{Result: formatterengine.ResultPass, Files: 3},
		vet.Report{Root: "/work"},
	)

	assertDetails(t, formatDetails(outcome),
		"fmtkit|Checked 3 file(s).",
		"result|pass. 0 changed, 0 violation(s), 0 error(s).",
		"vet|go vet ./... passed.",
		"vet result|pass. 0 error(s).",
	)
}

// TestFormatDetailsNoFiles reproduces the scraper's quirk: with no formatter
// Result line rendered, the "result" detail borrows the vet Result line and the
// separate "vet result" line is suppressed (they are identical).
func TestFormatDetailsNoFiles(t *testing.T) {
	outcome := outcomeFor(
		report.ModeFormat,
		formatterengine.Report{Result: formatterengine.ResultPass, Files: 0},
		vet.Report{Root: "/work"},
	)

	assertDetails(t, formatDetails(outcome),
		"fmtkit|No Go files found.",
		"result|pass. 0 error(s).",
		"vet|go vet ./... passed.",
	)
}

func TestFormatDetailsVetSkippedNoModule(t *testing.T) {
	outcome := outcomeFor(
		report.ModeFormat,
		formatterengine.Report{Result: formatterengine.ResultPass, Files: 1},
		vet.Report{Root: ""},
	)

	assertDetails(t, formatDetails(outcome),
		"fmtkit|Formatted 1 file(s).",
		"result|pass. 0 changed, 0 violation(s), 0 error(s).",
		"vet|Skipped automatic go vet ./... because no Go module or workspace was detected.",
		"vet result|skipped. 0 error(s).",
	)
}

func TestFormatDetailsVetSkippedToolchain(t *testing.T) {
	outcome := outcomeFor(
		report.ModeFormat,
		formatterengine.Report{Result: formatterengine.ResultPass, Files: 1},
		vet.Report{Root: "/work", Skipped: true},
	)

	assertDetails(t, formatDetails(outcome),
		"fmtkit|Formatted 1 file(s).",
		"result|pass. 0 changed, 0 violation(s), 0 error(s).",
		"vet|Skipped automatic go vet ./... because the Go toolchain is not available.",
		"vet result|skipped. 0 error(s).",
	)
}

// TestFormatDetailsVetFailure: a vet failure renders per-error lines instead of
// a status summary, so there is no "vet" detail, but the differing vet Result
// line still appears.
func TestFormatDetailsVetFailure(t *testing.T) {
	outcome := outcomeFor(
		report.ModeFormat,
		formatterengine.Report{Result: formatterengine.ResultPass, Files: 2},
		vet.Report{Root: "/work", Errors: []vet.ErrorResult{{File: "a.go", Message: "boom"}}},
	)

	assertDetails(t, formatDetails(outcome),
		"fmtkit|Formatted 2 file(s).",
		"result|pass. 0 changed, 0 violation(s), 0 error(s).",
		"vet result|fail. 1 error(s).",
	)
}

func TestSteps(t *testing.T) {
	steps := New().Steps(toolchain.Request{Paths: []string{"."}})

	if len(steps) != 1 {
		t.Fatalf("Steps len = %d, want 1", len(steps))
	}

	if got := steps[0].Label(); got != "Running Go formatting" {
		t.Fatalf("step label = %q, want %q", got, "Running Go formatting")
	}

	if got := New().Name(); got != "go" {
		t.Fatalf("Name = %q, want go", got)
	}
}
