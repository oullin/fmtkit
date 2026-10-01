package runtime

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestProjectOxlintExecutableFindsInstalledPackage(t *testing.T) {
	root := t.TempDir()
	bin := filepath.Join(root, "node_modules", ".bin", "oxlint")
	manifest := filepath.Join(root, "node_modules", "oxlint", "package.json")

	for _, path := range []string{bin, manifest} {
		if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
			t.Fatal(err)
		}
	}

	if err := os.WriteFile(bin, []byte("#!/bin/sh\n"), 0o755); err != nil {
		t.Fatal(err)
	}

	if err := os.WriteFile(manifest, []byte(`{"version":"1.80.0"}`), 0o644); err != nil {
		t.Fatal(err)
	}

	nested := filepath.Join(root, "package")

	if got, err := projectOxlintExecutable(root, nested, ""); err != nil || got != bin {
		t.Fatalf("projectOxlintExecutable = (%q, %v), want (%q, nil)", got, err, bin)
	}
}

func TestProjectOxlintExecutableExplainsMissingPackage(t *testing.T) {
	root := t.TempDir()
	_, err := projectOxlintExecutable(root, root, "")

	if err == nil || !strings.Contains(err.Error(), "project-installed Node-based oxlint") {
		t.Fatalf("error = %v, want missing-package explanation", err)
	}
}

func TestProjectOxlintExecutableRejectsOldVersion(t *testing.T) {
	root := t.TempDir()
	bin := filepath.Join(root, "node_modules", ".bin", "oxlint")
	manifest := filepath.Join(root, "node_modules", "oxlint", "package.json")

	for _, path := range []string{bin, manifest} {
		if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
			t.Fatal(err)
		}
	}

	if err := os.WriteFile(bin, []byte("#!/bin/sh\n"), 0o755); err != nil {
		t.Fatal(err)
	}

	if err := os.WriteFile(manifest, []byte(`{"version":"1.79.0"}`), 0o644); err != nil {
		t.Fatal(err)
	}

	_, err := projectOxlintExecutable(root, root, "")

	if err == nil || !strings.Contains(err.Error(), "oxlint 1.80.0+") {
		t.Fatalf("error = %v, want version explanation", err)
	}
}
