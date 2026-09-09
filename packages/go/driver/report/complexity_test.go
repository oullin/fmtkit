package report

import (
	"strings"
	"testing"

	"go.ollin.sh/fmtkit/complexity"
)

func complexityReport() *complexity.Report {
	return &complexity.Report{
		Files:     2,
		Functions: 5,
		Findings: []complexity.Finding{
			{
				Rule:    complexity.RuleCyclomatic,
				File:    "internal/envcfg/envcfg.go",
				Line:    31,
				Key:     "internal/envcfg/envcfg.go#(*Source).Read",
				Message: "internal/envcfg/envcfg.go#(*Source).Read scores 33 (limit 15)",
			},
			{
				Rule:    complexity.RuleAllow,
				File:    "src/app.ts",
				Key:     "src/app.ts#gone",
				Message: `allow entry "src/app.ts#gone" matches no function`,
			},
		},
		Errors: []complexity.ErrorResult{{File: "src/broken.ts", Message: "unparsable"}},
	}
}

func TestComplexityStatusAndSummaryClassifyTheSection(t *testing.T) {
	if got := ComplexityStatus(Combined{}); got != "skipped" {
		t.Errorf("status without a complexity run = %q", got)
	}

	if got := ComplexitySummary(Combined{}); !strings.Contains(got, "Skipped") {
		t.Errorf("summary without a complexity run = %q", got)
	}

	passing := Combined{Complexity: &complexity.Report{Files: 2, Functions: 5}}

	if got := ComplexityStatus(passing); got != "pass" {
		t.Errorf("passing status = %q", got)
	}

	if got := ComplexitySummary(passing); got != "Scored 5 function(s) in 2 file(s); none over the limits." {
		t.Errorf("passing summary = %q", got)
	}

	if got := ComplexitySummary(Combined{Complexity: complexityReport()}); got != "" {
		t.Errorf("failing summary = %q, want the per-finding lines instead", got)
	}
}

// A complexity failure fails every mode, and the complexity mode ignores the
// formatter and vet sections it never populated.
func TestExitCodeAnswersToTheComplexitySection(t *testing.T) {
	failing := Combined{Complexity: complexityReport()}

	for _, mode := range []Mode{ModeCheck, ModeFormat, ModeComplexity} {
		if got := failing.ExitCode(mode); got != 1 {
			t.Errorf("ExitCode(%q) = %d, want 1", mode, got)
		}
	}

	passing := Combined{Complexity: &complexity.Report{}}

	if got := passing.ExitCode(ModeComplexity); got != 0 {
		t.Errorf("passing ExitCode = %d, want 0", got)
	}
}

func TestRenderTextWritesTheComplexitySection(t *testing.T) {
	var out strings.Builder

	renderer := Renderer{Root: t.TempDir(), Mode: ModeComplexity}

	if err := renderer.Render(&out, FormatText, Combined{Complexity: complexityReport()}); err != nil {
		t.Fatalf("render: %v", err)
	}

	want := []string{
		"internal/envcfg/envcfg.go:31: complexity/cyclomatic: internal/envcfg/envcfg.go#(*Source).Read scores 33 (limit 15)",
		"src/app.ts: complexity/allow:",
		"unparsable",
		"Result: fail. 2 finding(s), 1 error(s).",
	}

	for _, line := range want {
		if !strings.Contains(out.String(), line) {
			t.Fatalf("render missing %q:\n%s", line, out.String())
		}
	}

	if strings.Contains(out.String(), "Formatter") {
		t.Fatalf("the complexity mode rendered a formatter section:\n%s", out.String())
	}
}

func TestRenderTextAppendsComplexityToACheckReport(t *testing.T) {
	var out strings.Builder

	renderer := Renderer{Root: t.TempDir(), Mode: ModeCheck}

	if err := renderer.Render(&out, FormatText, Combined{Complexity: &complexity.Report{Files: 1, Functions: 1}}); err != nil {
		t.Fatalf("render: %v", err)
	}

	for _, line := range []string{"Formatter", "Vet", "Complexity", "none over the limits"} {
		if !strings.Contains(out.String(), line) {
			t.Fatalf("render missing %q:\n%s", line, out.String())
		}
	}
}

func TestRenderJSONAndAgentCarryTheComplexitySection(t *testing.T) {
	for _, format := range []Format{FormatJSON, FormatAgent} {
		var out strings.Builder

		renderer := Renderer{Root: t.TempDir(), Mode: ModeComplexity}

		if err := renderer.Render(&out, format, Combined{Complexity: complexityReport()}); err != nil {
			t.Fatalf("render %s: %v", format, err)
		}

		if !strings.Contains(out.String(), `"complexity"`) || !strings.Contains(out.String(), `"result"`) {
			t.Fatalf("%s render = %s", format, out.String())
		}

		if strings.Contains(out.String(), `"formatter"`) {
			t.Fatalf("%s render carried a formatter section: %s", format, out.String())
		}
	}
}

// A formatting run never measured complexity, so its documents must keep the
// shape they have always had.
func TestRenderJSONOmitsAnAbsentComplexitySection(t *testing.T) {
	var out strings.Builder

	renderer := Renderer{Root: t.TempDir(), Mode: ModeCheck}

	if err := renderer.Render(&out, FormatJSON, Combined{}); err != nil {
		t.Fatalf("render: %v", err)
	}

	if strings.Contains(out.String(), "complexity") {
		t.Fatalf("render = %s", out.String())
	}
}
