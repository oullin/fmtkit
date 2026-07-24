package gotool

import (
	"bytes"
	"context"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"

	"go.ollin.sh/fmtkit/driver/internal/gitfiles"
	report "go.ollin.sh/fmtkit/driver/report"
)

const cleanSource = `package sample

func run() {
	println("ok")
}
`

// spacingViolationSource is missing the blank line the spacing rule
// requires between the defer statement and the return.
const spacingViolationSource = `package sample

func run() {
	defer println("done")
	return
}
`

// runInTempModulelessDir writes source to a Go file in a fresh temp dir,
// chdirs there (so vet finds no module and skips), and runs the CLI.
func runInTempModulelessDir(t *testing.T, source string, mode report.Mode, extraArgs ...string) (code int, stdout, stderr string, file string) {
	t.Helper()

	dir := t.TempDir()
	file = filepath.Join(dir, "sample.go")

	if err := os.WriteFile(file, []byte(source), 0o644); err != nil {
		t.Fatalf("write sample: %v", err)
	}

	t.Chdir(dir)

	var out, errOut bytes.Buffer

	code = Runner{Stdout: &out, Stderr: &errOut}.Run(context.Background(), mode, append(extraArgs, file))

	return code, out.String(), errOut.String(), file
}

func TestRunnerRunCleanFileJSON(t *testing.T) {
	code, stdout, stderr, _ := runInTempModulelessDir(t, cleanSource, report.ModeCheck, "--format", "json")

	if code != 0 {
		t.Fatalf("exit = %d, stderr: %s", code, stderr)
	}

	if !strings.Contains(stdout, `"result":"pass"`) {
		t.Fatalf("unexpected json output: %s", stdout)
	}

	if !strings.Contains(stdout, `"status":"skipped"`) {
		t.Fatalf("expected vet skipped in module-less dir: %s", stdout)
	}
}

func TestRunnerRunCheckModeReportsViolation(t *testing.T) {
	code, stdout, _, file := runInTempModulelessDir(t, spacingViolationSource, report.ModeCheck)

	if code != 1 {
		t.Fatalf("exit = %d, stdout: %s", code, stdout)
	}

	content, err := os.ReadFile(file)

	if err != nil {
		t.Fatalf("read sample: %v", err)
	}

	if string(content) != spacingViolationSource {
		t.Fatal("check mode must not rewrite the file")
	}
}

func TestRunnerRunFormatModeRewritesFile(t *testing.T) {
	code, stdout, stderr, file := runInTempModulelessDir(t, spacingViolationSource, report.ModeFormat)

	if code != 0 {
		t.Fatalf("exit = %d, stdout: %s, stderr: %s", code, stdout, stderr)
	}

	content, err := os.ReadFile(file)

	if err != nil {
		t.Fatalf("read sample: %v", err)
	}

	if string(content) == spacingViolationSource {
		t.Fatal("format mode should rewrite the file")
	}

	if !strings.Contains(string(content), "defer println(\"done\")\n\n\treturn") {
		t.Fatalf("expected blank line inserted after defer, got:\n%s", content)
	}
}

func TestRunnerRunRejectsUnsupportedFormat(t *testing.T) {
	code, _, stderr, _ := runInTempModulelessDir(t, cleanSource, report.ModeCheck, "--format", "yaml")

	if code != 1 {
		t.Fatalf("exit = %d", code)
	}

	if !strings.Contains(stderr, "unsupported output format") {
		t.Fatalf("unexpected stderr: %s", stderr)
	}
}

func TestRunnerRunRejectsUnknownFlag(t *testing.T) {
	dir := t.TempDir()

	t.Chdir(dir)

	var out, errOut bytes.Buffer

	if code := (Runner{Stdout: &out, Stderr: &errOut}).Run(context.Background(), report.ModeCheck, []string{"--bogus"}); code != 1 {
		t.Fatalf("exit = %d", code)
	}
}

func TestRunnerReportsConfigLoadError(t *testing.T) {
	dir := t.TempDir()

	if err := os.WriteFile(filepath.Join(dir, "sample.go"), []byte(cleanSource), 0o644); err != nil {
		t.Fatalf("write sample: %v", err)
	}

	t.Chdir(dir)

	var out, errOut bytes.Buffer

	// An explicit --config path that does not exist makes config.Load fail, so
	// the runner reports it on stderr and exits 1 before running the formatter.
	code := Runner{Stdout: &out, Stderr: &errOut}.Run(context.Background(), report.ModeCheck,
		[]string{"--config", filepath.Join(dir, "missing.yml"), "sample.go"})

	if code != 1 {
		t.Fatalf("exit = %d, want 1", code)
	}

	if !strings.Contains(errOut.String(), "load config") {
		t.Fatalf("expected a config-load error on stderr, got: %q", errOut.String())
	}
}

func TestRunnerHonorsReportRootFlag(t *testing.T) {
	work := t.TempDir()
	reportRoot := t.TempDir()

	if err := os.WriteFile(filepath.Join(work, "sample.go"), []byte(cleanSource), 0o644); err != nil {
		t.Fatalf("write sample: %v", err)
	}

	t.Chdir(work)

	var out, errOut bytes.Buffer

	// --cwd points config discovery and report-relative paths at reportRoot while
	// the process stays in work; a clean file still passes.
	code := Runner{Stdout: &out, Stderr: &errOut}.Run(context.Background(), report.ModeCheck,
		[]string{"--cwd", reportRoot, "--format", "json", "sample.go"})

	if code != 0 {
		t.Fatalf("exit = %d, stderr: %s", code, errOut.String())
	}

	if !strings.Contains(out.String(), `"result":"pass"`) {
		t.Fatalf("unexpected output: %s", out.String())
	}
}

// generatedViolationSource carries the same spacing violation as
// spacingViolationSource, but is marked generated so the engine must never
// rewrite it.
const generatedViolationSource = `// Code generated by fixture. DO NOT EDIT.

package sample

func generated() {
	defer println("done")
	return
}
`

// generatedCommittedSource is the formatted form of generatedViolationSource.
// Committing this and then writing the violation makes the generated file a
// working-tree change, so a run that scopes by git alone would pick it up — and
// only the engine's exclusion keeps it out.
const generatedCommittedSource = `// Code generated by fixture. DO NOT EDIT.

package sample

func generated() {
	defer println("done")

	return
}
`

// initScopedRepo builds a git repo holding a committed-but-unformatted file, a
// committed generated file, and an untracked unformatted file, then chdirs into
// it. Only untracked.go is a working-tree change.
func initScopedRepo(t *testing.T) (dir, committed, generated, untracked string) {
	t.Helper()

	dir = t.TempDir()

	gitRun := func(args ...string) {
		t.Helper()

		cmd := exec.Command("git", args...)
		cmd.Dir = dir

		if out, err := cmd.CombinedOutput(); err != nil {
			t.Fatalf("git %v: %v\n%s", args, err, out)
		}
	}

	write := func(name, source string) string {
		t.Helper()

		path := filepath.Join(dir, name)

		if err := os.WriteFile(path, []byte(source), 0o644); err != nil {
			t.Fatalf("write %s: %v", name, err)
		}

		return path
	}

	gitRun("init", "-q")
	gitRun("config", "user.email", "tests@example.com")
	gitRun("config", "user.name", "Test Runner")

	committed = write("committed.go", spacingViolationSource)
	generated = write("generated.go", generatedCommittedSource)

	gitRun("add", "-A")
	gitRun("commit", "-q", "-m", "fixture")

	untracked = write("untracked.go", spacingViolationSource)

	// Now generated.go is modified *and* generated: git reports it as a change,
	// so only cfg's exclusion can keep the formatter off it.
	write("generated.go", generatedViolationSource)

	t.Chdir(dir)

	return dir, committed, generated, untracked
}

func readFile(t *testing.T, path string) string {
	t.Helper()

	content, err := os.ReadFile(path)

	if err != nil {
		t.Fatalf("read %s: %v", path, err)
	}

	return string(content)
}

func TestScopedRunnerFormatsOnlyTheWorkingTreesChanges(t *testing.T) {
	_, committed, generated, untracked := initScopedRepo(t)

	var out, errOut bytes.Buffer

	code := Runner{Stdout: &out, Stderr: &errOut, Scope: gitfiles.SelectionChanged}.Run(context.Background(), report.ModeFormat, nil)

	if code != 0 {
		t.Fatalf("exit code = %d, want 0\n%s\n%s", code, out.String(), errOut.String())
	}

	if got := readFile(t, untracked); got == spacingViolationSource {
		t.Fatalf("untracked.go is a working-tree change and should have been formatted:\n%s", got)
	}

	if got := readFile(t, committed); got != spacingViolationSource {
		t.Fatalf("committed.go is unchanged in the working tree and must be left alone:\n%s", got)
	}

	if got := readFile(t, generated); got != generatedViolationSource {
		t.Fatalf("generated.go must never be rewritten:\n%s", got)
	}
}

func TestUnscopedRunnerFormatsEveryOwnedFile(t *testing.T) {
	_, committed, generated, untracked := initScopedRepo(t)

	var out, errOut bytes.Buffer

	code := Runner{Stdout: &out, Stderr: &errOut}.Run(context.Background(), report.ModeFormat, nil)

	if code != 0 {
		t.Fatalf("exit code = %d, want 0\n%s\n%s", code, out.String(), errOut.String())
	}

	for name, path := range map[string]string{"committed.go": committed, "untracked.go": untracked} {
		if got := readFile(t, path); got == spacingViolationSource {
			t.Fatalf("%s should have been formatted without a selection:\n%s", name, got)
		}
	}

	// The exclusion has to survive whichever path collected the files.
	if got := readFile(t, generated); got != generatedViolationSource {
		t.Fatalf("generated.go must never be rewritten:\n%s", got)
	}
}

func TestScopedRunnerOnACleanTreeFormatsNothing(t *testing.T) {
	_, committed, _, untracked := initScopedRepo(t)

	cmd := exec.Command("git", "add", "-A")
	cmd.Dir = filepath.Dir(committed)

	if out, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("git add: %v\n%s", err, out)
	}

	cmd = exec.Command("git", "commit", "-q", "-m", "everything")
	cmd.Dir = filepath.Dir(committed)

	if out, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("git commit: %v\n%s", err, out)
	}

	var out, errOut bytes.Buffer

	code := Runner{Stdout: &out, Stderr: &errOut, Scope: gitfiles.SelectionChanged}.Run(context.Background(), report.ModeFormat, nil)

	if code != 0 {
		t.Fatalf("exit code = %d, want 0\n%s\n%s", code, out.String(), errOut.String())
	}

	// Nothing diverges from the commit, so the unformatted files stay as they are
	// — which is exactly why a CI gate must use format-all, not format.
	if got := readFile(t, committed); got != spacingViolationSource {
		t.Fatalf("a clean working tree has no changes to format:\n%s", got)
	}

	if got := readFile(t, untracked); got != spacingViolationSource {
		t.Fatalf("a clean working tree has no changes to format:\n%s", got)
	}
}
