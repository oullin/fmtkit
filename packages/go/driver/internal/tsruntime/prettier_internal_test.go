package tsruntime

import (
	"context"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestMigrateCommandDefaultsToSidecar(t *testing.T) {
	support := Support{Dir: filepath.Join("some", "dir")}

	bin, args := support.migrateCommand(overrides{})

	if bin != support.Sidecar() {
		t.Fatalf("bin = %q, want sidecar %q", bin, support.Sidecar())
	}

	if len(args) != 1 || args[0] != "oxfmt" {
		t.Fatalf("args = %q, want [oxfmt]", args)
	}
}

func TestMigrateCommandHonorsOxfmtBin(t *testing.T) {
	bin, args := Support{}.migrateCommand(overrides{oxfmtBin: "/usr/bin/oxfmt"})

	if bin != "/usr/bin/oxfmt" || len(args) != 0 {
		t.Fatalf("migrateCommand = (%q, %q), want (/usr/bin/oxfmt, [])", bin, args)
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

func TestPrettierDerivedConfigWarnsWhenCacheWriteFails(t *testing.T) {
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

	env := overrides{oxfmtBin: oxfmt}

	var stderr strings.Builder

	if got := support.prettierDerivedConfig(context.Background(), cwd, env, &stderr); got != "" {
		t.Fatalf("expected empty result on cache failure, got %q", got)
	}

	if !strings.Contains(stderr.String(), "could not cache") {
		t.Fatalf("expected a cache warning, got %q", stderr.String())
	}
}

func TestPrettierDerivedConfigWarnsWhenMigrationWritesNoConfig(t *testing.T) {
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

	var stderr strings.Builder

	got := support.prettierDerivedConfig(context.Background(), cwd, overrides{oxfmtBin: silent}, &stderr)

	if got != "" {
		t.Fatalf("expected empty result when no config is produced, got %q", got)
	}

	if !strings.Contains(stderr.String(), "could not derive") {
		t.Fatalf("expected a derive warning, got %q", stderr.String())
	}
}
