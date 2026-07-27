package sourcefiles

import (
	"os"
	"path/filepath"
	"reflect"
	"testing"

	"go.ollin.sh/fmtkit/driver/internal/gitfiles"
	"go.ollin.sh/fmtkit/driver/testutil"
)

func TestCollectSurfacesUnreadablePrettierIgnore(t *testing.T) {
	dir := testutil.InitRepo(t)
	testutil.WriteFile(t, filepath.Join(dir, "app.ts"), "const value = 1;\n")
	testutil.GitAdd(t, dir, ".")

	// A directory named .prettierignore is not IsNotExist, so the read error must
	// surface rather than being swallowed.
	if err := os.Mkdir(filepath.Join(dir, ".prettierignore"), 0o755); err != nil {
		t.Fatalf("mkdir .prettierignore: %v", err)
	}

	if _, _, err := collectFormattable(t, dir, false, gitfiles.SelectionAll); err == nil {
		t.Fatal("expected an error from an unreadable .prettierignore")
	}
}

func TestCollectHonorsPrettierIgnore(t *testing.T) {
	dir := testutil.InitRepo(t)
	testutil.WriteFile(t, filepath.Join(dir, ".prettierignore"), "generated.ts\ndist/\n")
	testutil.WriteFile(t, filepath.Join(dir, "app.ts"), "const value = 1;\n")
	testutil.WriteFile(t, filepath.Join(dir, "generated.ts"), "const generated = 1;\n")
	testutil.WriteFile(t, filepath.Join(dir, "dist", "bundle.ts"), "const bundle = 1;\n")
	testutil.GitAdd(t, dir, ".")

	files, warnings, err := collectFormattable(t, dir, false, gitfiles.SelectionAll)

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
	dir := testutil.InitRepo(t)
	testutil.WriteFile(t, filepath.Join(dir, ".prettierignore"), "vendor/\n")
	testutil.WriteFile(t, filepath.Join(dir, "app.ts"), "const value = 1;\n")
	testutil.WriteFile(t, filepath.Join(dir, "vendor", "lib.ts"), "const lib = 1;\n")
	testutil.GitAdd(t, dir, ".")

	files, _, err := collectLintable(t, dir, false, gitfiles.SelectionAll)

	if err != nil {
		t.Fatalf("collect lintable: %v", err)
	}

	want := []string{filepath.Join(dir, "app.ts")}

	if !reflect.DeepEqual(files, want) {
		t.Fatalf("files mismatch\nwant: %#v\n got: %#v", want, files)
	}
}
