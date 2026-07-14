package runtimex

import (
	"os"
	"path/filepath"
	"testing"
)

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

func readFile(t *testing.T, path string) string {
	t.Helper()

	content, err := os.ReadFile(path)

	if err != nil {
		t.Fatalf("read %s: %v", path, err)
	}

	return string(content)
}
