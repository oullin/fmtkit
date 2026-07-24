package runtime

import (
	"context"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"go.ollin.sh/fmtkit/driver/internal/typescript/proto"
)

func TestOxfmtExecutableDefaultsToSidecar(t *testing.T) {
	migration := PrettierMigration{Assets: Assets{Dir: filepath.Join("some", "dir")}}

	bin, viaSidecar := migration.oxfmtExecutable()

	if bin != migration.Assets.Sidecar() {
		t.Fatalf("bin = %q, want sidecar %q", bin, migration.Assets.Sidecar())
	}

	if !viaSidecar {
		t.Fatal("expected the sidecar dispatch path (viaSidecar = true)")
	}
}

func TestOxfmtExecutableHonorsOxfmtBin(t *testing.T) {
	migration := PrettierMigration{Env: proto.Overrides{OxfmtBin: "/usr/bin/oxfmt"}}

	bin, viaSidecar := migration.oxfmtExecutable()

	if bin != "/usr/bin/oxfmt" || viaSidecar {
		t.Fatalf("oxfmtExecutable = (%q, %v), want (/usr/bin/oxfmt, false)", bin, viaSidecar)
	}
}

func TestCopyFileContentsReadError(t *testing.T) {
	err := copyFileContents(filepath.Join(t.TempDir(), "missing"), filepath.Join(t.TempDir(), "dst"))

	if err == nil {
		t.Fatal("expected an error copying a missing source")
	}
}

func TestWriteCacheFileMkdirError(t *testing.T) {
	dir := t.TempDir()

	blocker := filepath.Join(dir, "blocker")

	if err := os.WriteFile(blocker, []byte("x"), 0o644); err != nil {
		t.Fatalf("write blocker: %v", err)
	}

	// blocker is a file, so creating a directory beneath it must fail.
	err := writeCacheFile(filepath.Join(blocker, "sub", "cfg.json"), []byte("{}"))

	if err == nil {
		t.Fatal("expected an error when the cache parent cannot be created")
	}
}

func TestPackageJSONHasPrettierKeyMissingFile(t *testing.T) {
	if packageJSONHasPrettierKey(filepath.Join(t.TempDir(), "package.json")) {
		t.Fatal("a missing package.json cannot carry a prettier key")
	}
}

func TestDerivedConfigWarnsWhenCacheWriteFails(t *testing.T) {
	support := supportWithStub(t)

	oxfmt := filepath.Join(t.TempDir(), "oxfmt")
	writeMigrateStub(t, oxfmt)

	// Occupy the cache directory path with a regular file so MkdirAll fails.
	if err := os.WriteFile(filepath.Join(support.Dir, "prettier-derived"), []byte("x"), 0o644); err != nil {
		t.Fatalf("write cache blocker: %v", err)
	}

	cwd := t.TempDir()

	if err := os.WriteFile(filepath.Join(cwd, ".prettierrc.json"), []byte("{}"), 0o644); err != nil {
		t.Fatalf("write prettier config: %v", err)
	}

	migration := PrettierMigration{Assets: support, Env: proto.Overrides{OxfmtBin: oxfmt}}

	var stderr strings.Builder

	if got := migration.DerivedConfig(context.Background(), cwd, &stderr); got != "" {
		t.Fatalf("expected empty result on cache failure, got %q", got)
	}

	if !strings.Contains(stderr.String(), "could not cache") {
		t.Fatalf("expected a cache warning, got %q", stderr.String())
	}
}

func TestDerivedConfigWarnsWhenMigrationWritesNoConfig(t *testing.T) {
	support := supportWithStub(t)

	// A stub that exits 0 but writes no .oxfmtrc.json: the read-back must fail.
	silent := filepath.Join(t.TempDir(), "oxfmt")

	if err := os.WriteFile(silent, []byte("#!/bin/sh\nexit 0\n"), 0o755); err != nil {
		t.Fatalf("write silent stub: %v", err)
	}

	cwd := t.TempDir()

	if err := os.WriteFile(filepath.Join(cwd, ".prettierrc.json"), []byte("{}"), 0o644); err != nil {
		t.Fatalf("write prettier config: %v", err)
	}

	migration := PrettierMigration{Assets: support, Env: proto.Overrides{OxfmtBin: silent}}

	var stderr strings.Builder

	got := migration.DerivedConfig(context.Background(), cwd, &stderr)

	if got != "" {
		t.Fatalf("expected empty result when no config is produced, got %q", got)
	}

	if !strings.Contains(stderr.String(), "could not derive") {
		t.Fatalf("expected a derive warning, got %q", stderr.String())
	}
}
