package complexity_test

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	"go.ollin.sh/fmtkit/complexity"
)

// goScan is a one-function Go measurement over a file that really exists, so
// the allow-staleness rules have a working tree to read.
func goScan(t *testing.T, key string, cyclomatic, cognitive int) (string, complexity.Scan) {
	t.Helper()

	root := t.TempDir()
	file := complexity.KeyFile(key)

	if err := os.WriteFile(filepath.Join(root, file), []byte("package shapes\n"), 0o600); err != nil {
		t.Fatalf("write source: %v", err)
	}

	return root, complexity.Scan{
		Lane:  complexity.LaneGo,
		Files: []string{file},
		Functions: []complexity.Function{{
			Key:        key,
			File:       file,
			Line:       3,
			Cyclomatic: cyclomatic,
			Cognitive:  cognitive,
		}},
	}
}

func TestEvaluateReportsBothMetricsOverTheirLimits(t *testing.T) {
	root, scan := goScan(t, "shapes.go#Read", 31, 42)

	out := complexity.Evaluate(root, complexity.Default(), []complexity.Scan{scan})

	if out.FindingCount() != 2 {
		t.Fatalf("findings = %#v", out.Findings)
	}

	if out.Status() != "fail" {
		t.Errorf("status = %q, want fail", out.Status())
	}

	want := "shapes.go#Read scores 31 (limit 15)"

	if out.Findings[0].Rule != complexity.RuleCognitive {
		t.Errorf("findings are not sorted by rule: %#v", out.Findings)
	}

	if out.Findings[1].Message != want {
		t.Errorf("message = %q, want %q", out.Findings[1].Message, want)
	}
}

func TestEvaluatePassesWhenBothMetricsSitOnTheirLimit(t *testing.T) {
	root, scan := goScan(t, "shapes.go#Read", complexity.DefaultCyclomatic, complexity.DefaultCognitive)

	out := complexity.Evaluate(root, complexity.Default(), []complexity.Scan{scan})

	if out.Status() != "pass" || out.FindingCount() != 0 {
		t.Fatalf("report = %#v", out)
	}

	if out.Functions != 1 || out.Files != 1 {
		t.Errorf("counts = %d function(s), %d file(s)", out.Functions, out.Files)
	}
}

func TestEvaluateSuppressesAnAllowedFunction(t *testing.T) {
	root, scan := goScan(t, "shapes.go#Read", 31, 42)

	cfg := complexity.Default()
	cfg.Allow = []complexity.AllowEntry{{Key: "shapes.go#Read", Reason: "burnt down next"}}

	out := complexity.Evaluate(root, cfg, []complexity.Scan{scan})

	if out.FindingCount() != 0 {
		t.Fatalf("findings = %#v", out.Findings)
	}
}

func TestEvaluateReportsAnAllowEntryThatMatchesNoFunction(t *testing.T) {
	root, scan := goScan(t, "shapes.go#Read", 1, 1)

	cfg := complexity.Default()
	cfg.Allow = []complexity.AllowEntry{{Key: "shapes.go#Gone", Reason: "renamed away"}}

	out := complexity.Evaluate(root, cfg, []complexity.Scan{scan})

	if out.FindingCount() != 1 || out.Findings[0].Rule != complexity.RuleAllow {
		t.Fatalf("findings = %#v", out.Findings)
	}

	if !strings.Contains(out.Findings[0].Message, "shapes.go#Gone") {
		t.Errorf("message = %q", out.Findings[0].Message)
	}
}

func TestEvaluateReportsAnAllowEntryWhoseFileIsGone(t *testing.T) {
	root, scan := goScan(t, "shapes.go#Read", 1, 1)

	cfg := complexity.Default()
	cfg.Allow = []complexity.AllowEntry{{Key: "deleted.go#Read", Reason: "file is gone"}}

	out := complexity.Evaluate(root, cfg, []complexity.Scan{scan})

	if out.FindingCount() != 1 || out.Findings[0].File != "deleted.go" {
		t.Fatalf("findings = %#v", out.Findings)
	}
}

// An allow entry for a file this run did not cover is somebody else's scope,
// not a stale entry: a --go run must not trip over the TypeScript baseline,
// and a run scoped to one directory must not trip over another's.
func TestEvaluateIgnoresAllowEntriesOutsideTheRunsScope(t *testing.T) {
	root, scan := goScan(t, "shapes.go#Read", 1, 1)

	if err := os.WriteFile(filepath.Join(root, "other.go"), []byte("package shapes\n"), 0o600); err != nil {
		t.Fatalf("write source: %v", err)
	}

	cfg := complexity.Default()
	cfg.Allow = []complexity.AllowEntry{
		{Key: "other.go#Read", Reason: "not covered by this run"},
		{Key: "src/app.ts#read", Reason: "the other lane's baseline"},
	}

	out := complexity.Evaluate(root, cfg, []complexity.Scan{scan})

	if out.FindingCount() != 0 {
		t.Fatalf("findings = %#v", out.Findings)
	}
}

func TestEvaluateWithoutLanesIsSkipped(t *testing.T) {
	out := complexity.Evaluate(t.TempDir(), complexity.Default(), nil)

	if out.Status() != "skipped" {
		t.Fatalf("status = %q, want skipped", out.Status())
	}
}

func TestEvaluateCarriesLaneErrorsIntoTheReport(t *testing.T) {
	out := complexity.Evaluate(t.TempDir(), complexity.Default(), []complexity.Scan{{
		Lane:   complexity.LaneTS,
		Errors: []complexity.ErrorResult{{File: "src/app.ts", Message: "unparsable"}},
	}})

	if out.ErrorCount() != 1 || out.Status() != "fail" {
		t.Fatalf("report = %#v", out)
	}
}

// A limit of zero or less turns its metric off without touching the other.
func TestEvaluateTreatsANonPositiveLimitAsOff(t *testing.T) {
	root, scan := goScan(t, "shapes.go#Read", 31, 42)

	out := complexity.Evaluate(root, complexity.Config{Cyclomatic: 0, Cognitive: 20}, []complexity.Scan{scan})

	if out.FindingCount() != 1 || out.Findings[0].Rule != complexity.RuleCognitive {
		t.Fatalf("findings = %#v", out.Findings)
	}
}
