package runtimepath

import (
	"path/filepath"
	"testing"
)

func TestResolveAndContains(t *testing.T) {
	root := t.TempDir()
	t.Setenv("GO_FMT_RUNTIME_DIR", root)

	resolved := Resolve()

	if resolved != root {
		t.Fatalf("expected %q, got %q", root, resolved)
	}

	if !Contains(resolved, filepath.Join(root, "bin", "fmt-ts")) {
		t.Fatal("expected nested path to be contained")
	}

	if Contains(resolved, filepath.Dir(root)) {
		t.Fatal("expected parent path not to be contained")
	}
}
