package full

import (
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"testing"

	"github.com/oullin/go-fmt/packages/driver/testutil"
)

func TestRunnerFormatRunsFullPipeline(t *testing.T) {
	workdir := t.TempDir()
	runtimeDir := t.TempDir()
	logPath := filepath.Join(t.TempDir(), "invocations.log")
	formatBin := writeStubTool(t, "fmt-ts", logPath, 0, "[blank-lines] processed 1 file(s) in /work, 0 changed\n")
	lintBin := writeStubTool(t, "fmt-lint", logPath, 0, "Found 0 warnings and 0 errors.\n")

	testutil.WriteGoFile(t, filepath.Join(workdir, "sample.go"), `package sample

func run() {
	println("ok")
}
`)
	t.Setenv("FORMAT_TS_BIN", formatBin)
	t.Setenv("FORMAT_LINT_BIN", lintBin)
	t.Setenv("GO_FMT_RUNTIME_DIR", runtimeDir)
	chdir(t, workdir)

	var stdout strings.Builder

	var stderr strings.Builder

	exitCode := NewRunner(&stdout, &stderr, "test").Run([]string{"format", "."})

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d\nstdout:\n%s\nstderr:\n%s", exitCode, stdout.String(), stderr.String())
	}

	invocations := readFile(t, logPath)

	if invocations != "fmt-ts .\nfmt-lint .\n" {
		t.Fatalf("unexpected invocations:\n%s", invocations)
	}

	if !strings.Contains(stderr.String(), "==> Formatting target(s)") || !strings.Contains(stderr.String(), "==> Running TS/Vue formatting") {
		t.Fatalf("missing streamed sections:\n%s", stderr.String())
	}

	if !strings.Contains(stdout.String(), "Formatter") || !strings.Contains(stdout.String(), "Vet") {
		t.Fatalf("expected Go report on stdout, got:\n%s", stdout.String())
	}

	shim := readFile(t, filepath.Join(runtimeDir, "bin", "fmt-sources"))

	if !strings.Contains(shim, " go sources ") {
		t.Fatalf("expected fmt-sources shim to call go sources, got:\n%s", shim)
	}
}

func TestRunnerFormatStopsOnFailingTool(t *testing.T) {
	workdir := t.TempDir()
	logPath := filepath.Join(t.TempDir(), "invocations.log")
	formatBin := writeStubTool(t, "fmt-ts", logPath, 7, "format failed\n")
	lintBin := writeStubTool(t, "fmt-lint", logPath, 0, "should not run\n")

	t.Setenv("FORMAT_TS_BIN", formatBin)
	t.Setenv("FORMAT_LINT_BIN", lintBin)
	t.Setenv("GO_FMT_RUNTIME_DIR", t.TempDir())
	chdir(t, workdir)

	var stdout strings.Builder

	var stderr strings.Builder

	exitCode := NewRunner(&stdout, &stderr, "test").Run([]string{"format", "."})

	if exitCode != 7 {
		t.Fatalf("expected exit code 7, got %d", exitCode)
	}

	invocations := readFile(t, logPath)

	if invocations != "fmt-ts .\n" {
		t.Fatalf("unexpected invocations:\n%s", invocations)
	}

	if !strings.Contains(stderr.String(), "format failed") || !strings.Contains(stderr.String(), "Running TS/Vue formatting failed") {
		t.Fatalf("unexpected stderr:\n%s", stderr.String())
	}
}

func TestRunnerGoPassthroughUsesInProcessGoCLI(t *testing.T) {
	workdir := t.TempDir()
	chdir(t, workdir)

	var stdout strings.Builder

	var stderr strings.Builder

	exitCode := NewRunner(&stdout, &stderr, "test").Run([]string{"go", "version"})

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d", exitCode)
	}

	if stdout.String() != "go-fmt test\n" {
		t.Fatalf("unexpected stdout: %q", stdout.String())
	}

	if stderr.String() != "" {
		t.Fatalf("unexpected stderr: %q", stderr.String())
	}
}

func writeStubTool(t *testing.T, name string, logPath string, exitCode int, output string) string {
	t.Helper()

	path := filepath.Join(t.TempDir(), name)
	content := "#!/usr/bin/env sh\n" +
		"printf '%s %s\\n' '" + name + "' \"$*\" >> '" + logPath + "'\n" +
		"printf '%s' '" + strings.ReplaceAll(output, "'", "'\\''") + "'\n" +
		"exit " + strconv.Itoa(exitCode) + "\n"

	if err := os.WriteFile(path, []byte(content), 0o755); err != nil {
		t.Fatalf("write stub: %v", err)
	}

	return path
}

func readFile(t *testing.T, path string) string {
	t.Helper()

	content, err := os.ReadFile(path)

	if err != nil {
		t.Fatalf("read %s: %v", path, err)
	}

	return string(content)
}

func chdir(t *testing.T, path string) {
	t.Helper()

	oldwd, err := os.Getwd()

	if err != nil {
		t.Fatalf("getwd: %v", err)
	}

	if err := os.Chdir(path); err != nil {
		t.Fatalf("chdir: %v", err)
	}

	t.Cleanup(func() {
		_ = os.Chdir(oldwd)
	})
}
