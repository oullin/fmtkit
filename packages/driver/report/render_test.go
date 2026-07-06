package report

import (
	"bytes"
	"testing"

	"github.com/fatih/color"
	formatterengine "github.com/oullin/go-fmt/packages/formatter/engine"
	"github.com/oullin/go-fmt/packages/vet"
)

func TestRenderDispatch(t *testing.T) {
	previous := color.NoColor
	color.NoColor = true

	t.Cleanup(func() { color.NoColor = previous })

	for _, format := range []string{"text", "json", "agent"} {
		var out bytes.Buffer

		if err := Render(&out, format, "/work", "check", sampleCombinedReport()); err != nil {
			t.Fatalf("render %s: %v", format, err)
		}

		if out.Len() == 0 {
			t.Fatalf("render %s produced no output", format)
		}
	}

	var out bytes.Buffer

	err := Render(&out, "yaml", "/work", "check", sampleCombinedReport())

	if err == nil || err.Error() != "unsupported output format" {
		t.Fatalf("expected unsupported format error, got %v", err)
	}
}

func TestCombinedResult(t *testing.T) {
	cases := []struct {
		name   string
		report Combined
		want   string
	}{
		{
			name:   "vet errors force fail",
			report: Combined{Formatter: formatterengine.Report{Result: "pass"}, Vet: vet.Report{Errors: []vet.ErrorResult{{Message: "boom"}}}},
			want:   "fail",
		},
		{
			name:   "formatter errors force fail",
			report: Combined{Formatter: formatterengine.Report{Result: "fixed", Errors: []formatterengine.ErrorResult{{Message: "walk failed"}}}},
			want:   "fail",
		},
		{
			name:   "formatter result passes through",
			report: Combined{Formatter: formatterengine.Report{Result: "pass"}},
			want:   "pass",
		},
	}

	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			if got := combinedResult(tc.report); got != tc.want {
				t.Fatalf("combinedResult() = %q, want %q", got, tc.want)
			}
		})
	}
}

func TestVetStatus(t *testing.T) {
	cases := []struct {
		name   string
		report vet.Report
		want   string
	}{
		{name: "skipped toolchain", report: vet.Report{Skipped: true, Root: "/work"}, want: "skipped"},
		{name: "skipped no module", report: vet.Report{}, want: "skipped"},
		{name: "fail", report: vet.Report{Root: "/work", Errors: []vet.ErrorResult{{Message: "boom"}}}, want: "fail"},
		{name: "pass", report: vet.Report{Root: "/work"}, want: "pass"},
	}

	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			if got := vetStatus(tc.report); got != tc.want {
				t.Fatalf("vetStatus() = %q, want %q", got, tc.want)
			}
		})
	}
}

func TestRelativePath(t *testing.T) {
	if got := relativePath("/work", "/work/pkg/main.go"); got != "pkg/main.go" {
		t.Fatalf("relativePath() = %q", got)
	}

	// filepath.Rel cannot express an absolute target against a relative
	// root, so the original path is returned unchanged.
	if got := relativePath("relative-root", "/abs/main.go"); got != "/abs/main.go" {
		t.Fatalf("relativePath() fallback = %q", got)
	}
}
