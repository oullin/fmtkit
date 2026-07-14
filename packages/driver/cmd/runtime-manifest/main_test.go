package main

import (
	"bytes"
	"os"
	"path/filepath"
	"testing"

	"github.com/oullin/fmtkit/packages/runtimex/integrityx"
)

func TestRunWritesPlatformManifest(t *testing.T) {
	root := t.TempDir()

	if err := os.Mkdir(filepath.Join(root, "bin"), 0o700); err != nil {
		t.Fatalf("create bin: %v", err)
	}

	if err := os.WriteFile(filepath.Join(root, "bin", "node"), []byte("node"), 0o700); err != nil {
		t.Fatalf("write node: %v", err)
	}

	archive := filepath.Join(t.TempDir(), "runtime.tar.gz")

	if err := os.WriteFile(archive, []byte("archive"), 0o600); err != nil {
		t.Fatalf("write archive: %v", err)
	}

	output := filepath.Join(t.TempDir(), "runtime.manifest.json")

	var stderr bytes.Buffer

	if got := run([]string{"--root", root, "--archive", archive, "--output", output, "--goos", "linux", "--goarch", "amd64", "--required", "bin/node"}, &stderr); got != 0 {
		t.Fatalf("run exit=%d stderr=%s", got, stderr.String())
	}

	content, err := os.ReadFile(output)

	if err != nil {
		t.Fatalf("read manifest: %v", err)
	}

	manifest, err := integrityx.Parse(content)

	if err != nil {
		t.Fatalf("parse manifest: %v", err)
	}

	if manifest.GOOS != "linux" || manifest.GOARCH != "amd64" {
		t.Fatalf("unexpected platform: %s/%s", manifest.GOOS, manifest.GOARCH)
	}
}

func TestRunRejectsWhitespaceAndUnsupportedPlatformFlags(t *testing.T) {
	for _, args := range [][]string{
		{"--goos", " ", "--goarch", "arm64"},
		{"--goos", "darwin", "--goarch", "amd64", "--root", ".", "--archive", ".", "--output", "manifest.json", "--required", "bin/node"},
	} {
		var stderr bytes.Buffer

		if got := run(args, &stderr); got == 0 {
			t.Fatalf("expected platform rejection for %q", args)
		}
	}
}
