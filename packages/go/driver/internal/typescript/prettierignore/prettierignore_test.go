package prettierignore

import (
	"os"
	"path/filepath"
	"reflect"
	"testing"
)

func TestMatcherIgnores(t *testing.T) {
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
			matcher := Compile([]byte(tc.ignore))

			if got := matcher.Ignores(tc.path); got != tc.excluded {
				t.Fatalf("Ignores(%q) = %v, want %v", tc.path, got, tc.excluded)
			}
		})
	}
}

func TestLoadAbsentFile(t *testing.T) {
	matcher, err := Load(filepath.Join(t.TempDir(), ".prettierignore"))

	if err != nil {
		t.Fatalf("Load: %v", err)
	}

	if matcher != nil {
		t.Fatalf("expected nil matcher for absent file, got %#v", matcher)
	}
}

func TestLoadCompilesPresentFile(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, ".prettierignore")

	if err := os.WriteFile(path, []byte("dist/\n"), 0o644); err != nil {
		t.Fatalf("write file: %v", err)
	}

	matcher, err := Load(path)

	if err != nil {
		t.Fatalf("Load: %v", err)
	}

	if matcher == nil || !matcher.Ignores("dist/bundle.ts") {
		t.Fatalf("expected the loaded matcher to honour dist/, got %#v", matcher)
	}
}

func TestLoadSurfacesReadErrors(t *testing.T) {
	dir := t.TempDir()

	// A directory named .prettierignore is not IsNotExist, so its read error must
	// surface rather than being swallowed as "no file".
	if err := os.Mkdir(filepath.Join(dir, ".prettierignore"), 0o755); err != nil {
		t.Fatalf("mkdir: %v", err)
	}

	if _, err := Load(filepath.Join(dir, ".prettierignore")); err == nil {
		t.Fatal("expected an error reading a directory as .prettierignore")
	}
}

func TestFilterAbsDropsIgnoredAndKeepsOutsiders(t *testing.T) {
	root := filepath.Join(string(filepath.Separator), "project", "root")
	matcher := Compile([]byte("dist/\n"))

	files := []string{
		filepath.Join(root, "src", "app.ts"),
		filepath.Join(root, "dist", "bundle.ts"),
		// Above root: .prettierignore cannot speak to it, so it is kept untouched.
		filepath.Join(string(filepath.Separator), "project", "sibling.ts"),
	}

	kept, err := matcher.FilterAbs(root, files)

	if err != nil {
		t.Fatalf("FilterAbs: %v", err)
	}

	want := []string{
		filepath.Join(root, "src", "app.ts"),
		filepath.Join(string(filepath.Separator), "project", "sibling.ts"),
	}

	if !reflect.DeepEqual(kept, want) {
		t.Fatalf("kept mismatch\nwant: %#v\n got: %#v", want, kept)
	}
}
