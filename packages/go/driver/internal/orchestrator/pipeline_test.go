package orchestrator

import (
	"bytes"
	"context"
	"errors"
	"flag"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"go.ollin.sh/fmtkit/driver/internal/console"
)

type invocation struct {
	tool string
	args []string
}

var updateGolden = flag.Bool("update", false, "rewrite pipeline transcript golden files")

// TestMain pins a color-free environment: CI task runners export FORCE_COLOR,
// which would inject ANSI codes into the captured output these tests assert.

// The stub outputs mirror infra/test-binary-smoke.sh so
// the Go orchestrator preserves the entrypoint's summary contract.

func TestMain(m *testing.M) {
	_ = os.Unsetenv("FORCE_COLOR")
	_ = os.Setenv("NO_COLOR", "1")

	os.Exit(m.Run())
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

func stubTools(log *[]invocation, tsErr, lintErr error, goCode int) Tools {
	return Tools{
		TS: func(_ context.Context, scopes []string, output io.Writer) error {
			*log = append(*log, invocation{"ts", scopes})

			_, _ = io.WriteString(output, stubTSOutput)

			return tsErr
		},
		Lint: func(_ context.Context, scopes []string, output io.Writer) error {
			*log = append(*log, invocation{"lint", scopes})

			_, _ = io.WriteString(output, stubLintOutput)

			return lintErr
		},
		Go: func(_ context.Context, args []string, output io.Writer) int {
			*log = append(*log, invocation{"go", args})

			_, _ = io.WriteString(output, stubGoOutput)

			return goCode
		},
	}
}

// TestRunFormatTranscriptGoldens pins the complete stderr transcript the
// pipeline renders, byte for byte, across the success and failure paths in both
// streaming and quiet modes. Color is forced off by TestMain, so the golden
// files carry no ANSI escapes. These goldens characterize the current
// rendering so later refactor stages cannot silently change it; regenerate with
// `go test ./driver/internal/orchestrator -run TestRunFormatTranscriptGoldens -update`.
func TestRunFormatTranscriptGoldens(t *testing.T) {
	cases := []struct {
		name   string
		quiet  bool
		tsErr  error
		goCode int
		golden string
	}{
		{"success", false, nil, 0, "transcript_success.txt"},
		{"success_quiet", true, nil, 0, "transcript_success_quiet.txt"},
		{"go_failure", false, nil, 3, "transcript_go_failure.txt"},
		{"go_failure_quiet", true, nil, 3, "transcript_go_failure_quiet.txt"},
		{"ts_failure", false, errors.New("sidecar exploded"), 0, "transcript_ts_failure.txt"},
	}

	for _, tc := range cases {
		tc := tc

		t.Run(tc.name, func(t *testing.T) {
			var log []invocation

			var stderr bytes.Buffer

			pipeline := Pipeline{
				Tools:  stubTools(&log, tc.tsErr, nil, tc.goCode),
				Quiet:  tc.quiet,
				Stderr: &stderr,
			}

			pipeline.RunFormat(context.Background(), []string{"."})

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

func TestRunFormatRunsStepsInOrder(t *testing.T) {
	var log []invocation

	var stderr bytes.Buffer

	pipeline := Pipeline{
		Tools:  stubTools(&log, nil, nil, 0),
		Stderr: &stderr,
	}

	if code := pipeline.RunFormat(context.Background(), []string{"."}); code != 0 {
		t.Fatalf("RunFormat = %d, want 0\n%s", code, stderr.String())
	}

	want := []invocation{
		{"lint", []string{"."}},
		{"ts", []string{"."}},
		{"go", []string{"format", "."}},
	}

	if fmt.Sprint(log) != fmt.Sprint(want) {
		t.Fatalf("invocations = %v, want %v", log, want)
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

func TestRunFormatStreamsToolOutputLive(t *testing.T) {
	var log []invocation

	var stderr bytes.Buffer

	pipeline := Pipeline{
		Tools:  stubTools(&log, nil, nil, 0),
		Stderr: &stderr,
	}

	if code := pipeline.RunFormat(context.Background(), nil); code != 0 {
		t.Fatalf("RunFormat = %d, want 0", code)
	}

	// The raw tool line appears indented (live stream) in addition to the
	// condensed summary line.
	if !strings.Contains(stderr.String(), "    [blank-lines] processed 3 file(s) in /work, 0 changed") {
		t.Fatalf("stderr missing live-streamed tool output:\n%s", stderr.String())
	}
}

func TestRunFormatQuietHidesToolOutput(t *testing.T) {
	var log []invocation

	var stderr bytes.Buffer

	pipeline := Pipeline{
		Tools:  stubTools(&log, nil, nil, 0),
		Quiet:  true,
		Stderr: &stderr,
	}

	if code := pipeline.RunFormat(context.Background(), nil); code != 0 {
		t.Fatalf("RunFormat = %d, want 0", code)
	}

	if strings.Contains(stderr.String(), "    [blank-lines]") {
		t.Fatalf("quiet mode streamed tool output:\n%s", stderr.String())
	}

	if !strings.Contains(stderr.String(), "blank-lines  processed 3 file(s) in /work, 0 changed") {
		t.Fatalf("quiet mode lost summary:\n%s", stderr.String())
	}
}

func TestRunFormatShortCircuitsOnTSFailure(t *testing.T) {
	var log []invocation

	var stderr bytes.Buffer

	pipeline := Pipeline{
		Tools:  stubTools(&log, errors.New("sidecar exploded"), nil, 0),
		Stderr: &stderr,
	}

	if code := pipeline.RunFormat(context.Background(), nil); code != 1 {
		t.Fatalf("RunFormat = %d, want 1", code)
	}

	if len(log) != 2 || log[0].tool != "lint" || log[1].tool != "ts" {
		t.Fatalf("invocations = %v, want lint then ts (Go short-circuited)", log)
	}

	if !strings.Contains(stderr.String(), "!! Running TS/Vue formatting failed") {
		t.Fatalf("stderr missing failure banner:\n%s", stderr.String())
	}

	if !strings.Contains(stderr.String(), "sidecar exploded") {
		t.Fatalf("stderr missing error message:\n%s", stderr.String())
	}
}

func TestRunFormatQuietDumpsLogOnFailure(t *testing.T) {
	var log []invocation

	var stderr bytes.Buffer

	pipeline := Pipeline{
		Tools:  stubTools(&log, nil, nil, 3),
		Quiet:  true,
		Stderr: &stderr,
	}

	if code := pipeline.RunFormat(context.Background(), nil); code != 3 {
		t.Fatalf("RunFormat = %d, want 3", code)
	}

	if !strings.Contains(stderr.String(), "Formatted 2 file(s).") {
		t.Fatalf("quiet failure did not dump captured log:\n%s", stderr.String())
	}
}

func TestSummarizeTSLintFallbacks(t *testing.T) {
	var out bytes.Buffer

	log := console.NewPrinter(&out, console.ColorNever)

	summarizeTSLint("[lint] no TS/Vue files to lint.\n", log)

	if !strings.Contains(out.String(), "oxlint       no TS/Vue files to lint.") {
		t.Fatalf("missing skip summary: %q", out.String())
	}

	out.Reset()

	summarizeTSLint("nothing interesting\n", log)

	if !strings.Contains(out.String(), "oxlint       no issues found") {
		t.Fatalf("missing fallback summary: %q", out.String())
	}
}

func TestSummarizeTSFormatCountsMissing(t *testing.T) {
	var out bytes.Buffer

	log := console.NewPrinter(&out, console.ColorNever)

	summarizeTSFormat("[sources] path not found, skipping: /work/a\n[sources] path not found, skipping: /work/b\n", log)

	if !strings.Contains(out.String(), "skipped      2 missing tracked file(s)") {
		t.Fatalf("missing skipped summary: %q", out.String())
	}
}
