package sourcefiles

import (
	"bytes"
	"context"
	"path/filepath"
	"strings"
	"testing"
)

func TestRunPrintsNULSeparatedFiles(t *testing.T) {
	dir := initRepo(t)
	writeFile(t, filepath.Join(dir, "src", "app.ts"), "const value = 1;\n")
	writeFile(t, filepath.Join(dir, "src", "notes.md"), "# Notes\n")
	writeFile(t, filepath.Join(dir, "src", "types.d.ts"), "declare const value: string;\n")
	gitAdd(t, dir, ".")

	var stdout, stderr bytes.Buffer

	code := Run(context.Background(), []string{"--cwd", dir, "src"}, &stdout, &stderr)

	if code != 0 {
		t.Fatalf("RunCLI exit = %d, stderr: %s", code, stderr.String())
	}

	got := splitNUL(stdout.String())

	want := []string{
		filepath.Join(dir, "src", "app.ts"),
		filepath.Join(dir, "src", "notes.md"),
	}

	if len(got) != len(want) {
		t.Fatalf("files mismatch\nwant: %#v\n got: %#v", want, got)
	}

	for i := range want {
		if got[i] != want[i] {
			t.Fatalf("files mismatch\nwant: %#v\n got: %#v", want, got)
		}
	}
}

func TestRunIncludesDeclarationsFlag(t *testing.T) {
	dir := initRepo(t)
	writeFile(t, filepath.Join(dir, "types.d.ts"), "declare const value: string;\n")
	gitAdd(t, dir, ".")

	var stdout, stderr bytes.Buffer

	code := Run(context.Background(), []string{"--cwd", dir, "--include-declarations"}, &stdout, &stderr)

	if code != 0 {
		t.Fatalf("RunCLI exit = %d, stderr: %s", code, stderr.String())
	}

	got := splitNUL(stdout.String())

	if len(got) != 1 || got[0] != filepath.Join(dir, "types.d.ts") {
		t.Fatalf("expected the declaration file, got %#v", got)
	}
}

func TestRunWarnsOnMissingScopes(t *testing.T) {
	dir := initRepo(t)
	writeFile(t, filepath.Join(dir, "app.ts"), "const value = 1;\n")
	gitAdd(t, dir, ".")

	var stdout, stderr bytes.Buffer

	code := Run(context.Background(), []string{"--cwd", dir, "missing"}, &stdout, &stderr)

	if code != 0 {
		t.Fatalf("RunCLI exit = %d", code)
	}

	if !strings.Contains(stderr.String(), "[sources] path not found, skipping") {
		t.Fatalf("expected a missing-path warning, got stderr: %q", stderr.String())
	}
}

func TestRunDefaultsToWorkingDirectory(t *testing.T) {
	dir := initRepo(t)
	writeFile(t, filepath.Join(dir, "app.ts"), "const value = 1;\n")
	gitAdd(t, dir, ".")
	t.Chdir(dir)

	var stdout, stderr bytes.Buffer

	if code := Run(context.Background(), nil, &stdout, &stderr); code != 0 {
		t.Fatalf("RunCLI exit = %d, stderr: %s", code, stderr.String())
	}

	got := splitNUL(stdout.String())

	// With no --cwd, RunCLI resolves the process working directory itself.
	if len(got) != 1 || got[0] != filepath.Join(dir, "app.ts") {
		t.Fatalf("expected app.ts under the resolved cwd, got %#v", got)
	}
}

func TestRunReportsBadFlags(t *testing.T) {
	var stdout, stderr bytes.Buffer

	if code := Run(context.Background(), []string{"--nope"}, &stdout, &stderr); code != 1 {
		t.Fatalf("expected exit 1 for an unknown flag, got %d", code)
	}
}

func splitNUL(s string) []string {
	if s == "" {
		return nil
	}

	parts := strings.Split(s, "\x00")
	out := make([]string, 0, len(parts))

	for _, part := range parts {
		if part == "" {
			continue
		}

		out = append(out, part)
	}

	return out
}
