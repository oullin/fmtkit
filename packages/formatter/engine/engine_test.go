package engine_test

import (
	"fmt"
	"go/format"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/oullin/go-fmt/packages/driver/testutil"
	"github.com/oullin/go-fmt/packages/formatter/config"
	"github.com/oullin/go-fmt/packages/formatter/engine"
	"github.com/oullin/go-fmt/packages/formatter/rules"
	"github.com/oullin/go-fmt/packages/formatter/rules/spacing"
	"golang.org/x/tools/imports"
)

type gofmtFormatter struct{}

type goimportsFormatter struct{}

func defaultRules() []rules.Rule {
	return []rules.Rule{spacing.New()}
}

func (gofmtFormatter) Name() string {
	return "gofmt"
}

func (gofmtFormatter) Format(src []byte) ([]byte, error) {
	return format.Source(src)
}

func (goimportsFormatter) Name() string {
	return "goimports"
}

func (goimportsFormatter) Format(src []byte) ([]byte, error) {
	return imports.Process("", src, nil)
}

func defaultFormatters() []engine.Formatter {
	return []engine.Formatter{gofmtFormatter{}, goimportsFormatter{}}
}

func TestCollectGoFilesSkipsHiddenVendorAndGenerated(t *testing.T) {
	root := t.TempDir()
	testutil.WriteGoFile(t, filepath.Join(root, "root.go"), "package sample\n")
	testutil.WriteGoFile(t, filepath.Join(root, "pkg", "nested.go"), "package sample\n")
	testutil.WriteGoFile(t, filepath.Join(root, "vendor", "skip.go"), "package sample\n")
	testutil.WriteGoFile(t, filepath.Join(root, ".hidden", "skip.go"), "package sample\n")
	testutil.WriteGoFile(t, filepath.Join(root, "generated.gen.go"), "package sample\n")
	testutil.WriteFile(t, filepath.Join(root, "docker", "Dockerfile.go"), "FROM golang:1.26-bookworm\n")

	files, err := engine.CollectGoFiles([]string{root}, config.Default())

	if err != nil {
		t.Fatalf("collect files: %v", err)
	}

	if len(files) != 2 {
		t.Fatalf("expected 2 files, got %d: %#v", len(files), files)
	}
}

func TestCheckReportsStyleChangesWithoutWriting(t *testing.T) {
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

	report, err := engine.New(config.Default(), defaultRules(), defaultFormatters()).Check([]string{root})

	if err != nil {
		t.Fatalf("check: %v", err)
	}

	if report.Result != "fail" {
		t.Fatalf("expected fail result, got %q", report.Result)
	}

	if report.Changed != 1 {
		t.Fatalf("expected 1 changed file, got %d", report.Changed)
	}

	content, err := os.ReadFile(path)

	if err != nil {
		t.Fatalf("read file: %v", err)
	}

	if strings.Contains(string(content), "\n\n\tprintln(\"next\")") {
		t.Fatalf("check should not write changes")
	}
}

func TestFormatWritesSpacingChanges(t *testing.T) {
	root := t.TempDir()
	path := filepath.Join(root, "sample.go")
	testutil.WriteGoFile(t, path, `package sample

func run() {
	defer println("done")
	return
}
`)

	report, err := engine.New(config.Default(), defaultRules(), defaultFormatters()).Format([]string{root})

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
		t.Fatalf("expected file to be rewritten, got:\n%s", content)
	}
}

func TestFormatIsDeterministicAcrossConcurrencyLevels(t *testing.T) {
	const fileCount = 16

	source := `package sample

func run() {
	defer println("done")
	return
}
`

	makeTree := func(t *testing.T) string {
		t.Helper()
		root := t.TempDir()

		for i := 0; i < fileCount; i++ {
			path := filepath.Join(root, fmt.Sprintf("pkg%02d", i), "sample.go")
			testutil.WriteGoFile(t, path, source)
		}

		return root
	}

	readAll := func(t *testing.T, root string) map[string]string {
		t.Helper()
		got := map[string]string{}

		err := filepath.Walk(root, func(path string, info os.FileInfo, err error) error {
			if err != nil {
				return err
			}

			if info.IsDir() || filepath.Ext(path) != ".go" {
				return nil
			}

			data, err := os.ReadFile(path)

			if err != nil {
				return err
			}

			rel, err := filepath.Rel(root, path)

			if err != nil {
				return err
			}

			got[rel] = string(data)

			return nil
		})

		if err != nil {
			t.Fatalf("walk: %v", err)
		}

		return got
	}

	runWith := func(t *testing.T, concurrency int) (engine.Report, map[string]string) {
		t.Helper()
		root := makeTree(t)
		cfg := config.Default()
		cfg.Concurrency = concurrency

		report, err := engine.New(cfg, defaultRules(), defaultFormatters()).Format([]string{root})

		if err != nil {
			t.Fatalf("format (concurrency=%d): %v", concurrency, err)
		}

		return report, readAll(t, root)
	}

	serialReport, serialContent := runWith(t, 1)
	parallelReport, parallelContent := runWith(t, 8)

	if serialReport.Files != parallelReport.Files {
		t.Fatalf("Files mismatch: serial=%d parallel=%d", serialReport.Files, parallelReport.Files)
	}

	if serialReport.Changed != parallelReport.Changed {
		t.Fatalf("Changed mismatch: serial=%d parallel=%d", serialReport.Changed, parallelReport.Changed)
	}

	if serialReport.Result != parallelReport.Result {
		t.Fatalf("Result mismatch: serial=%q parallel=%q", serialReport.Result, parallelReport.Result)
	}

	if len(serialReport.Results) != len(parallelReport.Results) {
		t.Fatalf("Results length mismatch: serial=%d parallel=%d", len(serialReport.Results), len(parallelReport.Results))
	}

	for i := range serialReport.Results {
		s := filepath.Base(filepath.Dir(serialReport.Results[i].File))
		p := filepath.Base(filepath.Dir(parallelReport.Results[i].File))

		if s != p {
			t.Fatalf("Results order diverged at index %d: serial=%s parallel=%s", i, s, p)
		}
	}

	if len(serialContent) != len(parallelContent) {
		t.Fatalf("content map size mismatch: serial=%d parallel=%d", len(serialContent), len(parallelContent))
	}

	for rel, want := range serialContent {
		got, ok := parallelContent[rel]

		if !ok {
			t.Fatalf("missing %s in parallel output", rel)
		}

		if got != want {
			t.Fatalf("content mismatch for %s\n--- serial\n%s\n--- parallel\n%s", rel, want, got)
		}
	}
}

func TestFormatSkipsSingleLineFuncLiteralSpacingViolations(t *testing.T) {
	root := t.TempDir()
	path := filepath.Join(root, "sample.go")
	testutil.WriteGoFile(t, path, `package sample

type config struct {
	SecureCookie bool
}

func run() config {
	return config{
		SecureCookie: func() bool { value := true; return value }(),
	}
}
`)

	report, err := engine.New(config.Default(), defaultRules(), defaultFormatters()).Format([]string{root})

	if err != nil {
		t.Fatalf("format: %v", err)
	}

	if report.Result != "pass" {
		t.Fatalf("expected pass result, got %q", report.Result)
	}

	if report.Changed != 0 {
		t.Fatalf("expected 0 changed files, got %d", report.Changed)
	}

	if report.ViolationCount() != 0 {
		t.Fatalf("expected 0 violations, got %d", report.ViolationCount())
	}
}
