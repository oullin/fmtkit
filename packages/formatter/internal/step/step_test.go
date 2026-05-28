package step_test

import (
	"strings"
	"testing"

	"github.com/oullin/go-fmt/packages/formatter/internal/step"
)

func TestGofmtFormatter(t *testing.T) {
	formatter := step.NewGofmt()

	if formatter.Name() != "gofmt" {
		t.Fatalf("expected gofmt name, got %q", formatter.Name())
	}

	formatted, err := formatter.Format([]byte("package sample\nfunc run( ){println(\"ok\")}\n"))

	if err != nil {
		t.Fatalf("format: %v", err)
	}

	if string(formatted) != "package sample\n\nfunc run() { println(\"ok\") }\n" {
		t.Fatalf("unexpected formatted source:\n%s", formatted)
	}
}

func TestGofmtFormatterReportsSyntaxErrors(t *testing.T) {
	_, err := step.NewGofmt().Format([]byte("package sample\nfunc run("))

	if err == nil {
		t.Fatal("expected syntax error")
	}
}

func TestGoimportsFormatter(t *testing.T) {
	formatter := step.NewGoimports()

	if formatter.Name() != "goimports" {
		t.Fatalf("expected goimports name, got %q", formatter.Name())
	}

	formatted, err := formatter.Format([]byte("package sample\n\nfunc run(){fmt.Println(strings.TrimSpace(\" ok \"))}\n"))

	if err != nil {
		t.Fatalf("format: %v", err)
	}

	got := string(formatted)

	for _, want := range []string{"\"fmt\"", "\"strings\"", "fmt.Println(strings.TrimSpace(\" ok \"))"} {
		if !strings.Contains(got, want) {
			t.Fatalf("expected formatted source to contain %q, got:\n%s", want, got)
		}
	}
}

func TestGoimportsFormatterReportsSyntaxErrors(t *testing.T) {
	_, err := step.NewGoimports().Format([]byte("package sample\nfunc run("))

	if err == nil {
		t.Fatal("expected syntax error")
	}
}
