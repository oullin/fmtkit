package orchestrator

import (
	"bytes"
	"errors"
	"fmt"
	"io"
	"os"
	"strings"
	"testing"
)

// TestMain pins a color-free environment: CI task runners export FORCE_COLOR,
// which would inject ANSI codes into the captured output these tests assert.

// The stub outputs mirror infra/test-binary-smoke.sh so
// the Go orchestrator preserves the entrypoint's summary contract.

type invocation struct {
	tool string
	args []string
}

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
		TS: func(scopes []string, output io.Writer) error {
			*log = append(*log, invocation{"ts", scopes})

			_, _ = io.WriteString(output, stubTSOutput)

			return tsErr
		},
		Lint: func(scopes []string, output io.Writer) error {
			*log = append(*log, invocation{"lint", scopes})

			_, _ = io.WriteString(output, stubLintOutput)

			return lintErr
		},
		Go: func(args []string, output io.Writer) int {
			*log = append(*log, invocation{"go", args})

			_, _ = io.WriteString(output, stubGoOutput)

			return goCode
		},
	}
}

func TestRunFormatRunsStepsInOrder(t *testing.T) {
	var log []invocation

	var stderr bytes.Buffer

	pipeline := Pipeline{
		Tools:  stubTools(&log, nil, nil, 0),
		Stderr: &stderr,
	}

	if code := pipeline.RunFormat([]string{"."}); code != 0 {
		t.Fatalf("RunFormat = %d, want 0\n%s", code, stderr.String())
	}

	want := []invocation{
		{"ts", []string{"."}},
		{"lint", []string{"."}},
		{"go", []string{"format", "."}},
	}

	if fmt.Sprint(log) != fmt.Sprint(want) {
		t.Fatalf("invocations = %v, want %v", log, want)
	}

	for _, needle := range []string{
		"==> Formatting target(s)",
		"paths        .",
		"==> Running TS/Vue formatting",
		"blank-lines  processed 3 file(s) in /work, 0 changed",
		"oxfmt        Finished in 10ms on 3 files using 8 threads.",
		"fluent       processed 3 file(s) in /work, 1 changed",
		"==> Running TS/Vue lint",
		"oxlint       Found 0 warnings and 0 errors.",
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

	if code := pipeline.RunFormat(nil); code != 0 {
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

	if code := pipeline.RunFormat(nil); code != 0 {
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

	if code := pipeline.RunFormat(nil); code != 1 {
		t.Fatalf("RunFormat = %d, want 1", code)
	}

	if len(log) != 1 || log[0].tool != "ts" {
		t.Fatalf("invocations = %v, want only ts", log)
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

	if code := pipeline.RunFormat(nil); code != 3 {
		t.Fatalf("RunFormat = %d, want 3", code)
	}

	if !strings.Contains(stderr.String(), "Formatted 2 file(s).") {
		t.Fatalf("quiet failure did not dump captured log:\n%s", stderr.String())
	}
}

func TestSummarizeTSLintFallbacks(t *testing.T) {
	var out bytes.Buffer

	log := newLogger(&out, true)

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

	log := newLogger(&out, true)

	summarizeTSFormat("[sources] path not found, skipping: /work/a\n[sources] path not found, skipping: /work/b\n", log)

	if !strings.Contains(out.String(), "skipped      2 missing tracked file(s)") {
		t.Fatalf("missing skipped summary: %q", out.String())
	}
}
