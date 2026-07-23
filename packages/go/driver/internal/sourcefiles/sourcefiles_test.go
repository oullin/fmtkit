package sourcefiles

import (
	"context"
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
	writeFile(t, filepath.Join(dir, "src", "index.html"), "<script>const value = 1;</script>\n")
	gitAdd(t, dir, ".")

	files, warnings, err := Collect(context.Background(), Options{Cwd: dir, Scopes: []string{"src"}})

	if err != nil {
		t.Fatalf("collect: %v", err)
	}

	if len(warnings) != 0 {
		t.Fatalf("unexpected warnings: %v", warnings)
	}

	want := []string{
		filepath.Join(dir, "src", "app.ts"),
		filepath.Join(dir, "src", "component.vue"),
		filepath.Join(dir, "src", "index.html"),
		filepath.Join(dir, "src", "notes.md"),
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

	files, warnings, err := Collect(context.Background(), Options{Cwd: dir, IncludeDeclarations: true, Scopes: []string{"src"}})

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

func TestCollectLintableExcludesNonScriptDocuments(t *testing.T) {
	dir := initRepo(t)
	writeFile(t, filepath.Join(dir, "src", "app.ts"), "const value = 1;\n")
	writeFile(t, filepath.Join(dir, "src", "component.vue"), "<script setup lang=\"ts\">\nconst value = 1;\n</script>\n")
	writeFile(t, filepath.Join(dir, "src", "types.d.ts"), "declare const value: string;\n")
	writeFile(t, filepath.Join(dir, "src", "notes.md"), "# Notes\n")
	writeFile(t, filepath.Join(dir, "src", "readme.markdown"), "# Readme\n")
	writeFile(t, filepath.Join(dir, "src", "index.html"), "<script>const value = 1;</script>\n")
	gitAdd(t, dir, ".")

	// Formatting owns the HTML and Markdown documents alongside the TS/Vue files.
	formatFiles, _, err := Collect(context.Background(), Options{Cwd: dir, Scopes: []string{"src"}})

	if err != nil {
		t.Fatalf("collect: %v", err)
	}

	wantFormat := []string{
		filepath.Join(dir, "src", "app.ts"),
		filepath.Join(dir, "src", "component.vue"),
		filepath.Join(dir, "src", "index.html"),
		filepath.Join(dir, "src", "notes.md"),
		filepath.Join(dir, "src", "readme.markdown"),
	}

	if !reflect.DeepEqual(formatFiles, wantFormat) {
		t.Fatalf("format files mismatch\nwant: %#v\n got: %#v", wantFormat, formatFiles)
	}

	// Linting sees only the TS/Vue files: no HTML, no Markdown, and .d.ts stays
	// out unless declarations are requested.
	lintFiles, _, err := CollectLintable(context.Background(), Options{Cwd: dir, Scopes: []string{"src"}})

	if err != nil {
		t.Fatalf("collect lintable: %v", err)
	}

	wantLint := []string{
		filepath.Join(dir, "src", "app.ts"),
		filepath.Join(dir, "src", "component.vue"),
	}

	if !reflect.DeepEqual(lintFiles, wantLint) {
		t.Fatalf("lintable files mismatch\nwant: %#v\n got: %#v", wantLint, lintFiles)
	}
}

func TestCollectLintableCanIncludeDeclarationFiles(t *testing.T) {
	dir := initRepo(t)
	writeFile(t, filepath.Join(dir, "src", "app.ts"), "const value = 1;\n")
	writeFile(t, filepath.Join(dir, "src", "types.d.ts"), "declare const value: string;\n")
	writeFile(t, filepath.Join(dir, "src", "index.html"), "<script>const value = 1;</script>\n")
	gitAdd(t, dir, ".")

	files, _, err := CollectLintable(context.Background(), Options{Cwd: dir, IncludeDeclarations: true, Scopes: []string{"src"}})

	if err != nil {
		t.Fatalf("collect lintable: %v", err)
	}

	// Declarations come back when requested; HTML never does.
	want := []string{
		filepath.Join(dir, "src", "app.ts"),
		filepath.Join(dir, "src", "types.d.ts"),
	}

	if !reflect.DeepEqual(files, want) {
		t.Fatalf("lintable files mismatch\nwant: %#v\n got: %#v", want, files)
	}
}

func TestCollectIncludesUntrackedAndIgnoresIgnored(t *testing.T) {
	dir := initRepo(t)
	writeFile(t, filepath.Join(dir, ".gitignore"), "ignored.ts\n")
	writeFile(t, filepath.Join(dir, "tracked.ts"), "const value = 1;\n")
	gitAdd(t, dir, ".gitignore", "tracked.ts")
	writeFile(t, filepath.Join(dir, "untracked.vue"), "<script setup lang=\"ts\"></script>\n")
	writeFile(t, filepath.Join(dir, "ignored.ts"), "const ignored = true;\n")

	files, warnings, err := Collect(context.Background(), Options{Cwd: dir})

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

	files, warnings, err := Collect(context.Background(), Options{
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

func TestCollectChangedCoversOnlyTheWorkingTreesChanges(t *testing.T) {
	dir := initRepo(t)
	writeFile(t, filepath.Join(dir, ".gitignore"), "ignored.ts\n")
	writeFile(t, filepath.Join(dir, "untouched.ts"), "const untouched = 1;\n")
	writeFile(t, filepath.Join(dir, "modified.ts"), "const modified = 1;\n")
	gitAdd(t, dir, ".gitignore", "untouched.ts", "modified.ts")
	gitCommit(t, dir)

	// Only these three diverge from the commit: a tracked file edited in the
	// working tree, a brand new file, and an ignored one that must stay out.
	writeFile(t, filepath.Join(dir, "modified.ts"), "const modified = 2;\n")
	writeFile(t, filepath.Join(dir, "untracked.vue"), "<script setup lang=\"ts\"></script>\n")
	writeFile(t, filepath.Join(dir, "ignored.ts"), "const ignored = true;\n")

	files, warnings, err := Collect(context.Background(), Options{Cwd: dir, Selection: SelectionChanged})

	if err != nil {
		t.Fatalf("collect: %v", err)
	}

	if len(warnings) != 0 {
		t.Fatalf("unexpected warnings: %v", warnings)
	}

	want := []string{
		filepath.Join(dir, "modified.ts"),
		filepath.Join(dir, "untracked.vue"),
	}

	if !reflect.DeepEqual(files, want) {
		t.Fatalf("files mismatch\nwant: %#v\n got: %#v", want, files)
	}
}

func TestCollectChangedIncludesStagedFiles(t *testing.T) {
	dir := initRepo(t)
	writeFile(t, filepath.Join(dir, "staged.ts"), "const staged = 1;\n")
	writeFile(t, filepath.Join(dir, "removed.ts"), "const removed = 1;\n")
	writeFile(t, filepath.Join(dir, "untouched.ts"), "const untouched = 1;\n")
	gitAdd(t, dir, "staged.ts", "removed.ts", "untouched.ts")
	gitCommit(t, dir)

	// Fully staged: the working tree and index agree, but HEAD does not. This is
	// the pre-commit-hook shape, where everything is added before the hook runs.
	writeFile(t, filepath.Join(dir, "staged.ts"), "const staged = 2;\n")
	gitAdd(t, dir, "staged.ts")

	// A staged deletion leaves no file to format and must stay out.
	run(t, dir, "git", "rm", "-q", "removed.ts")

	files, warnings, err := Collect(context.Background(), Options{Cwd: dir, Selection: SelectionChanged})

	if err != nil {
		t.Fatalf("collect: %v", err)
	}

	if len(warnings) != 0 {
		t.Fatalf("unexpected warnings: %v", warnings)
	}

	want := []string{filepath.Join(dir, "staged.ts")}

	if !reflect.DeepEqual(files, want) {
		t.Fatalf("files mismatch\nwant: %#v\n got: %#v", want, files)
	}
}

func TestCollectChangedWorksBeforeTheFirstCommit(t *testing.T) {
	dir := initRepo(t)
	writeFile(t, filepath.Join(dir, "staged.ts"), "const staged = 1;\n")
	gitAdd(t, dir, "staged.ts")

	files, warnings, err := Collect(context.Background(), Options{Cwd: dir, Selection: SelectionChanged})

	if err != nil {
		t.Fatalf("collect: %v", err)
	}

	if len(warnings) != 0 {
		t.Fatalf("unexpected warnings: %v", warnings)
	}

	want := []string{filepath.Join(dir, "staged.ts")}

	if !reflect.DeepEqual(files, want) {
		t.Fatalf("files mismatch\nwant: %#v\n got: %#v", want, files)
	}
}

func TestCollectAllCoversCommittedFilesThatChangedSelectionSkips(t *testing.T) {
	dir := initRepo(t)
	writeFile(t, filepath.Join(dir, "untouched.ts"), "const untouched = 1;\n")
	gitAdd(t, dir, "untouched.ts")
	gitCommit(t, dir)

	changed, _, err := Collect(context.Background(), Options{Cwd: dir, Selection: SelectionChanged})

	if err != nil {
		t.Fatalf("collect changed: %v", err)
	}

	if len(changed) != 0 {
		t.Fatalf("a clean working tree has no changes, got: %#v", changed)
	}

	all, _, err := Collect(context.Background(), Options{Cwd: dir, Selection: SelectionAll})

	if err != nil {
		t.Fatalf("collect all: %v", err)
	}

	want := []string{filepath.Join(dir, "untouched.ts")}

	if !reflect.DeepEqual(all, want) {
		t.Fatalf("files mismatch\nwant: %#v\n got: %#v", want, all)
	}
}

func TestCollectDefaultsToAll(t *testing.T) {
	dir := initRepo(t)
	writeFile(t, filepath.Join(dir, "untouched.ts"), "const untouched = 1;\n")
	gitAdd(t, dir, "untouched.ts")
	gitCommit(t, dir)

	files, _, err := Collect(context.Background(), Options{Cwd: dir})

	if err != nil {
		t.Fatalf("collect: %v", err)
	}

	want := []string{filepath.Join(dir, "untouched.ts")}

	if !reflect.DeepEqual(files, want) {
		t.Fatalf("the zero Selection must cover everything\nwant: %#v\n got: %#v", want, files)
	}
}

func gitCommit(t *testing.T, dir string) {
	t.Helper()

	run(t, dir, "git", "commit", "-q", "-m", "fixture")
}
