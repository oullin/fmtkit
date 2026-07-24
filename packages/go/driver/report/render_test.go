package report

import (
	"bytes"
	"testing"

	"github.com/fatih/color"
	formatterengine "go.ollin.sh/fmtkit/formatter/engine"
	"go.ollin.sh/fmtkit/vet"
)

func TestRenderDispatch(t *testing.T) {
	previous := color.NoColor
	color.NoColor = true

	t.Cleanup(func() { color.NoColor = previous })

	renderer := Renderer{Root: "/work", Mode: ModeCheck}

	for _, format := range []Format{FormatText, FormatJSON, FormatAgent} {
		var out bytes.Buffer

		if err := renderer.Render(&out, format, sampleCombinedReport()); err != nil {
			t.Fatalf("render %s: %v", format, err)
		}

		if out.Len() == 0 {
			t.Fatalf("render %s produced no output", format)
		}
	}

	var out bytes.Buffer

	err := renderer.Render(&out, Format("yaml"), sampleCombinedReport())

	if err == nil || err.Error() != "unsupported output format" {
		t.Fatalf("expected unsupported format error, got %v", err)
	}
}

func TestParseFormat(t *testing.T) {
	for _, tc := range []struct {
		in   string
		want Format
	}{
		{"text", FormatText},
		{"json", FormatJSON},
		{"agent", FormatAgent},
	} {
		got, err := ParseFormat(tc.in)

		if err != nil {
			t.Fatalf("ParseFormat(%q): %v", tc.in, err)
		}

		if got != tc.want {
			t.Fatalf("ParseFormat(%q) = %q, want %q", tc.in, got, tc.want)
		}
	}

	if _, err := ParseFormat("yaml"); err == nil || err.Error() != "unsupported output format" {
		t.Fatalf("expected unsupported format error, got %v", err)
	}
}

func TestExitCode(t *testing.T) {
	cases := []struct {
		name   string
		mode   Mode
		report Combined
		want   int
	}{
		{
			name:   "vet errors fail either mode",
			mode:   ModeFormat,
			report: Combined{Vet: vet.Report{Errors: []vet.ErrorResult{{Message: "boom"}}}},
			want:   1,
		},
		{
			name:   "check passes on pass result",
			mode:   ModeCheck,
			report: Combined{Formatter: formatterengine.Report{Result: "pass"}},
			want:   0,
		},
		{
			name:   "check fails on non-pass result",
			mode:   ModeCheck,
			report: Combined{Formatter: formatterengine.Report{Result: "fail"}},
			want:   1,
		},
		{
			name: "format fails on formatter errors",
			mode: ModeFormat,
			report: Combined{Formatter: formatterengine.Report{
				Result: "fail",
				Errors: []formatterengine.ErrorResult{{Message: "walk failed"}},
			}},
			want: 1,
		},
		{
			name:   "format succeeds after applying fixes",
			mode:   ModeFormat,
			report: Combined{Formatter: formatterengine.Report{Result: "fixed"}},
			want:   0,
		},
	}

	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			if got := tc.report.ExitCode(tc.mode); got != tc.want {
				t.Fatalf("ExitCode(%s) = %d, want %d", tc.mode, got, tc.want)
			}
		})
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
