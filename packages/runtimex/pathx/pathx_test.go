package pathx

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

func TestResolvePreservesWhitespaceInPath(t *testing.T) {
	root := filepath.Join(t.TempDir(), " runtime path ")
	t.Setenv("GO_FMT_RUNTIME_DIR", root)

	if resolved := Resolve(); resolved != root {
		t.Fatalf("expected %q, got %q", root, resolved)
	}
}

func TestResolveIgnoresWhitespaceOnlyPath(t *testing.T) {
	t.Setenv("GO_FMT_RUNTIME_DIR", " \t\n ")

	if resolved := Resolve(); resolved != "" {
		t.Fatalf("expected whitespace-only runtime path to be ignored, got %q", resolved)
	}
}
