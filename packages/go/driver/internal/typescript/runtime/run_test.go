package runtime

import (
	"bytes"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"

	"go.ollin.sh/fmtkit/driver/internal/typescript/proto"
)

// writeStub creates an executable that echoes its argv, one per line, so
// tests can assert the exact tool invocation.
func writeStub(t *testing.T, path string) {
	t.Helper()

	script := "#!/bin/sh\nfor arg in \"$@\"; do printf '%s\\n' \"$arg\"; done\n"

	if err := os.WriteFile(path, []byte(script), 0o755); err != nil {
		t.Fatalf("write stub %s: %v", path, err)
	}
}

func gitScratchRepo(t *testing.T, files map[string]string) string {
	t.Helper()

	dir := t.TempDir()

	run := func(args ...string) {
		cmd := exec.Command("git", args...)
		cmd.Dir = dir

		if out, err := cmd.CombinedOutput(); err != nil {
			t.Fatalf("git %v: %v\n%s", args, err, out)
		}
	}

	run("init", "--quiet")

	for name, contents := range files {
		if err := os.WriteFile(filepath.Join(dir, name), []byte(contents), 0o644); err != nil {
			t.Fatalf("write %s: %v", name, err)
		}
	}

	return dir
}

func supportWithStub(t *testing.T) Assets {
	t.Helper()

	dir := t.TempDir()

	writeStub(t, filepath.Join(dir, proto.SidecarName))

	return Assets{Dir: dir}
}

func TestRunPipelineInvokesSidecar(t *testing.T) {
	repo := gitScratchRepo(t, map[string]string{
		"app.ts":   "export const a = 1;\n",
		"types.ts": "export type T = number;\n",
		"decl.d.ts": "declare const d: number;\n" +
			"export default d;\n",
	})

	support := supportWithStub(t)

	if err := os.WriteFile(filepath.Join(support.Dir, ".oxfmtrc.json"), []byte("{}"), 0o644); err != nil {
		t.Fatalf("write bundled config: %v", err)
	}

	t.Setenv(proto.SourcesCwdEnv, repo)

	var stdout, stderr bytes.Buffer

	err := NewInvoker(support).RunPipeline(t.Context(), Request{Stdout: &stdout, Stderr: &stderr})

	if err != nil {
		t.Fatalf("RunPipeline: %v\nstderr: %s", err, stderr.String())
	}

	lines := strings.Split(strings.TrimSpace(stdout.String()), "\n")

	wantPrefix := []string{
		"pipeline",
		"--oxfmt-bin", support.Sidecar(),
		"--oxfmt-config", filepath.Join(support.Dir, ".oxfmtrc.json"),
		"--format-files",
		filepath.Join(repo, "app.ts"),
		filepath.Join(repo, "types.ts"),
		"--syntax-files",
		filepath.Join(repo, "app.ts"),
		filepath.Join(repo, "decl.d.ts"),
		filepath.Join(repo, "types.ts"),
	}

	if len(lines) != len(wantPrefix) {
		t.Fatalf("argv = %q, want %q", lines, wantPrefix)
	}

	for i, want := range wantPrefix {
		if lines[i] != want {
			t.Fatalf("argv[%d] = %q, want %q\nfull: %q", i, lines[i], want, lines)
		}
	}
}

func TestRunPipelineSkipsBundledConfigWhenProjectHasOne(t *testing.T) {
	repo := gitScratchRepo(t, map[string]string{
		"app.ts":        "export const a = 1;\n",
		".oxfmtrc.json": "{}",
	})

	support := supportWithStub(t)

	if err := os.WriteFile(filepath.Join(support.Dir, ".oxfmtrc.json"), []byte("{}"), 0o644); err != nil {
		t.Fatalf("write bundled config: %v", err)
	}

	t.Setenv(proto.SourcesCwdEnv, repo)

	var stdout, stderr bytes.Buffer

	if err := NewInvoker(support).RunPipeline(t.Context(), Request{Stdout: &stdout, Stderr: &stderr}); err != nil {
		t.Fatalf("RunPipeline: %v\nstderr: %s", err, stderr.String())
	}

	if strings.Contains(stdout.String(), "--oxfmt-config") {
		t.Fatalf("bundled config passed despite project config:\n%s", stdout.String())
	}
}

func TestRunPipelineReportsMissingScopes(t *testing.T) {
	repo := gitScratchRepo(t, map[string]string{"app.ts": "export const a = 1;\n"})

	support := supportWithStub(t)

	t.Setenv(proto.SourcesCwdEnv, repo)

	var stdout, stderr bytes.Buffer

	err := NewInvoker(support).RunPipeline(t.Context(), Request{
		Scopes: []string{"missing-dir"},
		Stdout: &stdout,
		Stderr: &stderr,
	})

	if err != nil {
		t.Fatalf("RunPipeline: %v", err)
	}

	want := fmt.Sprintf("[sources] path not found, skipping: %s", filepath.Join(repo, "missing-dir"))

	if !strings.Contains(stderr.String(), want) {
		t.Fatalf("stderr = %q, want it to contain %q", stderr.String(), want)
	}
}

func TestRunLintInvokesOxlintMode(t *testing.T) {
	repo := gitScratchRepo(t, map[string]string{"app.ts": "export const a = 1;\n"})

	support := supportWithStub(t)

	if err := os.WriteFile(filepath.Join(support.Dir, ".oxlintrc.json"), []byte("{}"), 0o644); err != nil {
		t.Fatalf("write bundled config: %v", err)
	}

	t.Setenv(proto.SourcesCwdEnv, repo)

	var stdout, stderr bytes.Buffer

	if err := NewInvoker(support).RunLint(t.Context(), Request{Stdout: &stdout, Stderr: &stderr}); err != nil {
		t.Fatalf("RunLint: %v\nstderr: %s", err, stderr.String())
	}

	want := []string{
		"oxlint",
		"--config", filepath.Join(support.Dir, ".oxlintrc.json"),
		filepath.Join(repo, "app.ts"),
	}

	got := strings.Split(strings.TrimSpace(stdout.String()), "\n")

	if len(got) != len(want) {
		t.Fatalf("argv = %q, want %q", got, want)
	}

	for i := range want {
		if got[i] != want[i] {
			t.Fatalf("argv[%d] = %q, want %q", i, got[i], want[i])
		}
	}
}

func TestRunLintFixPassesFixFlag(t *testing.T) {
	repo := gitScratchRepo(t, map[string]string{"app.ts": "export const a = 1;\n"})

	support := supportWithStub(t)

	if err := os.WriteFile(filepath.Join(support.Dir, ".oxlintrc.json"), []byte("{}"), 0o644); err != nil {
		t.Fatalf("write bundled config: %v", err)
	}

	t.Setenv(proto.SourcesCwdEnv, repo)

	var stdout, stderr bytes.Buffer

	if err := NewInvoker(support).RunLint(t.Context(), Request{Fix: true, Stdout: &stdout, Stderr: &stderr}); err != nil {
		t.Fatalf("RunLint: %v\nstderr: %s", err, stderr.String())
	}

	want := []string{
		"oxlint",
		"--fix",
		"--config", filepath.Join(support.Dir, ".oxlintrc.json"),
		filepath.Join(repo, "app.ts"),
	}

	got := strings.Split(strings.TrimSpace(stdout.String()), "\n")

	if len(got) != len(want) {
		t.Fatalf("argv = %q, want %q", got, want)
	}

	for i := range want {
		if got[i] != want[i] {
			t.Fatalf("argv[%d] = %q, want %q", i, got[i], want[i])
		}
	}
}

func TestRunLintComposesBundledConfigWhenProjectHasOne(t *testing.T) {
	repo := gitScratchRepo(t, map[string]string{
		"app.ts":         "export const a = 1;\n",
		".oxlintrc.json": "{}",
	})

	support := supportWithStub(t)

	if err := os.WriteFile(filepath.Join(support.Dir, ".oxlintrc.json"), []byte("{}"), 0o644); err != nil {
		t.Fatalf("write bundled config: %v", err)
	}

	t.Setenv(proto.SourcesCwdEnv, repo)

	var stdout, stderr bytes.Buffer

	if err := NewInvoker(support).RunLint(t.Context(), Request{Stdout: &stdout, Stderr: &stderr}); err != nil {
		t.Fatalf("RunLint: %v\nstderr: %s", err, stderr.String())
	}

	args := strings.Split(strings.TrimSpace(stdout.String()), "\n")

	if len(args) != 4 || args[0] != "oxlint" || args[1] != "--config" || args[3] != filepath.Join(repo, "app.ts") {
		t.Fatalf("argv = %q, want oxlint with composed config and app.ts", args)
	}

	if filepath.Dir(args[2]) != repo || !strings.HasPrefix(filepath.Base(args[2]), ".fmtkit-oxlint-") {
		t.Fatalf("config = %q, want a managed sibling of the project config", args[2])
	}

	if _, err := os.Stat(args[2]); !os.IsNotExist(err) {
		t.Fatalf("composed config remains after lint: %v", err)
	}
}

func TestRunLintSkipsSpawnWithoutFiles(t *testing.T) {
	repo := gitScratchRepo(t, map[string]string{"main.go": "package main\n"})

	support := Assets{Dir: t.TempDir()} // no sidecar: spawning would fail

	t.Setenv(proto.SourcesCwdEnv, repo)

	var stdout, stderr bytes.Buffer

	if err := NewInvoker(support).RunLint(t.Context(), Request{Stdout: &stdout, Stderr: &stderr}); err != nil {
		t.Fatalf("RunLint: %v", err)
	}

	if !strings.Contains(stdout.String(), "[lint] no TS/Vue files to lint.") {
		t.Fatalf("stdout = %q, want no-files notice", stdout.String())
	}
}

func TestRunLintSkipsSpawnForFormatOnlyDocuments(t *testing.T) {
	// HTML and Markdown are formattable but not lintable, so a scope holding only
	// those must reach the no-files notice rather than handing oxlint files it
	// cannot lint.
	repo := gitScratchRepo(t, map[string]string{
		"index.html": "<script>const value = 1;</script>\n",
		"notes.md":   "# Notes\n",
	})

	support := Assets{Dir: t.TempDir()} // no sidecar: spawning would fail

	t.Setenv(proto.SourcesCwdEnv, repo)

	var stdout, stderr bytes.Buffer

	if err := NewInvoker(support).RunLint(t.Context(), Request{Stdout: &stdout, Stderr: &stderr}); err != nil {
		t.Fatalf("RunLint: %v", err)
	}

	if !strings.Contains(stdout.String(), "[lint] no TS/Vue files to lint.") {
		t.Fatalf("stdout = %q, want no-files notice", stdout.String())
	}
}

func TestRunLintHonorsOxlintBinOverride(t *testing.T) {
	repo := gitScratchRepo(t, map[string]string{"app.ts": "export const a = 1;\n"})

	support := supportWithStub(t)

	if err := os.WriteFile(filepath.Join(support.Dir, ".oxlintrc.json"), []byte("{}"), 0o644); err != nil {
		t.Fatalf("write bundled config: %v", err)
	}

	override := filepath.Join(t.TempDir(), "oxlint")

	writeStub(t, override)

	t.Setenv(proto.SourcesCwdEnv, repo)
	t.Setenv(proto.OxlintBinEnv, override)

	var stdout, stderr bytes.Buffer

	if err := NewInvoker(support).RunLint(t.Context(), Request{Stdout: &stdout, Stderr: &stderr}); err != nil {
		t.Fatalf("RunLint: %v\nstderr: %s", err, stderr.String())
	}

	if strings.HasPrefix(strings.TrimSpace(stdout.String()), "oxlint") {
		t.Fatalf("mode argument passed to standalone oxlint override:\n%s", stdout.String())
	}
}

func TestRunLintRunsEveryConfigBatchAfterViolations(t *testing.T) {
	repo := gitScratchRepo(t, map[string]string{
		"app.ts":         "export const root = 1;\n",
		".oxlintrc.json": "{}",
	})
	nested := filepath.Join(repo, "package")

	if err := os.MkdirAll(nested, 0o755); err != nil {
		t.Fatalf("create nested package: %v", err)
	}

	if err := os.WriteFile(filepath.Join(nested, ".oxlintrc.json"), []byte("{}"), 0o644); err != nil {
		t.Fatalf("write nested config: %v", err)
	}

	if err := os.WriteFile(filepath.Join(nested, "client.ts"), []byte("export const nested = 1;\n"), 0o644); err != nil {
		t.Fatalf("write nested source: %v", err)
	}

	support := supportWithStub(t)
	logPath := filepath.Join(support.Dir, "lint-runs")
	script := "#!/bin/sh\nprintf 'run\\n' >> \"" + logPath + "\"\nexit 1\n"

	if err := os.WriteFile(support.Sidecar(), []byte(script), 0o755); err != nil {
		t.Fatalf("write failing sidecar: %v", err)
	}

	if err := os.WriteFile(filepath.Join(support.Dir, ".oxlintrc.json"), []byte("{}"), 0o644); err != nil {
		t.Fatalf("write bundled config: %v", err)
	}

	t.Setenv(proto.SourcesCwdEnv, repo)

	err := NewInvoker(support).RunLint(t.Context(), Request{Stdout: io.Discard, Stderr: io.Discard})

	if err == nil {
		t.Fatal("RunLint returned nil after batch violations")
	}

	log, readErr := os.ReadFile(logPath)

	if readErr != nil {
		t.Fatalf("read lint runs: %v", readErr)
	}

	if got := strings.Count(string(log), "run\n"); got != 2 {
		t.Fatalf("lint runs = %d, want 2", got)
	}
}
