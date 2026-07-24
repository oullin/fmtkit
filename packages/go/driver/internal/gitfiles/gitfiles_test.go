package gitfiles

import (
	"context"
	"os"
	"os/exec"
	"path/filepath"
	"reflect"
	"testing"
)

func TestNewTreeDefaultsToWorkingDirectory(t *testing.T) {
	tree, err := NewTree("")

	if err != nil {
		t.Fatalf("new tree: %v", err)
	}

	cwd, err := os.Getwd()

	if err != nil {
		t.Fatalf("getwd: %v", err)
	}

	if tree.Dir != cwd {
		t.Fatalf("empty dir must resolve to cwd\nwant: %q\n got: %q", cwd, tree.Dir)
	}
}

func TestNewTreeKeepsGivenDirectory(t *testing.T) {
	dir := t.TempDir()

	tree, err := NewTree(dir)

	if err != nil {
		t.Fatalf("new tree: %v", err)
	}

	if tree.Dir != dir {
		t.Fatalf("dir mismatch\nwant: %q\n got: %q", dir, tree.Dir)
	}
}

func TestFilesListsTrackedAndUntracked(t *testing.T) {
	dir := initRepo(t)
	writeFile(t, filepath.Join(dir, "tracked.ts"), "const value = 1;\n")
	gitAdd(t, dir, "tracked.ts")
	writeFile(t, filepath.Join(dir, "untracked.ts"), "const other = 2;\n")

	tree, err := NewTree(dir)

	if err != nil {
		t.Fatalf("new tree: %v", err)
	}

	entries, err := tree.Files(context.Background(), dir, SelectionAll)

	if err != nil {
		t.Fatalf("files: %v", err)
	}

	// git prints paths relative to the tree; order is git's own, so compare sets.
	got := map[string]struct{}{}

	for _, entry := range entries {
		got[entry] = struct{}{}
	}

	want := map[string]struct{}{"tracked.ts": {}, "untracked.ts": {}}

	if !reflect.DeepEqual(got, want) {
		t.Fatalf("entries mismatch\nwant: %#v\n got: %#v", want, got)
	}
}

func TestFilesSurfacesGitErrorsOutsideARepo(t *testing.T) {
	dir := t.TempDir()

	tree, err := NewTree(dir)

	if err != nil {
		t.Fatalf("new tree: %v", err)
	}

	if _, err := tree.Files(context.Background(), dir, SelectionAll); err == nil {
		t.Fatal("expected an error running git outside a work tree")
	}
}

func TestChangedPathsCoversOnlyTheWorkingTreesChanges(t *testing.T) {
	dir := initRepo(t)
	writeFile(t, filepath.Join(dir, "untouched.ts"), "const untouched = 1;\n")
	writeFile(t, filepath.Join(dir, "modified.ts"), "const modified = 1;\n")
	gitAdd(t, dir, "untouched.ts", "modified.ts")
	gitCommit(t, dir)

	writeFile(t, filepath.Join(dir, "modified.ts"), "const modified = 2;\n")
	writeFile(t, filepath.Join(dir, "added.ts"), "const added = 3;\n")

	tree, err := NewTree(dir)

	if err != nil {
		t.Fatalf("new tree: %v", err)
	}

	files, err := tree.ChangedPaths(context.Background(), nil)

	if err != nil {
		t.Fatalf("changed paths: %v", err)
	}

	want := []string{
		filepath.Join(dir, "added.ts"),
		filepath.Join(dir, "modified.ts"),
	}

	if !reflect.DeepEqual(files, want) {
		t.Fatalf("files mismatch\nwant: %#v\n got: %#v", want, files)
	}
}

func TestChangedPathsIgnoresPrettierIgnore(t *testing.T) {
	dir := initRepo(t)
	writeFile(t, filepath.Join(dir, ".prettierignore"), "main.go\n")
	writeFile(t, filepath.Join(dir, "main.go"), "package main\n")
	gitAdd(t, dir, ".")

	tree, err := NewTree(dir)

	if err != nil {
		t.Fatalf("new tree: %v", err)
	}

	files, err := tree.ChangedPaths(context.Background(), nil)

	if err != nil {
		t.Fatalf("changed paths: %v", err)
	}

	// The Go lane must still see main.go even though .prettierignore lists it:
	// gitfiles never consults .prettierignore.
	want := []string{
		filepath.Join(dir, ".prettierignore"),
		filepath.Join(dir, "main.go"),
	}

	if !reflect.DeepEqual(files, want) {
		t.Fatalf("files mismatch\nwant: %#v\n got: %#v", want, files)
	}
}

func TestChangedPathsSkipsMissingScopes(t *testing.T) {
	dir := initRepo(t)
	writeFile(t, filepath.Join(dir, "src", "app.ts"), "const value = 1;\n")
	gitAdd(t, dir, ".")

	tree, err := NewTree(dir)

	if err != nil {
		t.Fatalf("new tree: %v", err)
	}

	files, err := tree.ChangedPaths(context.Background(), []string{"src", "missing"})

	if err != nil {
		t.Fatalf("changed paths: %v", err)
	}

	want := []string{filepath.Join(dir, "src", "app.ts")}

	if !reflect.DeepEqual(files, want) {
		t.Fatalf("files mismatch\nwant: %#v\n got: %#v", want, files)
	}
}

func TestIntersectChangedKeepsOnlyOwnedAndChanged(t *testing.T) {
	dir := initRepo(t)
	writeFile(t, filepath.Join(dir, "changed.go"), "package main\n")
	writeFile(t, filepath.Join(dir, "untracked.go"), "package main\n")
	gitAdd(t, dir, ".")

	tree, err := NewTree(dir)

	if err != nil {
		t.Fatalf("new tree: %v", err)
	}

	owned := []string{
		filepath.Join(dir, "changed.go"),
		// Owned by the engine but not reported as changed by git.
		filepath.Join(dir, "vendored.go"),
	}

	files, err := tree.IntersectChanged(context.Background(), nil, owned)

	if err != nil {
		t.Fatalf("intersect changed: %v", err)
	}

	// Order follows owned, and only the git-changed intersection survives.
	want := []string{filepath.Join(dir, "changed.go")}

	if !reflect.DeepEqual(files, want) {
		t.Fatalf("files mismatch\nwant: %#v\n got: %#v", want, files)
	}
}

func TestIntersectChangedSurfacesGitErrors(t *testing.T) {
	dir := t.TempDir()

	tree, err := NewTree(dir)

	if err != nil {
		t.Fatalf("new tree: %v", err)
	}

	if _, err := tree.IntersectChanged(context.Background(), nil, []string{filepath.Join(dir, "a.go")}); err == nil {
		t.Fatal("expected an error running git outside a work tree")
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

func gitCommit(t *testing.T, dir string) {
	t.Helper()

	run(t, dir, "git", "commit", "-q", "-m", "fixture")
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
