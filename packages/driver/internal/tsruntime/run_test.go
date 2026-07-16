package tsruntime

import (
	"bytes"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
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

func supportWithStub(t *testing.T) Support {
	t.Helper()

	dir := t.TempDir()

	writeStub(t, filepath.Join(dir, sidecarName))

	return Support{Dir: dir}
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

	t.Setenv(SourcesCwdEnv, repo)

	var stdout, stderr bytes.Buffer

	err := support.RunPipeline(RunOptions{Stdout: &stdout, Stderr: &stderr})

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

	t.Setenv(SourcesCwdEnv, repo)

	var stdout, stderr bytes.Buffer

	if err := support.RunPipeline(RunOptions{Stdout: &stdout, Stderr: &stderr}); err != nil {
		t.Fatalf("RunPipeline: %v\nstderr: %s", err, stderr.String())
	}

	if strings.Contains(stdout.String(), "--oxfmt-config") {
		t.Fatalf("bundled config passed despite project config:\n%s", stdout.String())
	}
}

func TestRunPipelineReportsMissingScopes(t *testing.T) {
	repo := gitScratchRepo(t, map[string]string{"app.ts": "export const a = 1;\n"})

	support := supportWithStub(t)

	t.Setenv(SourcesCwdEnv, repo)

	var stdout, stderr bytes.Buffer

	err := support.RunPipeline(RunOptions{
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

	t.Setenv(SourcesCwdEnv, repo)

	var stdout, stderr bytes.Buffer

	if err := support.RunLint(RunOptions{Stdout: &stdout, Stderr: &stderr}); err != nil {
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

func TestRunLintSkipsBundledConfigWhenProjectHasOne(t *testing.T) {
	repo := gitScratchRepo(t, map[string]string{
		"app.ts":         "export const a = 1;\n",
		".oxlintrc.json": "{}",
	})

	support := supportWithStub(t)

	if err := os.WriteFile(filepath.Join(support.Dir, ".oxlintrc.json"), []byte("{}"), 0o644); err != nil {
		t.Fatalf("write bundled config: %v", err)
	}

	t.Setenv(SourcesCwdEnv, repo)

	var stdout, stderr bytes.Buffer

	if err := support.RunLint(RunOptions{Stdout: &stdout, Stderr: &stderr}); err != nil {
		t.Fatalf("RunLint: %v\nstderr: %s", err, stderr.String())
	}

	if strings.Contains(stdout.String(), "--config") {
		t.Fatalf("bundled config passed despite project config:\n%s", stdout.String())
	}
}

func TestRunLintSkipsSpawnWithoutFiles(t *testing.T) {
	repo := gitScratchRepo(t, map[string]string{"main.go": "package main\n"})

	support := Support{Dir: t.TempDir()} // no sidecar: spawning would fail

	t.Setenv(SourcesCwdEnv, repo)

	var stdout, stderr bytes.Buffer

	if err := support.RunLint(RunOptions{Stdout: &stdout, Stderr: &stderr}); err != nil {
		t.Fatalf("RunLint: %v", err)
	}

	if !strings.Contains(stdout.String(), "[lint] no TS/Vue files to lint.") {
		t.Fatalf("stdout = %q, want no-files notice", stdout.String())
	}
}

func TestRunLintHonorsOxlintBinOverride(t *testing.T) {
	repo := gitScratchRepo(t, map[string]string{"app.ts": "export const a = 1;\n"})

	support := supportWithStub(t)

	override := filepath.Join(t.TempDir(), "oxlint")

	writeStub(t, override)

	t.Setenv(SourcesCwdEnv, repo)
	t.Setenv(OxlintBinEnv, override)

	var stdout, stderr bytes.Buffer

	if err := support.RunLint(RunOptions{Stdout: &stdout, Stderr: &stderr}); err != nil {
		t.Fatalf("RunLint: %v\nstderr: %s", err, stderr.String())
	}

	if strings.HasPrefix(strings.TrimSpace(stdout.String()), "oxlint") {
		t.Fatalf("mode argument passed to standalone oxlint override:\n%s", stdout.String())
	}
}
