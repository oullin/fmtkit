package sourcefiles

import (
	"context"
	"os"
	"path/filepath"
	"reflect"
	"testing"
)

func TestCollectSurfacesUnreadablePrettierIgnore(t *testing.T) {
	dir := initRepo(t)
	writeFile(t, filepath.Join(dir, "app.ts"), "const value = 1;\n")
	gitAdd(t, dir, ".")

	// A directory named .prettierignore is not IsNotExist, so the read error must
	// surface rather than being swallowed.
	if err := os.Mkdir(filepath.Join(dir, ".prettierignore"), 0o755); err != nil {
		t.Fatalf("mkdir .prettierignore: %v", err)
	}

	if _, _, err := Collect(context.Background(), Options{Cwd: dir}); err == nil {
		t.Fatal("expected an error from an unreadable .prettierignore")
	}
}

func TestCollectHonorsPrettierIgnore(t *testing.T) {
	dir := initRepo(t)
	writeFile(t, filepath.Join(dir, ".prettierignore"), "generated.ts\ndist/\n")
	writeFile(t, filepath.Join(dir, "app.ts"), "const value = 1;\n")
	writeFile(t, filepath.Join(dir, "generated.ts"), "const generated = 1;\n")
	writeFile(t, filepath.Join(dir, "dist", "bundle.ts"), "const bundle = 1;\n")
	gitAdd(t, dir, ".")

	files, warnings, err := Collect(context.Background(), Options{Cwd: dir})

	if err != nil {
		t.Fatalf("collect: %v", err)
	}

	if len(warnings) != 0 {
		t.Fatalf("unexpected warnings: %v", warnings)
	}

	want := []string{filepath.Join(dir, "app.ts")}

	if !reflect.DeepEqual(files, want) {
		t.Fatalf("files mismatch\nwant: %#v\n got: %#v", want, files)
	}
}

func TestCollectLintableHonorsPrettierIgnore(t *testing.T) {
	dir := initRepo(t)
	writeFile(t, filepath.Join(dir, ".prettierignore"), "vendor/\n")
	writeFile(t, filepath.Join(dir, "app.ts"), "const value = 1;\n")
	writeFile(t, filepath.Join(dir, "vendor", "lib.ts"), "const lib = 1;\n")
	gitAdd(t, dir, ".")

	files, _, err := CollectLintable(context.Background(), Options{Cwd: dir})

	if err != nil {
		t.Fatalf("collect lintable: %v", err)
	}

	want := []string{filepath.Join(dir, "app.ts")}

	if !reflect.DeepEqual(files, want) {
		t.Fatalf("files mismatch\nwant: %#v\n got: %#v", want, files)
	}
}
