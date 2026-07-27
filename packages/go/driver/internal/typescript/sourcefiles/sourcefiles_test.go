package sourcefiles

import (
	"context"
	"path/filepath"
	"reflect"
	"testing"

	"go.ollin.sh/fmtkit/driver/internal/gitfiles"
	"go.ollin.sh/fmtkit/driver/testutil"
)

// collectFormattable and collectLintable build a Collector rooted at cwd and
// run the corresponding discovery, so each test names only the axes it cares
// about (declarations, selection, scopes).
func collectFormattable(t *testing.T, cwd string, includeDeclarations bool, selection gitfiles.Selection, scopes ...string) ([]string, []string, error) {
	t.Helper()

	collector, err := New(cwd, selection, includeDeclarations)

	if err != nil {
		t.Fatalf("new collector: %v", err)
	}

	return collector.Formattable(context.Background(), scopes)
}

func collectLintable(t *testing.T, cwd string, includeDeclarations bool, selection gitfiles.Selection, scopes ...string) ([]string, []string, error) {
	t.Helper()

	collector, err := New(cwd, selection, includeDeclarations)

	if err != nil {
		t.Fatalf("new collector: %v", err)
	}

	return collector.Lintable(context.Background(), scopes)
}

func TestCollectFiltersSourceFiles(t *testing.T) {
	dir := testutil.InitRepo(t)
	testutil.WriteFile(t, filepath.Join(dir, "src", "app.ts"), "const value = 1;\n")
	testutil.WriteFile(t, filepath.Join(dir, "src", "component.vue"), "<script setup lang=\"ts\">\nconst value = 1;\n</script>\n")
	testutil.WriteFile(t, filepath.Join(dir, "src", "types.d.ts"), "declare const value: string;\n")
	testutil.WriteFile(t, filepath.Join(dir, "src", "notes.md"), "# Notes\n")
	testutil.WriteFile(t, filepath.Join(dir, "src", "index.html"), "<script>const value = 1;</script>\n")
	testutil.GitAdd(t, dir, ".")

	files, warnings, err := collectFormattable(t, dir, false, gitfiles.SelectionAll, "src")

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
	dir := testutil.InitRepo(t)
	testutil.WriteFile(t, filepath.Join(dir, "src", "app.ts"), "const value = 1;\n")
	testutil.WriteFile(t, filepath.Join(dir, "src", "types.d.ts"), "declare const value: string;\n")
	testutil.GitAdd(t, dir, ".")

	files, warnings, err := collectFormattable(t, dir, true, gitfiles.SelectionAll, "src")

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
	dir := testutil.InitRepo(t)
	testutil.WriteFile(t, filepath.Join(dir, "src", "app.ts"), "const value = 1;\n")
	testutil.WriteFile(t, filepath.Join(dir, "src", "component.vue"), "<script setup lang=\"ts\">\nconst value = 1;\n</script>\n")
	testutil.WriteFile(t, filepath.Join(dir, "src", "types.d.ts"), "declare const value: string;\n")
	testutil.WriteFile(t, filepath.Join(dir, "src", "notes.md"), "# Notes\n")
	testutil.WriteFile(t, filepath.Join(dir, "src", "readme.markdown"), "# Readme\n")
	testutil.WriteFile(t, filepath.Join(dir, "src", "index.html"), "<script>const value = 1;</script>\n")
	testutil.GitAdd(t, dir, ".")

	// Formatting owns the HTML and Markdown documents alongside the TS/Vue files.
	formatFiles, _, err := collectFormattable(t, dir, false, gitfiles.SelectionAll, "src")

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
	lintFiles, _, err := collectLintable(t, dir, false, gitfiles.SelectionAll, "src")

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
	dir := testutil.InitRepo(t)
	testutil.WriteFile(t, filepath.Join(dir, "src", "app.ts"), "const value = 1;\n")
	testutil.WriteFile(t, filepath.Join(dir, "src", "types.d.ts"), "declare const value: string;\n")
	testutil.WriteFile(t, filepath.Join(dir, "src", "index.html"), "<script>const value = 1;</script>\n")
	testutil.GitAdd(t, dir, ".")

	files, _, err := collectLintable(t, dir, true, gitfiles.SelectionAll, "src")

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
	dir := testutil.InitRepo(t)
	testutil.WriteFile(t, filepath.Join(dir, ".gitignore"), "ignored.ts\n")
	testutil.WriteFile(t, filepath.Join(dir, "tracked.ts"), "const value = 1;\n")
	testutil.GitAdd(t, dir, ".gitignore", "tracked.ts")
	testutil.WriteFile(t, filepath.Join(dir, "untracked.vue"), "<script setup lang=\"ts\"></script>\n")
	testutil.WriteFile(t, filepath.Join(dir, "ignored.ts"), "const ignored = true;\n")

	files, warnings, err := collectFormattable(t, dir, false, gitfiles.SelectionAll)

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
	dir := testutil.InitRepo(t)
	testutil.WriteFile(t, filepath.Join(dir, "src", "app.ts"), "const value = 1;\n")
	testutil.WriteFile(t, filepath.Join(dir, "other", "app.ts"), "const value = 2;\n")
	testutil.GitAdd(t, dir, ".")

	files, warnings, err := collectFormattable(t, dir, false, gitfiles.SelectionAll,
		"src", filepath.Join(dir, "src", "app.ts"), "missing")

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

func TestCollectChangedCoversOnlyTheWorkingTreesChanges(t *testing.T) {
	dir := testutil.InitRepo(t)
	testutil.WriteFile(t, filepath.Join(dir, ".gitignore"), "ignored.ts\n")
	testutil.WriteFile(t, filepath.Join(dir, "untouched.ts"), "const untouched = 1;\n")
	testutil.WriteFile(t, filepath.Join(dir, "modified.ts"), "const modified = 1;\n")
	testutil.GitAdd(t, dir, ".gitignore", "untouched.ts", "modified.ts")
	testutil.GitCommit(t, dir)

	// Only these three diverge from the commit: a tracked file edited in the
	// working tree, a brand new file, and an ignored one that must stay out.
	testutil.WriteFile(t, filepath.Join(dir, "modified.ts"), "const modified = 2;\n")
	testutil.WriteFile(t, filepath.Join(dir, "untracked.vue"), "<script setup lang=\"ts\"></script>\n")
	testutil.WriteFile(t, filepath.Join(dir, "ignored.ts"), "const ignored = true;\n")

	files, warnings, err := collectFormattable(t, dir, false, gitfiles.SelectionChanged)

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
	dir := testutil.InitRepo(t)
	testutil.WriteFile(t, filepath.Join(dir, "staged.ts"), "const staged = 1;\n")
	testutil.WriteFile(t, filepath.Join(dir, "removed.ts"), "const removed = 1;\n")
	testutil.WriteFile(t, filepath.Join(dir, "untouched.ts"), "const untouched = 1;\n")
	testutil.GitAdd(t, dir, "staged.ts", "removed.ts", "untouched.ts")
	testutil.GitCommit(t, dir)

	// Fully staged: the working tree and index agree, but HEAD does not. This is
	// the pre-commit-hook shape, where everything is added before the hook runs.
	testutil.WriteFile(t, filepath.Join(dir, "staged.ts"), "const staged = 2;\n")
	testutil.GitAdd(t, dir, "staged.ts")

	// A staged deletion leaves no file to format and must stay out.
	testutil.Run(t, dir, "git", "rm", "-q", "removed.ts")

	files, warnings, err := collectFormattable(t, dir, false, gitfiles.SelectionChanged)

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
	dir := testutil.InitRepo(t)
	testutil.WriteFile(t, filepath.Join(dir, "staged.ts"), "const staged = 1;\n")
	testutil.GitAdd(t, dir, "staged.ts")

	files, warnings, err := collectFormattable(t, dir, false, gitfiles.SelectionChanged)

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
	dir := testutil.InitRepo(t)
	testutil.WriteFile(t, filepath.Join(dir, "untouched.ts"), "const untouched = 1;\n")
	testutil.GitAdd(t, dir, "untouched.ts")
	testutil.GitCommit(t, dir)

	changed, _, err := collectFormattable(t, dir, false, gitfiles.SelectionChanged)

	if err != nil {
		t.Fatalf("collect changed: %v", err)
	}

	if len(changed) != 0 {
		t.Fatalf("a clean working tree has no changes, got: %#v", changed)
	}

	all, _, err := collectFormattable(t, dir, false, gitfiles.SelectionAll)

	if err != nil {
		t.Fatalf("collect all: %v", err)
	}

	want := []string{filepath.Join(dir, "untouched.ts")}

	if !reflect.DeepEqual(all, want) {
		t.Fatalf("files mismatch\nwant: %#v\n got: %#v", want, all)
	}
}

func TestCollectDefaultsToAll(t *testing.T) {
	dir := testutil.InitRepo(t)
	testutil.WriteFile(t, filepath.Join(dir, "untouched.ts"), "const untouched = 1;\n")
	testutil.GitAdd(t, dir, "untouched.ts")
	testutil.GitCommit(t, dir)

	// The zero gitfiles.Selection is SelectionAll, so a Collector built with it
	// must cover committed files a changed run would skip.
	files, _, err := collectFormattable(t, dir, false, gitfiles.Selection(0))

	if err != nil {
		t.Fatalf("collect: %v", err)
	}

	want := []string{filepath.Join(dir, "untouched.ts")}

	if !reflect.DeepEqual(files, want) {
		t.Fatalf("the zero Selection must cover everything\nwant: %#v\n got: %#v", want, files)
	}
}
