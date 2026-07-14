package runner

import (
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"testing"

	"github.com/oullin/fmtkit/packages/driver/testutil"
)

func TestRunnerFormatRunsFullPipeline(t *testing.T) {
	workdir := t.TempDir()
	runtimeDir := realTempDir(t)
	logPath := filepath.Join(t.TempDir(), "invocations.log")
	formatBin := writeStubTool(t, "fmt-ts", logPath, 0, "[blank-lines] processed 1 file(s) in /work, 0 changed\n")
	lintBin := writeStubTool(t, "fmt-lint", logPath, 0, "Found 0 warnings and 0 errors.\n")
	testutil.WriteGoFile(t, filepath.Join(workdir, "sample.go"), "package sample\n\nfunc run() {\n\tprintln(\"ok\")\n}\n")
	t.Setenv("FORMAT_TS_BIN", formatBin)
	t.Setenv("FORMAT_LINT_BIN", lintBin)
	t.Setenv("GO_FMT_RUNTIME_DIR", runtimeDir)
	chdir(t, workdir)

	var stdout, stderr strings.Builder
	exitCode := New(&stdout, &stderr, "test").Run([]string{"format", "."})

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d\nstdout:\n%s\nstderr:\n%s", exitCode, stdout.String(), stderr.String())
	}

	if invocations := readFile(t, logPath); invocations != "fmt-ts .\nfmt-lint .\n" {
		t.Fatalf("unexpected invocations:\n%s", invocations)
	}

	if !strings.Contains(stderr.String(), "==> Formatting target(s)") || !strings.Contains(stderr.String(), "==> Running TS/Vue formatting") {
		t.Fatalf("missing streamed sections:\n%s", stderr.String())
	}

	if !strings.Contains(stdout.String(), "Formatter") || !strings.Contains(stdout.String(), "Vet") {
		t.Fatalf("expected Go report on stdout, got:\n%s", stdout.String())
	}
}

func TestRunnerFormatStopsOnFailingTool(t *testing.T) {
	logPath := filepath.Join(t.TempDir(), "invocations.log")
	formatBin := writeStubTool(t, "fmt-ts", logPath, 7, "format failed\n")
	lintBin := writeStubTool(t, "fmt-lint", logPath, 0, "should not run\n")
	t.Setenv("FORMAT_TS_BIN", formatBin)
	t.Setenv("FORMAT_LINT_BIN", lintBin)
	t.Setenv("GO_FMT_RUNTIME_DIR", realTempDir(t))
	chdir(t, t.TempDir())

	var stdout, stderr strings.Builder

	if exitCode := New(&stdout, &stderr, "test").Run([]string{"format", "."}); exitCode != 7 {
		t.Fatalf("expected exit code 7, got %d", exitCode)
	}

	if invocations := readFile(t, logPath); invocations != "fmt-ts .\n" {
		t.Fatalf("unexpected invocations:\n%s", invocations)
	}

	if !strings.Contains(stderr.String(), "format failed") || !strings.Contains(stderr.String(), "Running TS/Vue formatting failed") {
		t.Fatalf("unexpected stderr:\n%s", stderr.String())
	}
}

func TestRunnerGoPassthroughUsesInProcessGoCLI(t *testing.T) {
	var stdout, stderr strings.Builder

	if exitCode := New(&stdout, &stderr, "test").Run([]string{"go", "version"}); exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d", exitCode)
	}

	if stdout.String() != "go-fmt test\n" || stderr.String() != "" {
		t.Fatalf("unexpected streams: stdout=%q stderr=%q", stdout.String(), stderr.String())
	}
}

func TestRunnerGoFormatRunsVetWhenEnabled(t *testing.T) {
	workdir := realTempDir(t)

	if err := os.WriteFile(filepath.Join(workdir, "go.mod"), []byte("module example.com/vetcheck\n\ngo 1.26.4\n"), 0o600); err != nil {
		t.Fatalf("write go.mod: %v", err)
	}

	testutil.WriteGoFile(t, filepath.Join(workdir, "main.go"), "package vetcheck\n\nimport \"fmt\"\n\nfunc broken() {\n\tfmt.Printf(\"%d\", \"not an integer\")\n}\n")
	t.Setenv("GO_FMT_RUNTIME_DIR", realTempDir(t))
	chdir(t, workdir)

	var stdout, stderr strings.Builder

	if exitCode := New(&stdout, &stderr, "test").Run([]string{"go", "format", "."}); exitCode == 0 {
		t.Fatalf("expected vet-enabled go format to fail\nstdout:\n%s\nstderr:\n%s", stdout.String(), stderr.String())
	}
}

func TestRunnerFormatAllUsesCurrentDirectory(t *testing.T) {
	logPath := filepath.Join(t.TempDir(), "invocations.log")
	formatBin := writeStubTool(t, "fmt-ts", logPath, 7, "")
	t.Setenv("FORMAT_TS_BIN", formatBin)
	t.Setenv("GO_FMT_RUNTIME_DIR", realTempDir(t))
	chdir(t, t.TempDir())

	var stdout, stderr strings.Builder

	if exitCode := New(&stdout, &stderr, "test").Run([]string{"format-all"}); exitCode != 7 {
		t.Fatalf("expected exit code 7, got %d", exitCode)
	}

	if invocations := readFile(t, logPath); invocations != "fmt-ts .\n" {
		t.Fatalf("format-all did not forward the current-directory target: %q", invocations)
	}
}

func TestRunnerRejectsWhitespaceOnlyCommandAndPath(t *testing.T) {
	for _, args := range [][]string{{"   "}, {"format", "\t "}, {"ts", "\n"}} {
		t.Run(strings.Join(args, "/"), func(t *testing.T) {
			var stdout, stderr strings.Builder

			if exitCode := New(&stdout, &stderr, "test").Run(args); exitCode != 2 {
				t.Fatalf("expected exit code 2, got %d", exitCode)
			}
		})
	}
}

func TestRunnerTrimsCommandButForwardsSpaceContainingPath(t *testing.T) {
	logPath := filepath.Join(t.TempDir(), "invocations.log")
	formatBin := writeStubTool(t, "fmt-ts", logPath, 0, "")
	t.Setenv("FORMAT_TS_BIN", formatBin)
	t.Setenv("GO_FMT_RUNTIME_DIR", realTempDir(t))
	chdir(t, t.TempDir())

	var stdout, stderr strings.Builder

	if exitCode := New(&stdout, &stderr, "test").Run([]string{" ts ", "a path/with spaces"}); exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d: %s", exitCode, stderr.String())
	}

	if invocations := readFile(t, logPath); invocations != "fmt-ts a path/with spaces\n" {
		t.Fatalf("path was not forwarded unchanged: %q", invocations)
	}
}

func writeStubTool(t *testing.T, name, logPath string, exitCode int, output string) string {
	t.Helper()

	path := filepath.Join(t.TempDir(), name)
	content := "#!/usr/bin/env sh\n" + "printf '%s %s\\n' '" + name + "' \"$*\" >> '" + logPath + "'\n" + "printf '%s' '" + strings.ReplaceAll(output, "'", "'\\''") + "'\n" + "exit " + strconv.Itoa(exitCode) + "\n"

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

	t.Cleanup(func() { _ = os.Chdir(oldwd) })
}

func realTempDir(t *testing.T) string {
	t.Helper()

	path, err := filepath.EvalSymlinks(t.TempDir())

	if err != nil {
		t.Fatalf("resolve temp dir: %v", err)
	}

	if err := os.Chmod(path, 0o700); err != nil {
		t.Fatalf("secure temp dir: %v", err)
	}

	return path
}
