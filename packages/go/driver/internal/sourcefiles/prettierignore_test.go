package sourcefiles

import (
	"context"
	"os"
	"path/filepath"
	"reflect"
	"testing"
)

func TestPrettierIgnoreMatches(t *testing.T) {
	cases := []struct {
		name     string
		ignore   string
		path     string
		excluded bool
	}{
		{"blank and comment lines are inert", "\n# comment\n", "app.ts", false},
		{"plain name matches at root", "app.ts\n", "app.ts", true},
		{"plain name matches at any depth", "app.ts\n", "src/nested/app.ts", true},
		{"plain name does not match a different file", "app.ts\n", "app.tsx", false},
		{"leading slash anchors to root", "/app.ts\n", "app.ts", true},
		{"leading slash rejects nested", "/app.ts\n", "src/app.ts", false},
		{"trailing slash matches under a directory", "dist/\n", "dist/app.ts", true},
		{"trailing slash does not match a file of that name", "dist/\n", "dist", false},
		{"star stays within a segment", "*.ts\n", "src/app.ts", true},
		{"star does not cross a slash", "src/*.ts\n", "src/nested/app.ts", false},
		{"question matches one char", "app.?s\n", "app.ts", true},
		{"question needs exactly one char", "app.?s\n", "app.tss", false},
		{"character class matches a member", "app.[jt]s\n", "app.ts", true},
		{"character class rejects a non-member", "app.[jt]s\n", "app.xs", false},
		{"negated class rejects a member", "app.[!t]s\n", "app.ts", false},
		{"negated class matches a non-member", "app.[!t]s\n", "app.js", true},
		{"double star spans segments", "src/**/app.ts\n", "src/a/b/app.ts", true},
		{"double star spans zero segments", "src/**/app.ts\n", "src/app.ts", true},
		{"trailing double star matches everything below", "logs/**\n", "logs/a/b.txt", true},
		{"leading double star matches at any depth", "**/gen.ts\n", "a/b/gen.ts", true},
		{"excluding a directory excludes its contents", "build\n", "build/x/y.ts", true},
		{"a similarly named directory is untouched", "build\n", "prebuild/x.ts", false},
		{"negation re-includes a previously excluded file", "*.ts\n!keep.ts\n", "keep.ts", false},
		{"order matters: re-exclude after negation", "*.ts\n!keep.ts\nkeep.ts\n", "keep.ts", true},
		{"unclosed bracket is a literal", "a[b.ts\n", "a[b.ts", true},
		{"invalid character class is skipped without panicking", "[z-a]\napp.ts\n", "app.ts", true},
		{"invalid character class does not match its own line", "[z-a]\n", "z", false},
		{"crlf line trims the carriage return before spaces", "foo \r\n", "foo", true},
	}

	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			ignore := compilePrettierIgnore([]byte(tc.ignore))

			if got := ignore.ignores(tc.path); got != tc.excluded {
				t.Fatalf("ignores(%q) = %v, want %v", tc.path, got, tc.excluded)
			}
		})
	}
}

func TestLoadPrettierIgnoreAbsentFile(t *testing.T) {
	ignore, err := loadPrettierIgnore(filepath.Join(t.TempDir(), ".prettierignore"))

	if err != nil {
		t.Fatalf("loadPrettierIgnore: %v", err)
	}

	if ignore != nil {
		t.Fatalf("expected nil matcher for absent file, got %#v", ignore)
	}
}

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

func TestChangedPathsIgnoresPrettierIgnore(t *testing.T) {
	dir := initRepo(t)
	writeFile(t, filepath.Join(dir, ".prettierignore"), "main.go\n")
	writeFile(t, filepath.Join(dir, "main.go"), "package main\n")
	gitAdd(t, dir, ".")

	files, err := ChangedPaths(context.Background(), dir, nil)

	if err != nil {
		t.Fatalf("changed paths: %v", err)
	}

	// The Go lane must still see main.go even though .prettierignore lists it.
	want := []string{
		filepath.Join(dir, ".prettierignore"),
		filepath.Join(dir, "main.go"),
	}

	if !reflect.DeepEqual(files, want) {
		t.Fatalf("files mismatch\nwant: %#v\n got: %#v", want, files)
	}
}
