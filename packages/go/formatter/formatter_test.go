package formatter_test

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	"go.ollin.sh/fmtkit/driver/testutil"
	"go.ollin.sh/fmtkit/formatter"
	"go.ollin.sh/fmtkit/formatter/config"
)

func TestCheckReportsStyleChanges(t *testing.T) {
	root := t.TempDir()
	path := filepath.Join(root, "sample.go")
	testutil.WriteGoFile(t, path, `package sample

func run() {
	if true {
		println("ok")
	}
	println("next")
}
`)

	report, err := formatter.Check([]string{root}, config.Default())

	if err != nil {
		t.Fatalf("check: %v", err)
	}

	if report.Result != "fail" {
		t.Fatalf("expected fail result, got %q", report.Result)
	}
}

func TestFormatWritesChanges(t *testing.T) {
	root := t.TempDir()
	path := filepath.Join(root, "sample.go")
	testutil.WriteGoFile(t, path, `package sample

func run() {
	defer println("done")
	return
}
`)

	report, err := formatter.Format([]string{root}, config.Default())

	if err != nil {
		t.Fatalf("format: %v", err)
	}

	if report.Result != "fixed" {
		t.Fatalf("expected fixed result, got %q", report.Result)
	}

	content, err := os.ReadFile(path)

	if err != nil {
		t.Fatalf("read file: %v", err)
	}

	if !strings.Contains(string(content), "defer println(\"done\")\n\n\treturn") {
		t.Fatalf("expected formatted file, got:\n%s", content)
	}
}

func TestFormatRepairsGoEmbedDirectivePlacement(t *testing.T) {
	t.Parallel()

	tests := []struct {
		name      string
		directive string
	}{
		{
			name:      "space separated directive",
			directive: "//go:embed foo.txt",
		},
		{
			name:      "tab separated directive",
			directive: "//go:embed\tfoo.txt",
		},
	}

	for _, tt := range tests {
		tt := tt

		t.Run(tt.name, func(t *testing.T) {
			t.Parallel()

			root := t.TempDir()
			path := filepath.Join(root, "sample.go")
			testutil.WriteGoFile(t, path, "package sample\n\nimport \"embed\"\n\n"+tt.directive+"\n\ntype runtime struct{}\n\nvar rootTemplateFS embed.FS\n")

			report, err := formatter.Format([]string{root}, config.Default())

			if err != nil {
				t.Fatalf("format: %v", err)
			}

			if report.Result != "fixed" {
				t.Fatalf("expected fixed result, got %q", report.Result)
			}

			content, err := os.ReadFile(path)

			if err != nil {
				t.Fatalf("read file: %v", err)
			}

			expected := "package sample\n\nimport \"embed\"\n\n" + tt.directive + "\nvar rootTemplateFS embed.FS\n\ntype runtime struct{}\n"

			if string(content) != expected {
				t.Fatalf("expected repaired go:embed placement, got:\n%s", content)
			}
		})
	}
}

func TestFormatPreservesImportsBeforeAnchoredDecls(t *testing.T) {
	t.Parallel()

	tests := []struct {
		name      string
		directive string
	}{
		{
			name:      "space separated directive",
			directive: "//go:embed foo.txt",
		},
		{
			name:      "tab separated directive",
			directive: "//go:embed\tfoo.txt",
		},
	}

	for _, tt := range tests {
		tt := tt

		t.Run(tt.name, func(t *testing.T) {
			t.Parallel()

			root := t.TempDir()
			path := filepath.Join(root, "sample.go")
			testutil.WriteGoFile(t, path, "package sample\n\n"+tt.directive+"\n\nimport \"embed\"\n\ntype runtime struct{}\n\nvar rootTemplateFS embed.FS\n")

			report, err := formatter.Format([]string{root}, config.Default())

			if err != nil {
				t.Fatalf("format: %v", err)
			}

			if report.Result != "fixed" {
				t.Fatalf("expected fixed result, got %q", report.Result)
			}

			content, err := os.ReadFile(path)

			if err != nil {
				t.Fatalf("read file: %v", err)
			}

			expected := "package sample\n\nimport \"embed\"\n\n" + tt.directive + "\nvar rootTemplateFS embed.FS\n\ntype runtime struct{}\n"

			if string(content) != expected {
				t.Fatalf("expected imports to remain before anchored declaration, got:\n%s", content)
			}
		})
	}
}

func TestFormatLeavesAlreadyFormattedFileUnchanged(t *testing.T) {
	root := t.TempDir()
	path := filepath.Join(root, "sample.go")
	testutil.WriteGoFile(t, path, `package sample

import "embed"

//go:embed foo.txt
var rootTemplateFS embed.FS

type runtime struct{}
`)

	original, err := os.ReadFile(path)

	if err != nil {
		t.Fatalf("read file: %v", err)
	}

	report, err := formatter.Format([]string{root}, config.Default())

	if err != nil {
		t.Fatalf("format: %v", err)
	}

	if report.Result != "pass" {
		t.Fatalf("expected pass result, got %q", report.Result)
	}

	content, err := os.ReadFile(path)

	if err != nil {
		t.Fatalf("read file: %v", err)
	}

	if string(content) != string(original) {
		t.Fatalf("expected unchanged file, got:\n%s", content)
	}
}
