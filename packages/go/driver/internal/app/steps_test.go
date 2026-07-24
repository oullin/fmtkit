package app

import (
	"bytes"
	"errors"
	"fmt"
	"testing"

	"go.ollin.sh/fmtkit/driver/internal/pipeline"
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
