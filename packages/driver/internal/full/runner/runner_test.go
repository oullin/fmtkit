package runner

import (
	"os"
	"os/exec"
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
	writeTextFile(t, filepath.Join(workdir, "sample.ts"), "const value = 1;\n")
	t.Setenv("FORMAT_TS_BIN", formatBin)
	t.Setenv("FORMAT_LINT_BIN", lintBin)
	t.Setenv("GO_FMT_RUNTIME_DIR", runtimeDir)
	chdir(t, workdir)

	var stdout, stderr strings.Builder
	exitCode := New(&stdout, &stderr, "test").Run([]string{"format", "."})

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d\nstdout:\n%s\nstderr:\n%s", exitCode, stdout.String(), stderr.String())
	}

	if invocations := readFile(t, logPath); invocations != "fmt-ts sample.ts\nfmt-lint sample.ts\n" {
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
	workdir := t.TempDir()
	writeTextFile(t, filepath.Join(workdir, "sample.ts"), "const value = 1;\n")
	chdir(t, workdir)

	var stdout, stderr strings.Builder

	if exitCode := New(&stdout, &stderr, "test").Run([]string{"format", "."}); exitCode != 7 {
		t.Fatalf("expected exit code 7, got %d", exitCode)
	}

	if invocations := readFile(t, logPath); invocations != "fmt-ts sample.ts\n" {
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
	workdir := t.TempDir()
	writeTextFile(t, filepath.Join(workdir, "sample.ts"), "const value = 1;\n")
	chdir(t, workdir)

	var stdout, stderr strings.Builder

	if exitCode := New(&stdout, &stderr, "test").Run([]string{"format-all"}); exitCode != 7 {
		t.Fatalf("expected exit code 7, got %d", exitCode)
	}

	if invocations := readFile(t, logPath); invocations != "fmt-ts sample.ts\n" {
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

func TestRunnerSmartCleanRepositoryIsNoOpWithoutRuntimeExtraction(t *testing.T) {
	workdir := initGitRepo(t)
	writeTextFile(t, filepath.Join(workdir, "clean.ts"), "const value = 1;\n")
	runCommand(t, workdir, "git", "add", ".")
	runCommand(t, workdir, "git", "commit", "-qm", "initial")
	t.Setenv("GO_FMT_RUNTIME_DIR", "relative-runtime-would-fail")
	chdir(t, workdir)

	var stdout, stderr strings.Builder

	if exitCode := New(&stdout, &stderr, "test").Run(nil); exitCode != 0 {
		t.Fatalf("expected clean no-op, got %d\nstdout:\n%s\nstderr:\n%s", exitCode, stdout.String(), stderr.String())
	}

	if stdout.Len() != 0 || stderr.Len() != 0 {
		t.Fatalf("expected silent no-op, stdout=%q stderr=%q", stdout.String(), stderr.String())
	}
}

func TestRunnerSmartTSOnlyRunsFormattingAndLint(t *testing.T) {
	workdir := initGitRepo(t)
	writeTextFile(t, filepath.Join(workdir, "app.ts"), "const value = 1;\n")
	runCommand(t, workdir, "git", "add", ".")
	runCommand(t, workdir, "git", "commit", "-qm", "initial")
	writeTextFile(t, filepath.Join(workdir, "app.ts"), "const value = 2;\n")
	logPath := filepath.Join(t.TempDir(), "invocations.log")
	t.Setenv("FORMAT_TS_BIN", writeStubTool(t, "fmt-ts", logPath, 0, ""))
	t.Setenv("FORMAT_LINT_BIN", writeStubTool(t, "fmt-lint", logPath, 0, ""))
	t.Setenv("GO_FMT_RUNTIME_DIR", realTempDir(t))
	chdir(t, workdir)

	var stdout, stderr strings.Builder

	if exitCode := New(&stdout, &stderr, "test").Run(nil); exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d: %s", exitCode, stderr.String())
	}

	if invocations := readFile(t, logPath); invocations != "fmt-ts app.ts\nfmt-lint app.ts\n" {
		t.Fatalf("unexpected invocations: %q", invocations)
	}

	if stdout.Len() != 0 {
		t.Fatalf("Go formatter unexpectedly ran: %s", stdout.String())
	}
}

func TestRunnerSmartGoOnlyDoesNotRunTS(t *testing.T) {
	workdir := initGitRepo(t)
	testutil.WriteGoFile(t, filepath.Join(workdir, "sample.go"), "package sample\n\nconst value = 1\n")
	runCommand(t, workdir, "git", "add", ".")
	runCommand(t, workdir, "git", "commit", "-qm", "initial")
	testutil.WriteGoFile(t, filepath.Join(workdir, "sample.go"), "package sample\n\nconst value = 2\n")
	logPath := filepath.Join(t.TempDir(), "invocations.log")
	t.Setenv("FORMAT_TS_BIN", writeStubTool(t, "fmt-ts", logPath, 0, ""))
	t.Setenv("FORMAT_LINT_BIN", writeStubTool(t, "fmt-lint", logPath, 0, ""))
	t.Setenv("GO_FMT_RUNTIME_DIR", realTempDir(t))
	chdir(t, workdir)

	var stdout, stderr strings.Builder

	if exitCode := New(&stdout, &stderr, "test").Run(nil); exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d\nstdout:\n%s\nstderr:\n%s", exitCode, stdout.String(), stderr.String())
	}

	if _, err := os.Stat(logPath); !os.IsNotExist(err) {
		t.Fatalf("TS tooling unexpectedly ran; stat error=%v", err)
	}

	if !strings.Contains(stdout.String(), "Formatter") {
		t.Fatalf("Go formatter did not run: %s", stdout.String())
	}
}

func TestRunnerSmartMixedChangesRouteExactFilesToBothFamilies(t *testing.T) {
	workdir := initGitRepo(t)
	changedGoPath := filepath.Join(workdir, "changed.go")
	unchangedGoPath := filepath.Join(workdir, "unchanged.go")
	testutil.WriteGoFile(t, changedGoPath, "package sample\n\nfunc changed() {}\n")
	testutil.WriteGoFile(t, unchangedGoPath, "package sample\n\nfunc untouched( ){ }\n")
	writeTextFile(t, filepath.Join(workdir, "changed.ts"), "const value = 1;\n")
	writeTextFile(t, filepath.Join(workdir, "unchanged.ts"), "const untouched = true;\n")
	runCommand(t, workdir, "git", "add", ".")
	runCommand(t, workdir, "git", "commit", "-qm", "initial")
	testutil.WriteGoFile(t, changedGoPath, "package sample\n\nfunc changed( ){ }\n")
	writeTextFile(t, filepath.Join(workdir, "changed.ts"), "const value = 2;\n")
	unchangedGoBefore := readFile(t, unchangedGoPath)
	logPath := filepath.Join(t.TempDir(), "invocations.log")
	t.Setenv("FORMAT_TS_BIN", writeStubTool(t, "fmt-ts", logPath, 0, ""))
	t.Setenv("FORMAT_LINT_BIN", writeStubTool(t, "fmt-lint", logPath, 0, ""))
	t.Setenv("GO_FMT_RUNTIME_DIR", realTempDir(t))
	chdir(t, workdir)

	var stdout, stderr strings.Builder

	if exitCode := New(&stdout, &stderr, "test").Run(nil); exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d\nstdout:\n%s\nstderr:\n%s", exitCode, stdout.String(), stderr.String())
	}

	if invocations := readFile(t, logPath); invocations != "fmt-ts changed.ts\nfmt-lint changed.ts\n" {
		t.Fatalf("TS family did not receive the exact changed list: %q", invocations)
	}

	if changedGoAfter := readFile(t, changedGoPath); !strings.Contains(changedGoAfter, "func changed() {") {
		t.Fatalf("changed Go file was not formatted: %q", changedGoAfter)
	}

	if unchangedGoAfter := readFile(t, unchangedGoPath); unchangedGoAfter != unchangedGoBefore {
		t.Fatalf("unchanged Go file was routed unexpectedly\nbefore: %q\n after: %q", unchangedGoBefore, unchangedGoAfter)
	}

	if !strings.Contains(stderr.String(), "changed.go") || strings.Contains(stderr.String(), "unchanged.go") {
		t.Fatalf("Go target detail did not contain the exact changed list: %s", stderr.String())
	}
}

func TestRunnerSmartUnsupportedOnlyChangeIsNoOpWithoutRuntimeExtraction(t *testing.T) {
	workdir := initGitRepo(t)
	writeTextFile(t, filepath.Join(workdir, "notes.md"), "before\n")
	runCommand(t, workdir, "git", "add", ".")
	runCommand(t, workdir, "git", "commit", "-qm", "initial")
	writeTextFile(t, filepath.Join(workdir, "notes.md"), "after\n")
	t.Setenv("GO_FMT_RUNTIME_DIR", "relative-runtime-would-fail")
	chdir(t, workdir)

	var stdout, stderr strings.Builder

	if exitCode := New(&stdout, &stderr, "test").Run(nil); exitCode != 0 {
		t.Fatalf("expected unsupported-only no-op, got %d\nstdout:\n%s\nstderr:\n%s", exitCode, stdout.String(), stderr.String())
	}

	if stdout.Len() != 0 || stderr.Len() != 0 {
		t.Fatalf("expected silent no-op, stdout=%q stderr=%q", stdout.String(), stderr.String())
	}
}

func TestRunnerLanguageFlagFiltersMixedScope(t *testing.T) {
	workdir := t.TempDir()
	testutil.WriteGoFile(t, filepath.Join(workdir, "sample.go"), "package sample\n")
	writeTextFile(t, filepath.Join(workdir, "sample.ts"), "const value = 1;\n")
	logPath := filepath.Join(t.TempDir(), "invocations.log")
	t.Setenv("FORMAT_TS_BIN", writeStubTool(t, "fmt-ts", logPath, 0, ""))
	t.Setenv("FORMAT_LINT_BIN", writeStubTool(t, "fmt-lint", logPath, 0, ""))
	t.Setenv("GO_FMT_RUNTIME_DIR", realTempDir(t))
	chdir(t, workdir)

	var stdout, stderr strings.Builder

	if exitCode := New(&stdout, &stderr, "test").Run([]string{"--ts", "."}); exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d: %s", exitCode, stderr.String())
	}

	if invocations := readFile(t, logPath); invocations != "fmt-ts sample.ts\nfmt-lint sample.ts\n" {
		t.Fatalf("unexpected invocations: %q", invocations)
	}

	if stdout.Len() != 0 {
		t.Fatalf("Go formatter unexpectedly ran: %s", stdout.String())
	}
}

func TestRunnerGoFlagFiltersMixedExplicitScope(t *testing.T) {
	workdir := t.TempDir()
	goPath := filepath.Join(workdir, "sample.go")
	testutil.WriteGoFile(t, goPath, "package sample\n\nfunc run( ){ }\n")
	writeTextFile(t, filepath.Join(workdir, "sample.ts"), "const value = 1;\n")
	logPath := filepath.Join(t.TempDir(), "invocations.log")
	t.Setenv("FORMAT_TS_BIN", writeStubTool(t, "fmt-ts", logPath, 0, ""))
	t.Setenv("FORMAT_LINT_BIN", writeStubTool(t, "fmt-lint", logPath, 0, ""))
	t.Setenv("GO_FMT_RUNTIME_DIR", realTempDir(t))
	chdir(t, workdir)

	var stdout, stderr strings.Builder

	if exitCode := New(&stdout, &stderr, "test").Run([]string{"--go", "."}); exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d\nstdout:\n%s\nstderr:\n%s", exitCode, stdout.String(), stderr.String())
	}

	if _, err := os.Stat(logPath); !os.IsNotExist(err) {
		t.Fatalf("TS tooling unexpectedly ran; stat error=%v", err)
	}

	if goSource := readFile(t, goPath); !strings.Contains(goSource, "func run() {") {
		t.Fatalf("Go formatter did not receive the explicit scope: %q", goSource)
	}
}

func TestRunnerRejectsConflictingLanguageFlags(t *testing.T) {
	var stdout, stderr strings.Builder

	if exitCode := New(&stdout, &stderr, "test").Run([]string{"format", "--go", "--ts"}); exitCode != 2 {
		t.Fatalf("expected exit code 2, got %d", exitCode)
	}

	if !strings.Contains(stderr.String(), "mutually exclusive") {
		t.Fatalf("missing conflict error: %s", stderr.String())
	}
}

func TestRunnerSeparatorTreatsFlagLikeFilenameAsPath(t *testing.T) {
	workdir := t.TempDir()
	writeTextFile(t, filepath.Join(workdir, "--go.ts"), "const value = 1;\n")
	logPath := filepath.Join(t.TempDir(), "invocations.log")
	t.Setenv("FORMAT_TS_BIN", writeStubTool(t, "fmt-ts", logPath, 0, ""))
	t.Setenv("FORMAT_LINT_BIN", writeStubTool(t, "fmt-lint", logPath, 0, ""))
	t.Setenv("GO_FMT_RUNTIME_DIR", realTempDir(t))
	chdir(t, workdir)

	var stdout, stderr strings.Builder

	if exitCode := New(&stdout, &stderr, "test").Run([]string{"format", "--", "--go.ts"}); exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d: %s", exitCode, stderr.String())
	}

	dashTarget := "." + string(filepath.Separator) + "--go.ts"
	wantInvocations := "fmt-ts " + dashTarget + "\nfmt-lint " + dashTarget + "\n"

	if invocations := readFile(t, logPath); invocations != wantInvocations {
		t.Fatalf("separator path was not forwarded: %q", invocations)
	}
}

func TestRunnerSeparatorFormatsDashLeadingGoFilename(t *testing.T) {
	workdir := t.TempDir()
	goPath := filepath.Join(workdir, "--go.go")
	testutil.WriteGoFile(t, goPath, "package sample\n\nfunc run( ){ }\n")
	t.Setenv("GO_FMT_RUNTIME_DIR", realTempDir(t))
	chdir(t, workdir)

	var stdout, stderr strings.Builder

	if exitCode := New(&stdout, &stderr, "test").Run([]string{"format", "--go", "--", "--go.go"}); exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d\nstdout:\n%s\nstderr:\n%s", exitCode, stdout.String(), stderr.String())
	}

	if goSource := readFile(t, goPath); !strings.Contains(goSource, "func run() {") {
		t.Fatalf("dash-leading Go target was not formatted: %q", goSource)
	}
}

func TestRunnerSymlinkCandidatesCannotMutateExternalTargets(t *testing.T) {
	for _, test := range []struct {
		name    string
		git     bool
		runArgs []string
	}{
		{name: "implicit Git", git: true},
		{name: "explicit files", runArgs: []string{"format", "outside.go", "outside.ts"}},
	} {
		t.Run(test.name, func(t *testing.T) {
			workdir := t.TempDir()

			if test.git {
				workdir = initGitRepo(t)
			}

			external := t.TempDir()
			externalGo := filepath.Join(external, "external.go")
			externalTS := filepath.Join(external, "external.ts")
			goBefore := "package external\n\nfunc run( ){ }\n"
			tsBefore := "const external = true;\n"
			testutil.WriteGoFile(t, externalGo, goBefore)
			writeTextFile(t, externalTS, tsBefore)

			if err := os.Symlink(externalGo, filepath.Join(workdir, "outside.go")); err != nil {
				t.Fatalf("create Go symlink: %v", err)
			}

			if err := os.Symlink(externalTS, filepath.Join(workdir, "outside.ts")); err != nil {
				t.Fatalf("create TS symlink: %v", err)
			}

			logPath := filepath.Join(t.TempDir(), "invocations.log")
			t.Setenv("FORMAT_TS_BIN", writeMutatingStubTool(t, "fmt-ts", logPath))
			t.Setenv("FORMAT_LINT_BIN", writeMutatingStubTool(t, "fmt-lint", logPath))
			t.Setenv("GO_FMT_RUNTIME_DIR", realTempDir(t))
			chdir(t, workdir)

			var stdout, stderr strings.Builder

			if exitCode := New(&stdout, &stderr, "test").Run(test.runArgs); exitCode != 0 {
				t.Fatalf("expected symlink no-op, got %d\nstdout:\n%s\nstderr:\n%s", exitCode, stdout.String(), stderr.String())
			}

			if _, err := os.Stat(logPath); !os.IsNotExist(err) {
				t.Fatalf("TS tooling received a symlink target; stat error=%v", err)
			}

			if after := readFile(t, externalGo); after != goBefore {
				t.Fatalf("external Go target was mutated\nbefore: %q\n after: %q", goBefore, after)
			}

			if after := readFile(t, externalTS); after != tsBefore {
				t.Fatalf("external TS target was mutated\nbefore: %q\n after: %q", tsBefore, after)
			}
		})
	}
}

func TestRunnerTopLevelVersionUsesFmtkitName(t *testing.T) {
	var stdout, stderr strings.Builder

	if exitCode := New(&stdout, &stderr, "test").Run([]string{"version"}); exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d", exitCode)
	}

	if stdout.String() != "fmtkit test\n" || stderr.Len() != 0 {
		t.Fatalf("unexpected streams: stdout=%q stderr=%q", stdout.String(), stderr.String())
	}
}

func TestRunnerLegacyCheckAndHelpCommandsRemainAvailable(t *testing.T) {
	workdir := t.TempDir()
	t.Setenv("GO_FMT_RUNTIME_DIR", realTempDir(t))
	chdir(t, workdir)

	var checkStdout, checkStderr strings.Builder

	if exitCode := New(&checkStdout, &checkStderr, "test").Run([]string{"check"}); exitCode != 0 {
		t.Fatalf("legacy check failed with %d\nstdout:\n%s\nstderr:\n%s", exitCode, checkStdout.String(), checkStderr.String())
	}

	if !strings.Contains(checkStdout.String(), "Formatter") {
		t.Fatalf("legacy check did not reach Go CLI: %s", checkStdout.String())
	}

	var helpStdout, helpStderr strings.Builder

	if exitCode := New(&helpStdout, &helpStderr, "test").Run([]string{"help"}); exitCode != 0 {
		t.Fatalf("legacy help failed with %d", exitCode)
	}

	if helpStdout.Len() != 0 || !strings.Contains(helpStderr.String(), "usage: fmtkit") {
		t.Fatalf("unexpected help streams: stdout=%q stderr=%q", helpStdout.String(), helpStderr.String())
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

func writeMutatingStubTool(t *testing.T, name, logPath string) string {
	t.Helper()

	path := filepath.Join(t.TempDir(), name)
	content := "#!/usr/bin/env sh\n" +
		"printf '%s\\n' '" + name + "' >> '" + logPath + "'\n" +
		"for target in \"$@\"; do printf '%s\\n' mutated > \"$target\"; done\n"

	if err := os.WriteFile(path, []byte(content), 0o755); err != nil {
		t.Fatalf("write mutating stub: %v", err)
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

func writeTextFile(t *testing.T, path, content string) {
	t.Helper()

	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		t.Fatalf("create parent: %v", err)
	}

	if err := os.WriteFile(path, []byte(content), 0o644); err != nil {
		t.Fatalf("write %s: %v", path, err)
	}
}

func initGitRepo(t *testing.T) string {
	t.Helper()

	dir := t.TempDir()
	runCommand(t, dir, "git", "init", "-q")
	runCommand(t, dir, "git", "config", "user.email", "tests@example.com")
	runCommand(t, dir, "git", "config", "user.name", "Test Runner")

	return dir
}

func runCommand(t *testing.T, dir, name string, args ...string) {
	t.Helper()

	cmd := exec.Command(name, args...)
	cmd.Dir = dir

	if output, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("%s %v: %v\n%s", name, args, err, output)
	}
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
