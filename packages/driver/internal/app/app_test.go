package app

import (
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"

	"github.com/oullin/fmtkit/packages/driver/testutil"
)

// TestMain pins a color-free environment: CI task runners export FORCE_COLOR,
// which would inject ANSI codes into the captured output these tests assert.
func TestMain(m *testing.M) {
	_ = os.Unsetenv("FORCE_COLOR")
	_ = os.Setenv("NO_COLOR", "1")

	os.Exit(m.Run())
}

func runCLI(t *testing.T, workdir string, args ...string) (int, string, string) {
	t.Helper()

	oldwd, err := os.Getwd()

	if err != nil {
		t.Fatalf("getwd: %v", err)
	}

	if err := os.Chdir(workdir); err != nil {
		t.Fatalf("chdir: %v", err)
	}

	defer func() {
		_ = os.Chdir(oldwd)
	}()

	var stdout strings.Builder

	var stderr strings.Builder

	// "dev" mirrors the unstamped binary: no embedded TS assets.
	exitCode := New("dev", &stdout, &stderr).Run(args)

	return exitCode, stdout.String(), stderr.String()
}

// stubSupportDir builds a FMTKIT_SUPPORT_DIR whose sidecar logs its argv and
// prints canned tool output, the Go analog of the entrypoint test stubs.
func stubSupportDir(t *testing.T) (string, string) {
	t.Helper()

	dir := t.TempDir()
	logFile := filepath.Join(dir, "invocations.log")

	script := "#!/usr/bin/env bash\n" +
		"set -euo pipefail\n" +
		"printf '%s\\n' \"$*\" >> \"" + logFile + "\"\n" +
		"case \"${1:-}\" in\n" +
		"pipeline)\n" +
		"\tprintf '[blank-lines] processed 3 file(s) in /work, 0 changed\\n'\n" +
		"\tprintf 'Finished in 10ms on 3 files using 8 threads.\\n'\n" +
		"\tprintf '[fluent-chains] processed 3 file(s) in /work, 1 changed\\n'\n" +
		"\t;;\n" +
		"oxlint)\n" +
		"\tprintf 'Found 0 warnings and 0 errors.\\n'\n" +
		"\t;;\n" +
		"esac\n"

	if err := os.WriteFile(filepath.Join(dir, "fmtkit-ts-sidecar"), []byte(script), 0o755); err != nil {
		t.Fatalf("write stub sidecar: %v", err)
	}

	return dir, logFile
}

func gitWorkdir(t *testing.T) string {
	t.Helper()

	dir := t.TempDir()

	cmd := exec.Command("git", "init", "--quiet")
	cmd.Dir = dir

	if out, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("git init: %v\n%s", err, out)
	}

	if err := os.WriteFile(filepath.Join(dir, "app.ts"), []byte("export const a = 1;\n"), 0o644); err != nil {
		t.Fatalf("write app.ts: %v", err)
	}

	testutil.WriteGoFile(t, filepath.Join(dir, "sample.go"), `package sample

func run() {
	defer println("done")
	return
}
`)

	return dir
}

func TestRunWithoutArgsPrintsUsage(t *testing.T) {
	exitCode, _, stderr := runCLI(t, t.TempDir())

	if exitCode != 2 {
		t.Fatalf("expected exit code 2, got %d", exitCode)
	}

	if !strings.Contains(stderr, "usage: fmtkit <format|format-all|go|ts|lint|check|version|help> [args...]") {
		t.Fatalf("unexpected usage output:\n%s", stderr)
	}
}

func TestRunUnknownSubcommandFails(t *testing.T) {
	exitCode, _, stderr := runCLI(t, t.TempDir(), "unknown")

	if exitCode != 2 {
		t.Fatalf("expected exit code 2, got %d", exitCode)
	}

	if !strings.Contains(stderr, "unknown subcommand") {
		t.Fatalf("unexpected stderr:\n%s", stderr)
	}
}

func TestRunVersion(t *testing.T) {
	exitCode, stdout, _ := runCLI(t, t.TempDir(), "version")

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d", exitCode)
	}

	if !strings.Contains(stdout, "fmtkit dev") {
		t.Fatalf("unexpected stdout:\n%s", stdout)
	}
}

func TestFormatRunsFullPipeline(t *testing.T) {
	support, logFile := stubSupportDir(t)

	t.Setenv("FMTKIT_SUPPORT_DIR", support)

	workdir := gitWorkdir(t)

	exitCode, _, stderr := runCLI(t, workdir, "format", ".")

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d\n%s", exitCode, stderr)
	}

	log, err := os.ReadFile(logFile)

	if err != nil {
		t.Fatalf("read invocation log: %v", err)
	}

	invocations := strings.Split(strings.TrimSpace(string(log)), "\n")

	if len(invocations) != 2 {
		t.Fatalf("expected 2 sidecar invocations, got %q", invocations)
	}

	if !strings.HasPrefix(invocations[0], "pipeline ") {
		t.Fatalf("first invocation = %q, want pipeline", invocations[0])
	}

	if !strings.HasPrefix(invocations[1], "oxlint ") {
		t.Fatalf("second invocation = %q, want oxlint", invocations[1])
	}

	for _, needle := range []string{
		"==> Formatting target(s)",
		"==> Running TS/Vue formatting",
		"blank-lines  processed 3 file(s) in /work, 0 changed",
		"==> Running TS/Vue lint",
		"oxlint       Found 0 warnings and 0 errors.",
		"==> Running Go formatting",
		"==> Formatting complete",
	} {
		if !strings.Contains(stderr, needle) {
			t.Fatalf("stderr missing %q:\n%s", needle, stderr)
		}
	}
}

func TestFormatTSFlagSkipsGoStep(t *testing.T) {
	support, logFile := stubSupportDir(t)

	t.Setenv("FMTKIT_SUPPORT_DIR", support)

	workdir := gitWorkdir(t)

	exitCode, _, stderr := runCLI(t, workdir, "format", "--ts", ".")

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d\n%s", exitCode, stderr)
	}

	if strings.Contains(stderr, "==> Running Go formatting") {
		t.Fatalf("--ts still ran the Go step:\n%s", stderr)
	}

	log, err := os.ReadFile(logFile)

	if err != nil {
		t.Fatalf("read invocation log: %v", err)
	}

	if invocations := strings.Split(strings.TrimSpace(string(log)), "\n"); len(invocations) != 2 {
		t.Fatalf("expected pipeline + oxlint invocations, got %q", invocations)
	}
}

func TestFormatGoFlagSkipsTSSteps(t *testing.T) {
	workdir := gitWorkdir(t)

	// No FMTKIT_SUPPORT_DIR: the TS toolchain is unavailable, so --go must
	// succeed without touching it.
	exitCode, _, stderr := runCLI(t, workdir, "format", "--go", ".")

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d\n%s", exitCode, stderr)
	}

	if strings.Contains(stderr, "==> Running TS/Vue formatting") {
		t.Fatalf("--go still ran the TS step:\n%s", stderr)
	}

	if !strings.Contains(stderr, "==> Running Go formatting") {
		t.Fatalf("--go did not run the Go step:\n%s", stderr)
	}
}

func TestFormatBothFlagsRunEverything(t *testing.T) {
	support, _ := stubSupportDir(t)

	t.Setenv("FMTKIT_SUPPORT_DIR", support)

	workdir := gitWorkdir(t)

	exitCode, _, stderr := runCLI(t, workdir, "format", "--ts", "--go", ".")

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d\n%s", exitCode, stderr)
	}

	for _, needle := range []string{"==> Running TS/Vue formatting", "==> Running TS/Vue lint", "==> Running Go formatting"} {
		if !strings.Contains(stderr, needle) {
			t.Fatalf("stderr missing %q:\n%s", needle, stderr)
		}
	}
}

func TestFormatRejectsUnknownFlag(t *testing.T) {
	exitCode, _, stderr := runCLI(t, t.TempDir(), "format", "--nope")

	if exitCode != 2 {
		t.Fatalf("expected exit code 2, got %d", exitCode)
	}

	if !strings.Contains(stderr, "unknown flag") {
		t.Fatalf("unexpected stderr:\n%s", stderr)
	}
}

func TestFormatAllRejectsExtraArgs(t *testing.T) {
	exitCode, _, stderr := runCLI(t, t.TempDir(), "format-all", "extra")

	if exitCode != 2 {
		t.Fatalf("expected exit code 2, got %d", exitCode)
	}

	if !strings.Contains(stderr, "usage: fmtkit") {
		t.Fatalf("unexpected stderr:\n%s", stderr)
	}
}

func TestTSRunsSidecarOnly(t *testing.T) {
	support, logFile := stubSupportDir(t)

	t.Setenv("FMTKIT_SUPPORT_DIR", support)

	workdir := gitWorkdir(t)

	exitCode, stdout, stderr := runCLI(t, workdir, "ts", ".")

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d\n%s", exitCode, stderr)
	}

	if strings.Contains(stderr, "==> Running Go formatting") {
		t.Fatalf("ts mode ran the pipeline:\n%s", stderr)
	}

	if !strings.Contains(stdout, "[blank-lines] processed") {
		t.Fatalf("expected raw sidecar output on stdout:\n%s", stdout)
	}

	log, err := os.ReadFile(logFile)

	if err != nil {
		t.Fatalf("read invocation log: %v", err)
	}

	if got := strings.TrimSpace(string(log)); !strings.HasPrefix(got, "pipeline ") || strings.Contains(got, "\n") {
		t.Fatalf("expected a single pipeline invocation, got %q", got)
	}
}

func TestGoDelegatesToFormatterCLI(t *testing.T) {
	workdir := gitWorkdir(t)

	exitCode, stdout, stderr := runCLI(t, workdir, "go", "format", ".")

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d\nstdout:%s\nstderr:%s", exitCode, stdout, stderr)
	}

	if !strings.Contains(stdout, "Formatter") {
		t.Fatalf("expected formatter output:\n%s", stdout)
	}
}

func TestCheckDelegatesToFormatterCLI(t *testing.T) {
	dir := t.TempDir()

	testutil.WriteGoFile(t, filepath.Join(dir, "sample.go"), `package sample

func run() {
	if true {
		println("ok")
	}
	println("next")
}
`)

	exitCode, stdout, _ := runCLI(t, dir, "check", dir)

	if exitCode != 1 {
		t.Fatalf("expected exit code 1, got %d", exitCode)
	}

	if !strings.Contains(stdout, "Result: fail") {
		t.Fatalf("unexpected stdout:\n%s", stdout)
	}
}

func TestFormatFailsWithoutToolchain(t *testing.T) {
	workdir := gitWorkdir(t)

	// No FMTKIT_SUPPORT_DIR and dev builds carry no embedded assets.
	exitCode, _, stderr := runCLI(t, workdir, "format", ".")

	if exitCode != 1 {
		t.Fatalf("expected exit code 1, got %d\n%s", exitCode, stderr)
	}

	if !strings.Contains(stderr, "!! Running TS/Vue formatting failed") {
		t.Fatalf("stderr missing failure banner:\n%s", stderr)
	}

	if !strings.Contains(stderr, "no TS toolchain") {
		t.Fatalf("stderr missing toolchain guidance:\n%s", stderr)
	}
}
