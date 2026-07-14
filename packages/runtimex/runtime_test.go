package runtimex

import (
	"path/filepath"
	"runtime"
	"testing"
)

func TestRuntimeRootPreservesWhitespaceInConfiguredPath(t *testing.T) {
	base := filepath.Join(realTempDir(t), " runtime cache ")
	t.Setenv("GO_FMT_RUNTIME_DIR", base)

	root, err := runtimeRoot(runtimePayload{}, false)

	if err != nil {
		t.Fatalf("resolve runtime root: %v", err)
	}

	want := filepath.Join(base, "runtime", runtimeVersion, runtime.GOOS+"-"+runtime.GOARCH, "unbundled")

	if root != want {
		t.Fatalf("expected %q, got %q", want, root)
	}
}

func TestNormalizeRuntimeInputIgnoresWhitespaceOnlyPath(t *testing.T) {
	input, err := normalizeRuntimeInput(runtimeInput{RuntimeDir: " \t\n "})

	if err != nil {
		t.Fatalf("normalize input: %v", err)
	}

	if input.RuntimeDir != "" {
		t.Fatalf("expected whitespace-only runtime directory to be ignored, got %q", input.RuntimeDir)
	}
}

func TestNormalizeRuntimeInputRejectsRelativePath(t *testing.T) {
	if _, err := normalizeRuntimeInput(runtimeInput{RuntimeDir: "runtime"}); err == nil {
		t.Fatal("expected relative runtime directory rejection")
	}
}
