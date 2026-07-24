package orchestrator

import (
	"bytes"
	"context"
	"flag"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"go.ollin.sh/fmtkit/driver/internal/console"
)

var updateGolden = flag.Bool("update", false, "rewrite pipeline transcript golden files")

// fakeStep is a scripted Step: it streams a canned tool log to output, appends
// an optional trailing message (a non-exec error the real steps surface through
// output), and returns a fixed Result. It mirrors the tool stubs the earlier
// func-triple fakes used, now expressed against the Step interface.
type fakeStep struct {
	label    string
	output   string
	trailing string
	details  []Detail
	code     int

	log *[]string
}

func (s fakeStep) Label() string { return s.label }

func (s fakeStep) Run(_ context.Context, output io.Writer) Result {
	if s.log != nil {
		*s.log = append(*s.log, s.label)
	}

	_, _ = io.WriteString(output, s.output)

	if s.trailing != "" {
		_, _ = io.WriteString(output, s.trailing)
	}

	if s.code != 0 {
		return Result{ExitCode: s.code}
	}

	return Result{Details: s.details}
}

const (
	stubTSOutput = "[blank-lines] processed 3 file(s) in /work, 0 changed\n" +
		"Finished in 10ms on 3 files using 8 threads.\n" +
		"[fluent-chains] processed 3 file(s) in /work, 1 changed\n"

	stubLintOutput = "Found 0 warnings and 0 errors.\n"

	stubGoOutput = "\nFormatter\n\n" +
		"  Formatted 2 file(s).\n\n" +
		"  Result: pass. 0 changed, 0 violation(s), 0 error(s).\n\n" +
		"Vet\n\n" +
		"  go vet ./... passed.\n\n" +
		"  Result: pass. 0 error(s).\n"
)

var (
	lintDetails = []Detail{{"oxlint", "Found 0 warnings and 0 errors."}}

	tsDetails = []Detail{
		{"blank-lines", "processed 3 file(s) in /work, 0 changed"},
		{"oxfmt", "Finished in 10ms on 3 files using 8 threads."},
		{"fluent", "processed 3 file(s) in /work, 1 changed"},
	}

	goDetails = []Detail{
		{"fmtkit", "Formatted 2 file(s)."},
		{"result", "pass. 0 changed, 0 violation(s), 0 error(s)."},
		{"vet", "go vet ./... passed."},
		{"vet result", "pass. 0 error(s)."},
	}
)

// runFormat frames the three scripted steps exactly as the app composition root
// does (target header, completion footer), so the transcript the goldens pin is
// reproduced end to end without importing the app package.
func runFormat(t *testing.T, stderr io.Writer, quiet bool, steps []Step) int {
	t.Helper()

	printer := console.NewPrinter(stderr, console.ColorNever)

	printer.Section("Formatting target(s)")
	printer.Detail("paths", ".")

	code := Pipeline{Steps: steps, Quiet: quiet, Printer: printer, Stderr: stderr}.Run(context.Background())

	if code == 0 {
		printer.Section("Formatting complete")
		printer.SuccessDetail("status", "done")
	}

	return code
}

// successSteps are the three passing steps in pipeline order (lint, TS, Go).
func successSteps(log *[]string) []Step {
	return []Step{
		fakeStep{label: "Running TS/Vue lint", output: stubLintOutput, details: lintDetails, log: log},
		fakeStep{label: "Running TS/Vue formatting", output: stubTSOutput, details: tsDetails, log: log},
		fakeStep{label: "Running Go formatting", output: stubGoOutput, details: goDetails, log: log},
	}
}

// TestRunFormatTranscriptGoldens pins the complete stderr transcript the
// pipeline renders, byte for byte, across the success and failure paths in both
// streaming and quiet modes. Color is forced off, so the golden files carry no
// ANSI escapes. These goldens characterize the rendering so refactors cannot
// silently change it; regenerate with
// `go test ./driver/internal/orchestrator -run TestRunFormatTranscriptGoldens -update`.
func TestRunFormatTranscriptGoldens(t *testing.T) {
	cases := []struct {
		name   string
		quiet  bool
		steps  []Step
		golden string
	}{
		{"success", false, successSteps(nil), "transcript_success.txt"},
		{"success_quiet", true, successSteps(nil), "transcript_success_quiet.txt"},
		{
			"go_failure", false,
			[]Step{
				fakeStep{label: "Running TS/Vue lint", output: stubLintOutput, details: lintDetails},
				fakeStep{label: "Running TS/Vue formatting", output: stubTSOutput, details: tsDetails},
				fakeStep{label: "Running Go formatting", output: stubGoOutput, code: 3},
			},
			"transcript_go_failure.txt",
		},
		{
			"go_failure_quiet", true,
			[]Step{
				fakeStep{label: "Running TS/Vue lint", output: stubLintOutput, details: lintDetails},
				fakeStep{label: "Running TS/Vue formatting", output: stubTSOutput, details: tsDetails},
				fakeStep{label: "Running Go formatting", output: stubGoOutput, code: 3},
			},
			"transcript_go_failure_quiet.txt",
		},
		{
			"ts_failure", false,
			[]Step{
				fakeStep{label: "Running TS/Vue lint", output: stubLintOutput, details: lintDetails},
				fakeStep{label: "Running TS/Vue formatting", output: stubTSOutput, trailing: "sidecar exploded\n", code: 1},
			},
			"transcript_ts_failure.txt",
		},
	}

	for _, tc := range cases {
		tc := tc

		t.Run(tc.name, func(t *testing.T) {
			var stderr bytes.Buffer

			runFormat(t, &stderr, tc.quiet, tc.steps)

			path := filepath.Join("testdata", tc.golden)

			if *updateGolden {
				if err := os.WriteFile(path, stderr.Bytes(), 0o644); err != nil {
					t.Fatalf("update golden: %v", err)
				}

				return
			}

			want, err := os.ReadFile(path)

			if err != nil {
				t.Fatalf("read golden: %v", err)
			}

			if stderr.String() != string(want) {
				t.Fatalf("transcript mismatch for %s\n--- got ---\n%s\n--- want ---\n%s", tc.golden, stderr.String(), want)
			}
		})
	}
}

func TestRunRunsStepsInOrder(t *testing.T) {
	var log []string

	var stderr bytes.Buffer

	if code := runFormat(t, &stderr, false, successSteps(&log)); code != 0 {
		t.Fatalf("Run = %d, want 0\n%s", code, stderr.String())
	}

	want := []string{"Running TS/Vue lint", "Running TS/Vue formatting", "Running Go formatting"}

	if fmt.Sprint(log) != fmt.Sprint(want) {
		t.Fatalf("step order = %v, want %v", log, want)
	}

	for _, needle := range []string{
		"==> Formatting target(s)",
		"paths        .",
		"==> Running TS/Vue lint",
		"oxlint       Found 0 warnings and 0 errors.",
		"==> Running TS/Vue formatting",
		"blank-lines  processed 3 file(s) in /work, 0 changed",
		"oxfmt        Finished in 10ms on 3 files using 8 threads.",
		"fluent       processed 3 file(s) in /work, 1 changed",
		"==> Running Go formatting",
		"fmtkit       Formatted 2 file(s).",
		"result       pass. 0 changed, 0 violation(s), 0 error(s).",
		"vet          go vet ./... passed.",
		"vet result   pass. 0 error(s).",
		"==> Formatting complete",
		"status",
		"done",
	} {
		if !strings.Contains(stderr.String(), needle) {
			t.Fatalf("stderr missing %q:\n%s", needle, stderr.String())
		}
	}
}

func TestRunStreamsToolOutputLive(t *testing.T) {
	var stderr bytes.Buffer

	if code := runFormat(t, &stderr, false, successSteps(nil)); code != 0 {
		t.Fatalf("Run = %d, want 0", code)
	}

	// The raw tool line appears indented (live stream) in addition to the
	// condensed detail line.
	if !strings.Contains(stderr.String(), "    [blank-lines] processed 3 file(s) in /work, 0 changed") {
		t.Fatalf("stderr missing live-streamed tool output:\n%s", stderr.String())
	}
}

func TestRunQuietHidesToolOutput(t *testing.T) {
	var stderr bytes.Buffer

	if code := runFormat(t, &stderr, true, successSteps(nil)); code != 0 {
		t.Fatalf("Run = %d, want 0", code)
	}

	if strings.Contains(stderr.String(), "    [blank-lines]") {
		t.Fatalf("quiet mode streamed tool output:\n%s", stderr.String())
	}

	if !strings.Contains(stderr.String(), "blank-lines  processed 3 file(s) in /work, 0 changed") {
		t.Fatalf("quiet mode lost detail:\n%s", stderr.String())
	}
}

func TestRunShortCircuitsOnFailure(t *testing.T) {
	var log []string

	var stderr bytes.Buffer

	steps := []Step{
		fakeStep{label: "Running TS/Vue lint", output: stubLintOutput, details: lintDetails, log: &log},
		fakeStep{label: "Running TS/Vue formatting", output: stubTSOutput, trailing: "sidecar exploded\n", code: 1, log: &log},
		fakeStep{label: "Running Go formatting", output: stubGoOutput, details: goDetails, log: &log},
	}

	if code := runFormat(t, &stderr, false, steps); code != 1 {
		t.Fatalf("Run = %d, want 1", code)
	}

	want := []string{"Running TS/Vue lint", "Running TS/Vue formatting"}

	if fmt.Sprint(log) != fmt.Sprint(want) {
		t.Fatalf("step order = %v, want %v (Go should have been skipped)", log, want)
	}

	if !strings.Contains(stderr.String(), "!! Running TS/Vue formatting failed") {
		t.Fatalf("stderr missing failure banner:\n%s", stderr.String())
	}

	if !strings.Contains(stderr.String(), "sidecar exploded") {
		t.Fatalf("stderr missing error message:\n%s", stderr.String())
	}
}

func TestRunQuietDumpsLogOnFailure(t *testing.T) {
	var stderr bytes.Buffer

	steps := []Step{
		fakeStep{label: "Running Go formatting", output: stubGoOutput, code: 3},
	}

	if code := runFormat(t, &stderr, true, steps); code != 3 {
		t.Fatalf("Run = %d, want 3", code)
	}

	if !strings.Contains(stderr.String(), "Formatted 2 file(s).") {
		t.Fatalf("quiet failure did not dump captured log:\n%s", stderr.String())
	}
}
