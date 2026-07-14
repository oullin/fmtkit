package vet

import (
	"errors"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"

	"github.com/oullin/fmtkit/packages/driver/testutil"
)

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
	report := Run(t.TempDir(), Config{Enabled: false})

	if report.Root != "" || report.ErrorCount() != 0 {
		t.Fatalf("expected empty report, got %#v", report)
	}
}

func TestRunSkipsWhenGoToolchainUnavailable(t *testing.T) {
	restore := stubLookGoPath(t, func() (string, error) {
		return "", exec.ErrNotFound
	})

	defer restore()

	report := Run(t.TempDir(), Default())

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

	restore := stubGoEnvOutput(t, func(string, ...string) ([]byte, error) {
		return []byte(workspaceFile + "\n" + moduleFile + "\n"), nil
	})

	defer restore()

	restoreModules := stubGoListModulesOutput(t, func(string) ([]byte, error) {
		return nil, nil
	})

	defer restoreModules()

	report := Run(workRoot, Default())

	if report.Root != workspaceRoot {
		t.Fatalf("unexpected report: %#v", report)
	}
}

func TestRunFallsBackToModuleWhenWorkspaceUnset(t *testing.T) {
	workRoot := t.TempDir()
	moduleRoot := t.TempDir()
	moduleFile := filepath.Join(moduleRoot, "go.mod")

	testutil.WriteFile(t, moduleFile, "module example.com/test\n")

	restore := stubGoEnvOutput(t, func(string, ...string) ([]byte, error) {
		return []byte("\n" + moduleFile + "\n"), nil
	})

	defer restore()

	restoreModules := stubGoListModulesOutput(t, func(string) ([]byte, error) {
		return nil, nil
	})

	defer restoreModules()

	report := Run(workRoot, Default())

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

	report := Run(workspaceRoot, Default())

	if report.ErrorCount() != 1 {
		t.Fatalf("expected one vet error, got %#v", report)
	}

	if report.Errors[0].File != moduleA {
		t.Fatalf("unexpected error file: %#v", report.Errors[0])
	}

	if !strings.Contains(report.Errors[0].Message, "automatic go vet ./... failed") {
		t.Fatalf("unexpected error message: %#v", report.Errors[0])
	}

	if !strings.Contains(report.Errors[0].Message, "fmt.Printf format %d has arg") {
		t.Fatalf("expected fmt.Printf vet diagnostic, got %#v", report.Errors[0])
	}
}

func TestRunReportsGoEnvLookupError(t *testing.T) {
	restore := stubGoEnvOutput(t, func(string, ...string) ([]byte, error) {
		return nil, &exec.ExitError{Stderr: []byte("go env failed\n")}
	})

	defer restore()

	report := Run(t.TempDir(), Default())

	if report.ErrorCount() != 1 {
		t.Fatalf("expected one error, got %#v", report)
	}

	if !strings.Contains(report.Errors[0].Message, "resolve go GOWORK GOMOD: go env failed") {
		t.Fatalf("unexpected error: %#v", report)
	}
}

func TestRunSkipsOutsideModule(t *testing.T) {
	report := Run(t.TempDir(), Default())

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
	restore := stubGoEnvOutput(t, func(string, ...string) ([]byte, error) {
		return nil, errors.New("boom")
	})

	defer restore()

	_, err := goEnv(t.TempDir(), "GOWORK", "GOMOD")

	if err == nil {
		t.Fatal("expected error")
	}

	if !strings.Contains(err.Error(), "resolve go GOWORK GOMOD: boom") {
		t.Fatalf("unexpected error: %v", err)
	}
}

func stubLookGoPath(t *testing.T, fn func() (string, error)) func() {
	t.Helper()

	previous := lookGoPath
	lookGoPath = fn

	return func() {
		lookGoPath = previous
	}
}

func stubGoEnvOutput(t *testing.T, fn func(string, ...string) ([]byte, error)) func() {
	t.Helper()

	previous := goEnvOutput
	goEnvOutput = fn

	return func() {
		goEnvOutput = previous
	}
}

func stubGoListModulesOutput(t *testing.T, fn func(string) ([]byte, error)) func() {
	t.Helper()

	previous := goListModulesOutput
	goListModulesOutput = fn

	return func() {
		goListModulesOutput = previous
	}
}
