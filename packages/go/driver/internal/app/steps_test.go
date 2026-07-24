package app

import (
	"bytes"
	"errors"
	"fmt"
	"testing"

	"go.ollin.sh/fmtkit/driver/internal/gotool"
	"go.ollin.sh/fmtkit/driver/internal/pipeline"
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

// goOutcome builds a Go outcome for the given mode, formatter report, and vet
// report.
func goOutcome(mode report.Mode, fm formatterengine.Report, vt vet.Report) gotool.Outcome {
	return gotool.Outcome{Mode: mode, Combined: report.Combined{Formatter: fm, Vet: vt}}
}

func TestGoFormatDetailsPass(t *testing.T) {
	outcome := goOutcome(
		report.ModeFormat,
		formatterengine.Report{Result: formatterengine.ResultPass, Files: 2},
		vet.Report{Root: "/work"},
	)

	assertDetails(t, goFormatDetails(outcome),
		"fmtkit|Formatted 2 file(s).",
		"result|pass. 0 changed, 0 violation(s), 0 error(s).",
		"vet|go vet ./... passed.",
		"vet result|pass. 0 error(s).",
	)
}

func TestGoFormatDetailsCheckModeVerb(t *testing.T) {
	outcome := goOutcome(
		report.ModeCheck,
		formatterengine.Report{Result: formatterengine.ResultPass, Files: 3},
		vet.Report{Root: "/work"},
	)

	assertDetails(t, goFormatDetails(outcome),
		"fmtkit|Checked 3 file(s).",
		"result|pass. 0 changed, 0 violation(s), 0 error(s).",
		"vet|go vet ./... passed.",
		"vet result|pass. 0 error(s).",
	)
}

// TestGoFormatDetailsNoFiles reproduces the scraper's quirk: with no formatter
// Result line rendered, the "result" detail borrows the vet Result line and the
// separate "vet result" line is suppressed (they are identical).
func TestGoFormatDetailsNoFiles(t *testing.T) {
	outcome := goOutcome(
		report.ModeFormat,
		formatterengine.Report{Result: formatterengine.ResultPass, Files: 0},
		vet.Report{Root: "/work"},
	)

	assertDetails(t, goFormatDetails(outcome),
		"fmtkit|No Go files found.",
		"result|pass. 0 error(s).",
		"vet|go vet ./... passed.",
	)
}

func TestGoFormatDetailsVetSkippedNoModule(t *testing.T) {
	outcome := goOutcome(
		report.ModeFormat,
		formatterengine.Report{Result: formatterengine.ResultPass, Files: 1},
		vet.Report{Root: ""},
	)

	assertDetails(t, goFormatDetails(outcome),
		"fmtkit|Formatted 1 file(s).",
		"result|pass. 0 changed, 0 violation(s), 0 error(s).",
		"vet|Skipped automatic go vet ./... because no Go module or workspace was detected.",
		"vet result|skipped. 0 error(s).",
	)
}

func TestGoFormatDetailsVetSkippedToolchain(t *testing.T) {
	outcome := goOutcome(
		report.ModeFormat,
		formatterengine.Report{Result: formatterengine.ResultPass, Files: 1},
		vet.Report{Root: "/work", Skipped: true},
	)

	assertDetails(t, goFormatDetails(outcome),
		"fmtkit|Formatted 1 file(s).",
		"result|pass. 0 changed, 0 violation(s), 0 error(s).",
		"vet|Skipped automatic go vet ./... because the Go toolchain is not available.",
		"vet result|skipped. 0 error(s).",
	)
}

// TestGoFormatDetailsVetFailure: a vet failure renders per-error lines instead
// of a status summary, so there is no "vet" detail, but the differing vet Result
// line still appears.
func TestGoFormatDetailsVetFailure(t *testing.T) {
	outcome := goOutcome(
		report.ModeFormat,
		formatterengine.Report{Result: formatterengine.ResultPass, Files: 2},
		vet.Report{Root: "/work", Errors: []vet.ErrorResult{{File: "a.go", Message: "boom"}}},
	)

	assertDetails(t, goFormatDetails(outcome),
		"fmtkit|Formatted 2 file(s).",
		"result|pass. 0 changed, 0 violation(s), 0 error(s).",
		"vet result|fail. 1 error(s).",
	)
}

func TestTSFormatDetails(t *testing.T) {
	log := "[blank-lines] processed 3 file(s) in /work, 0 changed\n" +
		"Finished in 10ms on 3 files using 8 threads.\n" +
		"[fluent-chains] processed 3 file(s) in /work, 1 changed\n" +
		"[validate-syntax] checked 3 file(s).\n"

	assertDetails(t, tsFormatDetails(log),
		"blank-lines|processed 3 file(s) in /work, 0 changed",
		"oxfmt|Finished in 10ms on 3 files using 8 threads.",
		"fluent|processed 3 file(s) in /work, 1 changed",
		"validated|checked 3 file(s).",
	)
}

func TestTSFormatDetailsCountsMissing(t *testing.T) {
	log := "[sources] path not found, skipping: /work/a\n" +
		"[sources] path not found, skipping: /work/b\n"

	assertDetails(t, tsFormatDetails(log), "skipped|2 missing tracked file(s)")
}

func TestTSLintDetailsResult(t *testing.T) {
	assertDetails(t, tsLintDetails("Found 0 warnings and 0 errors.\n"), "oxlint|Found 0 warnings and 0 errors.")
}

func TestTSLintDetailsNoFiles(t *testing.T) {
	assertDetails(t, tsLintDetails("[lint] no TS/Vue files to lint.\n"), "oxlint|no TS/Vue files to lint.")
}

func TestTSLintDetailsFallback(t *testing.T) {
	assertDetails(t, tsLintDetails("nothing interesting\n"), "oxlint|no issues found")
}

func TestTSExitCodePlainErrorWritesToOutput(t *testing.T) {
	var buf bytes.Buffer

	if code := tsExitCode(errors.New("boom"), &buf); code != 1 {
		t.Fatalf("tsExitCode = %d, want 1", code)
	}

	if buf.String() != "boom\n" {
		t.Fatalf("tsExitCode output = %q, want %q", buf.String(), "boom\n")
	}
}

func TestTSExitCodeNil(t *testing.T) {
	var buf bytes.Buffer

	if code := tsExitCode(nil, &buf); code != 0 {
		t.Fatalf("tsExitCode(nil) = %d, want 0", code)
	}

	if buf.Len() != 0 {
		t.Fatalf("tsExitCode(nil) wrote %q", buf.String())
	}
}

func TestStepSelectionNormalized(t *testing.T) {
	if got := (stepSelection{}).normalized(); !got.TS || !got.Go {
		t.Fatalf("zero selection = %+v, want both set", got)
	}

	if got := (stepSelection{TS: true}).normalized(); !got.TS || got.Go {
		t.Fatalf("TS-only selection = %+v, want TS only", got)
	}

	if got := (stepSelection{Go: true}).normalized(); got.TS || !got.Go {
		t.Fatalf("Go-only selection = %+v, want Go only", got)
	}
}

func TestFormatStepsSelection(t *testing.T) {
	d := &deps{version: "dev"}

	labels := func(steps []pipeline.Step) []string {
		out := make([]string, 0, len(steps))

		for _, s := range steps {
			out = append(out, s.Label())
		}

		return out
	}

	all := labels(d.formatSteps([]string{"."}, stepSelection{}, 0))
	if fmt.Sprint(all) != fmt.Sprint([]string{"Running TS/Vue lint", "Running TS/Vue formatting", "Running Go formatting"}) {
		t.Fatalf("default steps = %v", all)
	}

	tsOnly := labels(d.formatSteps([]string{"."}, stepSelection{TS: true}, 0))
	if fmt.Sprint(tsOnly) != fmt.Sprint([]string{"Running TS/Vue lint", "Running TS/Vue formatting"}) {
		t.Fatalf("--ts steps = %v", tsOnly)
	}

	goOnly := labels(d.formatSteps([]string{"."}, stepSelection{Go: true}, 0))
	if fmt.Sprint(goOnly) != fmt.Sprint([]string{"Running Go formatting"}) {
		t.Fatalf("--go steps = %v", goOnly)
	}
}
