package sourcefiles

import (
	"os"
	"os/exec"
	"path/filepath"
	"reflect"
	"testing"
)

func TestCollectFiltersSourceFiles(t *testing.T) {
	dir := initRepo(t)
	writeFile(t, filepath.Join(dir, "src", "app.ts"), "const value = 1;\n")
	writeFile(t, filepath.Join(dir, "src", "component.vue"), "<script setup lang=\"ts\">\nconst value = 1;\n</script>\n")
	writeFile(t, filepath.Join(dir, "src", "types.d.ts"), "declare const value: string;\n")
	writeFile(t, filepath.Join(dir, "src", "notes.md"), "# Notes\n")
	gitAdd(t, dir, ".")

	files, warnings, err := Collect(Options{Cwd: dir, Scopes: []string{"src"}})

	if err != nil {
		t.Fatalf("collect: %v", err)
	}

	if len(warnings) != 0 {
		t.Fatalf("unexpected warnings: %v", warnings)
	}

	want := []string{
		filepath.Join(dir, "src", "app.ts"),
		filepath.Join(dir, "src", "component.vue"),
	}

	if !reflect.DeepEqual(files, want) {
		t.Fatalf("files mismatch\nwant: %#v\n got: %#v", want, files)
	}
}

func TestCollectCanIncludeDeclarationFiles(t *testing.T) {
	dir := initRepo(t)
	writeFile(t, filepath.Join(dir, "src", "app.ts"), "const value = 1;\n")
	writeFile(t, filepath.Join(dir, "src", "types.d.ts"), "declare const value: string;\n")
	gitAdd(t, dir, ".")

	files, warnings, err := Collect(Options{Cwd: dir, IncludeDeclarations: true, Scopes: []string{"src"}})

	if err != nil {
		t.Fatalf("collect: %v", err)
	}

	if len(warnings) != 0 {
		t.Fatalf("unexpected warnings: %v", warnings)
	}

	want := []string{
		filepath.Join(dir, "src", "app.ts"),
		filepath.Join(dir, "src", "types.d.ts"),
	}

	if !reflect.DeepEqual(files, want) {
		t.Fatalf("files mismatch\nwant: %#v\n got: %#v", want, files)
	}
}

func TestCollectIncludesUntrackedAndIgnoresIgnored(t *testing.T) {
	dir := initRepo(t)
	writeFile(t, filepath.Join(dir, ".gitignore"), "ignored.ts\n")
	writeFile(t, filepath.Join(dir, "tracked.ts"), "const value = 1;\n")
	gitAdd(t, dir, ".gitignore", "tracked.ts")
	writeFile(t, filepath.Join(dir, "untracked.vue"), "<script setup lang=\"ts\"></script>\n")
	writeFile(t, filepath.Join(dir, "ignored.ts"), "const ignored = true;\n")

	files, warnings, err := Collect(Options{Cwd: dir})

	if err != nil {
		t.Fatalf("collect: %v", err)
	}

	if len(warnings) != 0 {
		t.Fatalf("unexpected warnings: %v", warnings)
	}

	want := []string{
		filepath.Join(dir, "tracked.ts"),
		filepath.Join(dir, "untracked.vue"),
	}

	if !reflect.DeepEqual(files, want) {
		t.Fatalf("files mismatch\nwant: %#v\n got: %#v", want, files)
	}
}

func TestCollectScopesAndDeduplicatesFiles(t *testing.T) {
	dir := initRepo(t)
	writeFile(t, filepath.Join(dir, "src", "app.ts"), "const value = 1;\n")
	writeFile(t, filepath.Join(dir, "other", "app.ts"), "const value = 2;\n")
	gitAdd(t, dir, ".")

	files, warnings, err := Collect(Options{
		Cwd:    dir,
		Scopes: []string{"src", filepath.Join(dir, "src", "app.ts"), "missing"},
	})

	if err != nil {
		t.Fatalf("collect: %v", err)
	}

	if len(warnings) != 1 {
		t.Fatalf("expected one warning, got %v", warnings)
	}

	want := []string{filepath.Join(dir, "src", "app.ts")}

	if !reflect.DeepEqual(files, want) {
		t.Fatalf("files mismatch\nwant: %#v\n got: %#v", want, files)
	}
}

func initRepo(t *testing.T) string {
	t.Helper()

	dir := t.TempDir()
	run(t, dir, "git", "init", "-q")
	run(t, dir, "git", "config", "user.email", "tests@example.com")
	run(t, dir, "git", "config", "user.name", "Test Runner")

	return dir
}

func gitAdd(t *testing.T, dir string, paths ...string) {
	t.Helper()

	args := append([]string{"add"}, paths...)
	run(t, dir, "git", args...)
}

func writeFile(t *testing.T, path string, content string) {
	t.Helper()

	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		t.Fatalf("mkdir: %v", err)
	}

	if err := os.WriteFile(path, []byte(content), 0o644); err != nil {
		t.Fatalf("write file: %v", err)
	}
}

func run(t *testing.T, dir string, name string, args ...string) {
	t.Helper()

	cmd := exec.Command(name, args...)
	cmd.Dir = dir

	if out, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("%s %v: %v\n%s", name, args, err, out)
	}
}
