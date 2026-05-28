package testutil

import (
	"os"
	"path/filepath"
	"testing"
)

func WriteFile(t *testing.T, path string, content string) {
	t.Helper()

	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		t.Fatalf("mkdir parent: %v", err)
	}

	if err := os.WriteFile(path, []byte(content), 0o644); err != nil {
		t.Fatalf("write file: %v", err)
	}
}

func WriteGoFile(t *testing.T, path string, content string) {
	t.Helper()
	WriteFile(t, path, content)
}

func WriteGoMod(t *testing.T, dir string, modulePath string) {
	t.Helper()
	WriteFile(t, filepath.Join(dir, "go.mod"), "module "+modulePath+"\n\ngo 1.26.0\n")
}

func WriteGoWork(t *testing.T, dir string, content string) {
	t.Helper()
	WriteFile(t, filepath.Join(dir, "go.work"), content)
}
