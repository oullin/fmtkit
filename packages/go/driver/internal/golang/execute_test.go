package golang

import (
	"context"
	"os"
	"path/filepath"
	"testing"

	driverconfig "go.ollin.sh/fmtkit/driver/config"
	"go.ollin.sh/fmtkit/driver/internal/gitfiles"
	report "go.ollin.sh/fmtkit/driver/report"
	formatterengine "go.ollin.sh/fmtkit/formatter/engine"
)

func TestExecuteCheckReportsViolationWithoutRewriting(t *testing.T) {
	dir := t.TempDir()
	file := filepath.Join(dir, "sample.go")

	if err := os.WriteFile(file, []byte(spacingViolationSource), 0o644); err != nil {
		t.Fatalf("write sample: %v", err)
	}

	outcome, err := Execute(context.Background(), Request{
		Mode:   report.ModeCheck,
		Paths:  []string{file},
		Config: driverconfig.Default(),
		Root:   dir,
	})

	if err != nil {
		t.Fatalf("execute: %v", err)
	}

	if outcome.Mode != report.ModeCheck {
		t.Fatalf("outcome mode = %q", outcome.Mode)
	}

	if outcome.Combined.Formatter.Result == formatterengine.ResultPass {
		t.Fatalf("expected a non-pass result for the violation")
	}

	if outcome.ExitCode() != 1 {
		t.Fatalf("check exit = %d, want 1", outcome.ExitCode())
	}

	if got, _ := os.ReadFile(file); string(got) != spacingViolationSource {
		t.Fatal("check mode must not rewrite the file")
	}
}

func TestExecuteFormatRewritesFile(t *testing.T) {
	dir := t.TempDir()
	file := filepath.Join(dir, "sample.go")

	if err := os.WriteFile(file, []byte(spacingViolationSource), 0o644); err != nil {
		t.Fatalf("write sample: %v", err)
	}

	outcome, err := Execute(context.Background(), Request{
		Mode:   report.ModeFormat,
		Paths:  []string{file},
		Config: driverconfig.Default(),
		Root:   dir,
	})

	if err != nil {
		t.Fatalf("execute: %v", err)
	}

	if outcome.ExitCode() != 0 {
		t.Fatalf("format exit = %d, want 0", outcome.ExitCode())
	}

	got, err := os.ReadFile(file)

	if err != nil {
		t.Fatalf("read sample: %v", err)
	}

	if string(got) == spacingViolationSource {
		t.Fatal("format mode should rewrite the file")
	}
}

func TestExecuteChangedScopeOutsideGitTreeErrors(t *testing.T) {
	dir := t.TempDir()
	file := filepath.Join(dir, "sample.go")

	if err := os.WriteFile(file, []byte(cleanSource), 0o644); err != nil {
		t.Fatalf("write sample: %v", err)
	}

	// A changed scope needs a git tree; a bare temp dir has none, so Execute must
	// surface the error rather than silently formatting everything.
	_, err := Execute(context.Background(), Request{
		Mode:   report.ModeFormat,
		Paths:  []string{file},
		Config: driverconfig.Default(),
		Root:   dir,
		Scope:  gitfiles.SelectionChanged,
	})

	if err == nil {
		t.Fatal("expected an error scoping to changes outside a git tree")
	}
}
