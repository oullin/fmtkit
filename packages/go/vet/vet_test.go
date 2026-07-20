package vet

import (
	"context"
	"errors"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"

	"go.ollin.sh/fmtkit/driver/testutil"
)

// fakeToolchain drives run with canned toolchain responses. Nil fields fall
// back to a successful no-op, so each test only stubs what it exercises.
type fakeToolchain struct {
	lookGo      func() (string, error)
	envOutput   func(dir string, keys ...string) ([]byte, error)
	listModules func(dir string) ([]byte, error)
	vetOutput   func(dir string) ([]byte, error)
}

func (f fakeToolchain) LookGo() (string, error) {
	if f.lookGo != nil {
		return f.lookGo()
	}

	return "go", nil
}

func (f fakeToolchain) EnvOutput(_ context.Context, dir string, keys ...string) ([]byte, error) {
	if f.envOutput != nil {
		return f.envOutput(dir, keys...)
	}

	return nil, nil
}

func (f fakeToolchain) ListModulesOutput(_ context.Context, dir string) ([]byte, error) {
	if f.listModules != nil {
		return f.listModules(dir)
	}

	return nil, nil
}

func (f fakeToolchain) VetOutput(_ context.Context, dir string) ([]byte, error) {
	if f.vetOutput != nil {
		return f.vetOutput(dir)
	}

	return nil, nil
}

func TestParseGoEnvValuesPreservesOrderAndEmptyLines(t *testing.T) {
	t.Run("leading empty line falls back to gomod", func(t *testing.T) {
		values := parseGoEnvValues([]byte("\n/work/module/go.mod\n"), 2)

		if len(values) != 2 {
			t.Fatalf("unexpected values length: %d", len(values))
		}

		if values[0] != "" || values[1] != "/work/module/go.mod" {
			t.Fatalf("unexpected values: %#v", values)
		}
	})

	t.Run("crlf output trims carriage returns", func(t *testing.T) {
		values := parseGoEnvValues([]byte("C:\\work\\go.work\r\nC:\\work\\go.mod\r\n"), 2)

		if len(values) != 2 {
			t.Fatalf("unexpected values length: %d", len(values))
		}

		if strings.Contains(values[0], "\r") || strings.Contains(values[1], "\r") {
			t.Fatalf("unexpected carriage returns in values: %#v", values)
		}
	})
}

func TestDefaultEnablesVet(t *testing.T) {
	if !Default().Enabled {
		t.Fatal("expected default vet config to be enabled")
	}
}

func TestRunSkipsWhenDisabled(t *testing.T) {
	report := Run(context.Background(), t.TempDir(), Config{Enabled: false})

	if report.Root != "" || report.ErrorCount() != 0 {
		t.Fatalf("expected empty report, got %#v", report)
	}
}

func TestRunSkipsWhenGoToolchainUnavailable(t *testing.T) {
	tc := fakeToolchain{
		lookGo: func() (string, error) {
			return "", exec.ErrNotFound
		},
	}

	report := run(context.Background(), t.TempDir(), Default(), tc)

	if !report.Skipped {
		t.Fatalf("expected skipped report, got %#v", report)
	}

	if report.Root != "" || report.ErrorCount() != 0 {
		t.Fatalf("expected empty skipped report, got %#v", report)
	}
}

func TestRunPrefersWorkspace(t *testing.T) {
	workRoot := t.TempDir()
	workspaceRoot := t.TempDir()
	moduleRoot := t.TempDir()
	workspaceFile := filepath.Join(workspaceRoot, "go.work")
	moduleFile := filepath.Join(moduleRoot, "go.mod")

	testutil.WriteFile(t, workspaceFile, "go 1.26.4\n")
	testutil.WriteFile(t, moduleFile, "module example.com/test\n")

	tc := fakeToolchain{
		envOutput: func(string, ...string) ([]byte, error) {
			return []byte(workspaceFile + "\n" + moduleFile + "\n"), nil
		},
	}

	report := run(context.Background(), workRoot, Default(), tc)

	if report.Root != workspaceRoot {
		t.Fatalf("unexpected report: %#v", report)
	}
}

func TestRunFallsBackToModuleWhenWorkspaceUnset(t *testing.T) {
	workRoot := t.TempDir()
	moduleRoot := t.TempDir()
	moduleFile := filepath.Join(moduleRoot, "go.mod")

	testutil.WriteFile(t, moduleFile, "module example.com/test\n")

	tc := fakeToolchain{
		envOutput: func(string, ...string) ([]byte, error) {
			return []byte("\n" + moduleFile + "\n"), nil
		},
	}

	report := run(context.Background(), workRoot, Default(), tc)

	if report.Root != moduleRoot {
		t.Fatalf("unexpected report: %#v", report)
	}
}

func TestRunRunsGoVetAcrossWorkspaceModules(t *testing.T) {
	workspaceRoot := t.TempDir()
	moduleA := filepath.Join(workspaceRoot, "module-a")
	moduleB := filepath.Join(workspaceRoot, "module-b")

	testutil.WriteGoMod(t, moduleA, "example.com/module-a")
	testutil.WriteGoMod(t, moduleB, "example.com/module-b")
	testutil.WriteGoFile(t, filepath.Join(moduleA, "sample.go"), `package sample

import "fmt"

func run() {
	fmt.Printf("%d", "not-a-number")
}
`)
	testutil.WriteGoFile(t, filepath.Join(moduleB, "sample.go"), `package sample

func run() {
	println("ok")
}
`)
	testutil.WriteGoWork(t, workspaceRoot, `go 1.26.4

use (
	./module-a
	./module-b
)
`)

	report := Run(context.Background(), workspaceRoot, Default())

	if report.ErrorCount() != 1 {
		t.Fatalf("expected one vet error, got %#v", report)
	}

	if report.Errors[0].File != moduleA {
		t.Fatalf("unexpected error file: %#v", report.Errors[0])
	}

	if !strings.Contains(report.Errors[0].Message, "automatic go vet ./... failed") {
		t.Fatalf("unexpected error message: %#v", report.Errors[0])
	}
}

func TestRunReportsGoEnvLookupError(t *testing.T) {
	tc := fakeToolchain{
		envOutput: func(string, ...string) ([]byte, error) {
			return nil, &exec.ExitError{Stderr: []byte("go env failed\n")}
		},
	}

	report := run(context.Background(), t.TempDir(), Default(), tc)

	if report.ErrorCount() != 1 {
		t.Fatalf("expected one error, got %#v", report)
	}

	if !strings.Contains(report.Errors[0].Message, "resolve go GOWORK GOMOD: go env failed") {
		t.Fatalf("unexpected error: %#v", report)
	}
}

func TestRunSkipsOutsideModule(t *testing.T) {
	report := Run(context.Background(), t.TempDir(), Default())

	if report.ErrorCount() != 0 {
		t.Fatalf("expected empty report: %#v", report)
	}
}

func TestExistingGoRootFiltersInvalidCandidates(t *testing.T) {
	tempDir := t.TempDir()
	validRoot := t.TempDir()
	validPath := filepath.Join(validRoot, "go.mod")
	dirCandidate := filepath.Join(tempDir, "go.mod")
	wrongName := filepath.Join(tempDir, "other.file")

	testutil.WriteFile(t, validPath, "module example.com/test\n")

	if err := os.Mkdir(dirCandidate, 0o755); err != nil {
		t.Fatalf("mkdir candidate: %v", err)
	}

	testutil.WriteFile(t, wrongName, "module example.com/test\n")

	cases := []struct {
		name     string
		path     string
		filename string
		wantOK   bool
		wantRoot string
	}{
		{name: "empty", path: "", filename: "go.mod"},
		{name: "off", path: "off", filename: "go.mod"},
		{name: "devnull", path: os.DevNull, filename: "go.mod"},
		{name: "missing", path: filepath.Join(tempDir, "missing", "go.mod"), filename: "go.mod"},
		{name: "directory", path: dirCandidate, filename: "go.mod"},
		{name: "wrong basename", path: wrongName, filename: "go.mod"},
		{name: "valid", path: validPath, filename: "go.mod", wantOK: true, wantRoot: validRoot},
	}

	for _, tc := range cases {
		tc := tc
		t.Run(tc.name, func(t *testing.T) {
			root, ok := existingGoRoot(tc.path, tc.filename)

			if ok != tc.wantOK {
				t.Fatalf("unexpected ok: got %v want %v", ok, tc.wantOK)
			}

			if root != tc.wantRoot {
				t.Fatalf("unexpected root: got %q want %q", root, tc.wantRoot)
			}
		})
	}
}

func TestGoEnvWrapsGenericErrors(t *testing.T) {
	tc := fakeToolchain{
		envOutput: func(string, ...string) ([]byte, error) {
			return nil, errors.New("boom")
		},
	}

	_, err := goEnv(context.Background(), t.TempDir(), tc, "GOWORK", "GOMOD")

	if err == nil {
		t.Fatal("expected error")
	}

	if !strings.Contains(err.Error(), "resolve go GOWORK GOMOD: boom") {
		t.Fatalf("unexpected error: %v", err)
	}
}
