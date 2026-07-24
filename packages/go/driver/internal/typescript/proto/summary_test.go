package proto

import "testing"

// sampleTSOutput mirrors the sidecar's pipeline stdout, lifted from the
// pipeline's fake-tool fixtures.
const sampleTSOutput = "[blank-lines] processed 3 file(s) in /work, 0 changed\n" +
	"Finished in 10ms on 3 files using 8 threads.\n" +
	"[fluent-chains] processed 3 file(s) in /work, 1 changed\n" +
	"[validate-syntax] checked 3 file(s), all valid\n"

func TestParsePipelineSummary(t *testing.T) {
	got := ParsePipelineSummary(sampleTSOutput)

	want := PipelineSummary{
		BlankLines:     "processed 3 file(s) in /work, 0 changed",
		Oxfmt:          "Finished in 10ms on 3 files using 8 threads.",
		FluentChains:   "processed 3 file(s) in /work, 1 changed",
		ValidateSyntax: "checked 3 file(s), all valid",
	}

	if got != want {
		t.Fatalf("ParsePipelineSummary() = %+v, want %+v", got, want)
	}
}

func TestParsePipelineSummaryTakesLastOccurrence(t *testing.T) {
	log := "[blank-lines] processed 1 file(s) in /work, 0 changed\n" +
		"[blank-lines] processed 2 file(s) in /work, 1 changed\n"

	if got := ParsePipelineSummary(log).BlankLines; got != "processed 2 file(s) in /work, 1 changed" {
		t.Fatalf("BlankLines = %q, want the last occurrence", got)
	}
}

func TestParsePipelineSummaryEmptyLog(t *testing.T) {
	if got := ParsePipelineSummary(""); got != (PipelineSummary{}) {
		t.Fatalf("ParsePipelineSummary(\"\") = %+v, want zero value", got)
	}
}

func TestParseLintSummary(t *testing.T) {
	if got := ParseLintSummary("Found 0 warnings and 0 errors.\n").Result; got != "Found 0 warnings and 0 errors." {
		t.Fatalf("Result = %q, want the oxlint summary line", got)
	}
}

func TestParseLintSummaryMatchesErrorLine(t *testing.T) {
	if got := ParseLintSummary("noise\nFound 2 warnings and 1 error.\n").Result; got != "Found 2 warnings and 1 error." {
		t.Fatalf("Result = %q, want the matching line", got)
	}
}

func TestParseLintSummaryNoMatch(t *testing.T) {
	if got := ParseLintSummary("nothing interesting\n").Result; got != "" {
		t.Fatalf("Result = %q, want empty when no summary line", got)
	}
}
