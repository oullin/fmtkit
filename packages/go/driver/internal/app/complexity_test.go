package app

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// branchySource is one function whose two metrics both sit above the tight
// limits the fixture config sets, so a run over it reports one finding per
// metric.
const branchySource = `package sample

func Branchy(a, b int) int {
	if a > 0 {
		if b > 0 {
			return 1
		}
	}

	for i := 0; i < a; i++ {
		if b > i {
			return i
		}
	}

	return 0
}
`

// complexityWorkdir stages a tree holding the branchy fixture and a config
// whose limits it exceeds. allow is spliced into the config's allow list.
func complexityWorkdir(t *testing.T, allow string) string {
	t.Helper()

	dir := t.TempDir()

	if err := os.WriteFile(filepath.Join(dir, "sample.go"), []byte(branchySource), 0o600); err != nil {
		t.Fatalf("write fixture: %v", err)
	}

	config := "vet:\n  enabled: false\ncomplexity:\n  cyclomatic: 3\n  cognitive: 3\n" + allow

	if err := os.WriteFile(filepath.Join(dir, "config.yml"), []byte(config), 0o600); err != nil {
		t.Fatalf("write config: %v", err)
	}

	return dir
}

// assertGolden compares output against a pinned transcript. Goldens are added,
// never rewritten: a changed transcript means the report changed.
func assertGolden(t *testing.T, name, got string) {
	t.Helper()

	want, err := os.ReadFile(filepath.Join("testdata", name))

	if err != nil {
		t.Fatalf("read golden: %v", err)
	}

	if got != string(want) {
		t.Fatalf("%s mismatch\n--- got ---\n%s\n--- want ---\n%s", name, got, want)
	}
}

func TestComplexityReportsBothMetricsAndFails(t *testing.T) {
	exitCode, stdout, stderr := runCLI(t, complexityWorkdir(t, ""), "complexity", "--go", ".")

	if exitCode != 1 {
		t.Fatalf("exit = %d, want 1 (stderr: %s)", exitCode, stderr)
	}

	assertGolden(t, "complexity_go_failure.txt", stdout)
}

func TestComplexityPassesWhenEveryFunctionIsAllowed(t *testing.T) {
	dir := complexityWorkdir(t, "  allow:\n    - key: 'sample.go#Branchy'\n      reason: 'burnt down next'\n")

	exitCode, stdout, _ := runCLI(t, dir, "complexity", "--go", ".")

	if exitCode != 0 {
		t.Fatalf("exit = %d, want 0\n%s", exitCode, stdout)
	}

	assertGolden(t, "complexity_go_allowed.txt", stdout)
}

func TestComplexityFailsOnAnAllowEntryThatMatchesNoFunction(t *testing.T) {
	dir := complexityWorkdir(t, "  allow:\n    - key: 'sample.go#Branchy'\n      reason: 'burnt down next'\n    - key: 'sample.go#Gone'\n      reason: 'renamed away'\n")

	exitCode, stdout, _ := runCLI(t, dir, "complexity", "--go", ".")

	if exitCode != 1 {
		t.Fatalf("exit = %d, want 1", exitCode)
	}

	if !strings.Contains(stdout, "complexity/allow") || !strings.Contains(stdout, "sample.go#Gone") {
		t.Fatalf("stdout did not report the stale entry:\n%s", stdout)
	}
}

func TestComplexityQuietKeepsAPassingRunSilent(t *testing.T) {
	dir := complexityWorkdir(t, "  allow:\n    - key: 'sample.go#Branchy'\n      reason: 'burnt down next'\n")

	exitCode, stdout, _ := runCLI(t, dir, "complexity", "--go", "--quiet", ".")

	if exitCode != 0 || stdout != "" {
		t.Fatalf("exit = %d, stdout = %q", exitCode, stdout)
	}
}

func TestComplexityQuietStillReportsAFailingRun(t *testing.T) {
	exitCode, stdout, _ := runCLI(t, complexityWorkdir(t, ""), "complexity", "--go", "--quiet", ".")

	if exitCode != 1 || !strings.Contains(stdout, "complexity/cyclomatic") {
		t.Fatalf("exit = %d, stdout = %q", exitCode, stdout)
	}
}

func TestComplexityHonorsAnExplicitConfigPath(t *testing.T) {
	dir := complexityWorkdir(t, "")
	elsewhere := t.TempDir()

	if err := os.WriteFile(filepath.Join(elsewhere, "limits.yml"), []byte("complexity:\n  cyclomatic: 50\n  cognitive: 50\n"), 0o600); err != nil {
		t.Fatalf("write config: %v", err)
	}

	exitCode, _, _ := runCLI(t, dir, "complexity", "--go", "--config", filepath.Join(elsewhere, "limits.yml"), ".")

	if exitCode != 0 {
		t.Fatalf("exit = %d, want 0", exitCode)
	}
}

func TestComplexityRendersTheJSONShape(t *testing.T) {
	exitCode, stdout, _ := runCLI(t, complexityWorkdir(t, ""), "complexity", "--go", "--format", "json", ".")

	if exitCode != 1 {
		t.Fatalf("exit = %d, want 1", exitCode)
	}

	assertGolden(t, "complexity_go_failure.json", stdout)
}

func TestComplexityRejectsAnUnknownFlag(t *testing.T) {
	exitCode, _, stderr := runCLI(t, t.TempDir(), "complexity", "--bogus")

	if exitCode != 2 || !strings.Contains(stderr, "unknown flag") {
		t.Fatalf("exit = %d, stderr = %q", exitCode, stderr)
	}
}

func TestComplexityRejectsAnUnknownFormat(t *testing.T) {
	exitCode, _, stderr := runCLI(t, t.TempDir(), "complexity", "--format", "yaml")

	if exitCode != 2 || !strings.Contains(stderr, "unsupported output format") {
		t.Fatalf("exit = %d, stderr = %q", exitCode, stderr)
	}
}

// The TS lane reaches the sidecar's complexity mode with the root and the
// listing it needs, and the numbers it reports come back as findings.
func TestComplexityDrivesTheSidecarTypeScriptLane(t *testing.T) {
	dir := gitWorkdir(t)
	supportDir, logFile := stubSupportDir(t)

	t.Setenv("FMTKIT_SUPPORT_DIR", supportDir)

	if err := os.WriteFile(filepath.Join(dir, "config.yml"), []byte("complexity:\n  cyclomatic: 3\n  cognitive: 3\n"), 0o600); err != nil {
		t.Fatalf("write config: %v", err)
	}

	exitCode, stdout, stderr := runCLI(t, dir, "complexity", "--ts", ".")

	if exitCode != 1 {
		t.Fatalf("exit = %d, want 1 (stderr: %s)", exitCode, stderr)
	}

	if !strings.Contains(stdout, "app.ts#stubbed scores 9 (limit 3)") {
		t.Fatalf("stdout did not carry the sidecar's numbers:\n%s", stdout)
	}

	log, err := os.ReadFile(logFile)

	if err != nil {
		t.Fatalf("read invocation log: %v", err)
	}

	if !strings.Contains(string(log), "complexity --root ") || !strings.Contains(string(log), "--files-from ") {
		t.Fatalf("sidecar argv = %q", log)
	}
}

// `fmtkit check` is the Go gate, so it carries the complexity report and fails
// on it; `fmtkit go format` rewrites files and leaves the judgement to the gate.
func TestCheckCarriesTheComplexityReport(t *testing.T) {
	dir := complexityWorkdir(t, "")

	exitCode, stdout, stderr := runCLI(t, dir, "check", ".")

	if exitCode != 1 {
		t.Fatalf("exit = %d, want 1 (stderr: %s)", exitCode, stderr)
	}

	if !strings.Contains(stdout, "sample.go#Branchy scores 5 (limit 3)") {
		t.Fatalf("check did not report complexity:\n%s", stdout)
	}
}

func TestGoFormatLeavesComplexityToTheGate(t *testing.T) {
	dir := complexityWorkdir(t, "")

	exitCode, stdout, _ := runCLI(t, dir, "go", "format", ".")

	if exitCode != 0 {
		t.Fatalf("exit = %d, want 0", exitCode)
	}

	if strings.Contains(stdout, "Complexity") {
		t.Fatalf("format rendered a complexity section:\n%s", stdout)
	}
}

// `fmtkit lint` is the TS gate, so it scores complexity after oxlint. A clean
// lint with a breaching function still fails, and the finding is printed.
func TestLintFoldsInTheComplexityCheck(t *testing.T) {
	dir := gitWorkdir(t)
	supportDir, _ := stubSupportDir(t)

	t.Setenv("FMTKIT_SUPPORT_DIR", supportDir)

	if err := os.WriteFile(filepath.Join(dir, "config.yml"), []byte("complexity:\n  cyclomatic: 3\n  cognitive: 3\n"), 0o600); err != nil {
		t.Fatalf("write config: %v", err)
	}

	exitCode, stdout, stderr := runCLI(t, dir, "lint", ".")

	if exitCode != 1 {
		t.Fatalf("exit = %d, want 1 (stderr: %s)", exitCode, stderr)
	}

	if !strings.Contains(stdout, "app.ts#stubbed scores 9 (limit 3)") {
		t.Fatalf("lint did not report complexity:\n%s", stdout)
	}
}
