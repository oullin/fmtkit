package tsruntime

import (
	"os"
	"path/filepath"
	"testing"
	"testing/fstest"

	"go.ollin.sh/fmtkit/driver/internal/sidecarproto"
)

// fakeAssets mirrors a directory staged by stage-ts-assets.sh: the bindings
// and the sidecar, plus the configs that ride along with them.
func fakeAssets() fstest.MapFS {
	return fstest.MapFS{
		sidecarproto.SidecarName: &fstest.MapFile{Data: []byte("#!/bin/sh\n"), Mode: 0o755},
		"oxc-parser.node":        &fstest.MapFile{Data: []byte("parser")},
		"oxfmt.node":             &fstest.MapFile{Data: []byte("fmt")},
		"oxlint.node":            &fstest.MapFile{Data: []byte("lint")},
		".oxfmtrc.json":          &fstest.MapFile{Data: []byte("{}")},
		".oxlintrc.json":         &fstest.MapFile{Data: []byte("{}")},
	}
}

func TestExtractOncePopulatesSupportDir(t *testing.T) {
	dir := filepath.Join(t.TempDir(), "v1.0.0")

	if err := extractOnce(dir, fakeAssets()); err != nil {
		t.Fatalf("extractOnce: %v", err)
	}

	support := Assets{Dir: dir}

	if _, err := os.Stat(support.Sidecar()); err != nil {
		t.Fatalf("sidecar missing: %v", err)
	}

	info, err := os.Stat(support.Sidecar())

	if err != nil {
		t.Fatalf("stat sidecar: %v", err)
	}

	if info.Mode().Perm()&0o111 == 0 {
		t.Fatalf("sidecar is not executable: %v", info.Mode())
	}

	for _, name := range []string{"oxc-parser.node", "oxfmt.node", "oxlint.node", ".oxfmtrc.json", ".oxlintrc.json", sentinelName} {
		if _, err := os.Stat(filepath.Join(dir, name)); err != nil {
			t.Fatalf("%s missing: %v", name, err)
		}
	}

	if support.OxfmtConfig() == "" {
		t.Fatal("expected bundled oxfmt config")
	}

	if support.OxlintConfig() == "" {
		t.Fatal("expected bundled oxlint config")
	}
}

func TestExtractOnceIsIdempotent(t *testing.T) {
	dir := filepath.Join(t.TempDir(), "v1.0.0")

	if err := extractOnce(dir, fakeAssets()); err != nil {
		t.Fatalf("first extractOnce: %v", err)
	}

	marker := filepath.Join(dir, "marker")

	if err := os.WriteFile(marker, []byte("keep"), 0o644); err != nil {
		t.Fatalf("write marker: %v", err)
	}

	if err := extractOnce(dir, fakeAssets()); err != nil {
		t.Fatalf("second extractOnce: %v", err)
	}

	if _, err := os.Stat(marker); err != nil {
		t.Fatalf("completed extraction was redone: %v", err)
	}
}

func TestExtractOnceLosingRaceKeepsWinner(t *testing.T) {
	dir := filepath.Join(t.TempDir(), "v1.0.0")

	// Simulate another process having completed extraction between the
	// sentinel check and the rename: pre-create the final dir with a sentinel.
	if err := extractOnce(dir, fakeAssets()); err != nil {
		t.Fatalf("winner extractOnce: %v", err)
	}

	if err := extractOnce(dir, fakeAssets()); err != nil {
		t.Fatalf("loser extractOnce: %v", err)
	}
}

func TestResolvePrefersSupportDirEnv(t *testing.T) {
	dir := t.TempDir()

	if err := os.WriteFile(filepath.Join(dir, sidecarproto.SidecarName), []byte("#!/bin/sh\n"), 0o755); err != nil {
		t.Fatalf("write sidecar: %v", err)
	}

	t.Setenv(sidecarproto.SupportDirEnv, dir)

	support, err := Resolve("v1.0.0")

	if err != nil {
		t.Fatalf("Resolve: %v", err)
	}

	if support.Dir != dir {
		t.Fatalf("Dir = %q, want %q", support.Dir, dir)
	}
}

func TestResolveRejectsSupportDirWithoutSidecar(t *testing.T) {
	t.Setenv(sidecarproto.SupportDirEnv, t.TempDir())

	if _, err := Resolve("v1.0.0"); err == nil {
		t.Fatal("expected error for support dir without sidecar")
	}
}

func TestAssetsDigestIsStable(t *testing.T) {
	first, err := assetsDigest(fakeAssets())

	if err != nil {
		t.Fatalf("assetsDigest: %v", err)
	}

	second, err := assetsDigest(fakeAssets())

	if err != nil {
		t.Fatalf("assetsDigest: %v", err)
	}

	if first != second {
		t.Fatalf("digest not stable: %q vs %q", first, second)
	}

	changed := fakeAssets()
	changed["oxfmt.node"] = &fstest.MapFile{Data: []byte("different")}

	third, err := assetsDigest(changed)

	if err != nil {
		t.Fatalf("assetsDigest: %v", err)
	}

	if third == first {
		t.Fatal("digest did not change with contents")
	}
}
