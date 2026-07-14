package engine

import (
	"os"
	"path/filepath"
	"testing"

	"github.com/oullin/fmtkit/packages/driver/testutil"
	"github.com/oullin/fmtkit/packages/formatter/config"
	"github.com/oullin/fmtkit/packages/runtimepath"
)

func TestFilterFiles(t *testing.T) {
	files := []string{"a.go", "b.go", "c.go"}

	if got := filterFiles(nil, []string{"a.go"}); got != nil {
		t.Fatalf("expected nil for empty files, got %#v", got)
	}

	if got := filterFiles(files, nil); got != nil {
		t.Fatalf("expected nil for empty selected, got %#v", got)
	}

	got := filterFiles(files, []string{"c.go", "a.go"})
	want := []string{"a.go", "c.go"}

	if len(got) != len(want) {
		t.Fatalf("expected %#v, got %#v", want, got)
	}

	for i := range want {
		if got[i] != want[i] {
			t.Fatalf("expected %#v, got %#v", want, got)
		}
	}
}

func TestEffectiveConcurrencyBounds(t *testing.T) {
	tests := []struct {
		name       string
		configured int
		fileCount  int
		want       int
	}{
		{name: "defaults at least one when no files", configured: 0, fileCount: 0, want: 1},
		{name: "negative defaults and clamps to files", configured: -1, fileCount: 2, want: 2},
		{name: "configured clamps to files", configured: 8, fileCount: 3, want: 3},
		{name: "configured one stays serial", configured: 1, fileCount: 3, want: 1},
	}

	for _, tt := range tests {
		tt := tt

		t.Run(tt.name, func(t *testing.T) {
			if got := effectiveConcurrency(tt.configured, tt.fileCount); got != tt.want {
				t.Fatalf("expected %d, got %d", tt.want, got)
			}
		})
	}
}

func TestIsGoSourceHandlesReadErrorsAsSource(t *testing.T) {
	path := filepath.Join(t.TempDir(), "sample.go")

	if !isGoSource(path) {
		t.Fatal("expected unreadable missing .go path to be treated as Go source")
	}
}

func TestShouldSkipDirHonorsRootHiddenDirectory(t *testing.T) {
	cfg := config.Default()
	root := filepath.Join(t.TempDir(), ".root")
	runtimeDir := runtimepath.Resolve()

	if shouldSkipDir(root, root, ".root", cfg, runtimeDir) {
		t.Fatal("expected root directory to be walkable even when hidden")
	}

	if !shouldSkipDir(filepath.Join(root, ".hidden"), root, ".hidden", cfg, runtimeDir) {
		t.Fatal("expected nested hidden directory to be skipped")
	}
}

func TestCollectGoFilesSkipsRuntimeDirectory(t *testing.T) {
	root := t.TempDir()
	runtimeDir := filepath.Join(root, "runtime")
	testutil.WriteGoFile(t, filepath.Join(root, "sample.go"), "package sample\n")
	testutil.WriteGoFile(t, filepath.Join(runtimeDir, "go", "src", "ignored.go"), "package ignored\n")
	t.Setenv("GO_FMT_RUNTIME_DIR", runtimeDir)

	files, err := CollectGoFiles([]string{root}, config.Default())

	if err != nil {
		t.Fatalf("collect: %v", err)
	}

	want := []string{filepath.Join(root, "sample.go")}

	if len(files) != len(want) || files[0] != want[0] {
		t.Fatalf("files mismatch\nwant: %#v\n got: %#v", want, files)
	}
}

func TestCollectGoFilesSkipsExplicitRuntimeFile(t *testing.T) {
	root := t.TempDir()
	runtimeDir := filepath.Join(root, "runtime")
	file := filepath.Join(runtimeDir, "go", "src", "ignored.go")
	testutil.WriteGoFile(t, file, "package ignored\n")
	t.Setenv("GO_FMT_RUNTIME_DIR", runtimeDir)

	files, err := CollectGoFiles([]string{file}, config.Default())

	if err != nil {
		t.Fatalf("collect: %v", err)
	}

	if len(files) != 0 {
		t.Fatalf("expected runtime file exclusion, got %#v", files)
	}
}

func TestCollectGoFilesReturnsWalkErrors(t *testing.T) {
	if os.Geteuid() == 0 {
		t.Skip("root bypasses file permissions, so the blocked directory stays readable")
	}

	root := t.TempDir()
	blocked := filepath.Join(root, "blocked")
	testutil.WriteGoFile(t, filepath.Join(blocked, "sample.go"), "package sample\n")

	if err := os.Chmod(blocked, 0o000); err != nil {
		t.Fatalf("chmod blocked: %v", err)
	}

	t.Cleanup(func() {
		_ = os.Chmod(blocked, 0o755)
	})

	_, err := CollectGoFiles([]string{root}, config.Default())

	if err == nil {
		t.Fatal("expected walk error")
	}
}
