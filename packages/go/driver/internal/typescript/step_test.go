package typescript

import (
	"bytes"
	"errors"
	"fmt"
	"testing"

	"go.ollin.sh/fmtkit/driver/internal/pipeline"
	"go.ollin.sh/fmtkit/driver/internal/toolchain"
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

func TestFormatDetails(t *testing.T) {
	log := "[blank-lines] processed 3 file(s) in /work, 0 changed\n" +
		"Finished in 10ms on 3 files using 8 threads.\n" +
		"[fluent-chains] processed 3 file(s) in /work, 1 changed\n" +
		"[validate-syntax] checked 3 file(s).\n"

	assertDetails(t, formatDetails(log),
		"blank-lines|processed 3 file(s) in /work, 0 changed",
		"oxfmt|Finished in 10ms on 3 files using 8 threads.",
		"fluent|processed 3 file(s) in /work, 1 changed",
		"validated|checked 3 file(s).",
	)
}

func TestFormatDetailsCountsMissing(t *testing.T) {
	log := "[sources] path not found, skipping: /work/a\n" +
		"[sources] path not found, skipping: /work/b\n"

	assertDetails(t, formatDetails(log), "skipped|2 missing tracked file(s)")
}

func TestLintDetailsResult(t *testing.T) {
	assertDetails(t, lintDetails("Found 0 warnings and 0 errors.\n"), "oxlint|Found 0 warnings and 0 errors.")
}

func TestLintDetailsNoFiles(t *testing.T) {
	assertDetails(t, lintDetails("[lint] no TS/Vue files to lint.\n"), "oxlint|no TS/Vue files to lint.")
}

func TestLintDetailsFallback(t *testing.T) {
	assertDetails(t, lintDetails("nothing interesting\n"), "oxlint|no issues found")
}

func TestExitCodePlainErrorWritesToOutput(t *testing.T) {
	var buf bytes.Buffer

	if code := exitCode(errors.New("boom"), &buf); code != 1 {
		t.Fatalf("exitCode = %d, want 1", code)
	}

	if buf.String() != "boom\n" {
		t.Fatalf("exitCode output = %q, want %q", buf.String(), "boom\n")
	}
}

func TestExitCodeNil(t *testing.T) {
	var buf bytes.Buffer

	if code := exitCode(nil, &buf); code != 0 {
		t.Fatalf("exitCode(nil) = %d, want 0", code)
	}

	if buf.Len() != 0 {
		t.Fatalf("exitCode(nil) wrote %q", buf.String())
	}
}

func TestSteps(t *testing.T) {
	steps := New().Steps(toolchain.Request{Version: "dev", Paths: []string{"."}})

	labels := make([]string, 0, len(steps))

	for _, s := range steps {
		labels = append(labels, s.Label())
	}

	if got := fmt.Sprint(labels); got != fmt.Sprint([]string{"Running TS/Vue lint", "Running TS/Vue formatting"}) {
		t.Fatalf("step labels = %s, want [Running TS/Vue lint, Running TS/Vue formatting]", got)
	}

	if got := New().Name(); got != "ts" {
		t.Fatalf("Name = %q, want ts", got)
	}
}
