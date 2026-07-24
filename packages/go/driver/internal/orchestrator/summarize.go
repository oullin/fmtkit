package orchestrator

import (
	"fmt"
	"regexp"
	"strings"

	"go.ollin.sh/fmtkit/driver/internal/sidecarproto"
)

// The summarizers distill a step's captured output into the aligned detail
// lines shown under its section header. Lines the TS sidecar emits are parsed
// by sidecarproto; the Go-report scraping below stays here (G6 retires it).

var (
	goFileSummaryPattern  = regexp.MustCompile(`^  (Formatted|Checked) [0-9]+ file\(s\)\.$|^  No Go files found\.$`)
	goVetSummaryPattern   = regexp.MustCompile(`^  go vet \./\.\.\. passed\.$|^  Skipped automatic go vet `)
	sourcesMissingPrefix  = "[sources] path not found, skipping:"
	lintNothingToLintLine = "[lint] no TS/Vue files to lint."
	goResultPrefix        = "  Result: "
)

func lines(log string) []string {
	return strings.Split(log, "\n")
}

func lastWithPrefix(logLines []string, prefix string) string {
	var match string

	for _, line := range logLines {
		if strings.HasPrefix(line, prefix) {
			match = line
		}
	}

	return match
}

func summarizeTSFormat(log string, l *logger) {
	summary := sidecarproto.ParsePipelineSummary(log)
	missing := 0

	for _, line := range lines(log) {
		if strings.HasPrefix(line, sourcesMissingPrefix) {
			missing++
		}
	}

	if summary.BlankLines != "" {
		l.detail("blank-lines", summary.BlankLines)
	}

	if missing > 0 {
		l.detail("skipped", fmt.Sprintf("%d missing tracked file(s)", missing))
	}

	if summary.Oxfmt != "" {
		l.detail("oxfmt", summary.Oxfmt)
	}

	if summary.FluentChains != "" {
		l.detail("fluent", summary.FluentChains)
	}

	if summary.ValidateSyntax != "" {
		l.detail("validated", summary.ValidateSyntax)
	}
}

func summarizeTSLint(log string, l *logger) {
	if lastWithPrefix(lines(log), lintNothingToLintLine) != "" {
		l.detail("oxlint", strings.TrimPrefix(lintNothingToLintLine, "[lint] "))

		return
	}

	if result := sidecarproto.ParseLintSummary(log).Result; result != "" {
		l.detail("oxlint", result)

		return
	}

	l.detail("oxlint", "no issues found")
}

func summarizeGoFormat(log string, l *logger) {
	var fileSummary, formatterResult, vetSummary, vetResult string

	for _, line := range lines(log) {
		if fileSummary == "" && goFileSummaryPattern.MatchString(line) {
			fileSummary = strings.TrimPrefix(line, "  ")
		}

		if vetSummary == "" && goVetSummaryPattern.MatchString(line) {
			vetSummary = strings.TrimPrefix(line, "  ")
		}

		if strings.HasPrefix(line, goResultPrefix) {
			if formatterResult == "" {
				formatterResult = strings.TrimPrefix(line, goResultPrefix)
			}

			vetResult = strings.TrimPrefix(line, goResultPrefix)
		}
	}

	if fileSummary != "" {
		l.detail("fmtkit", fileSummary)
	}

	if formatterResult != "" {
		l.detail("result", formatterResult)
	}

	if vetSummary != "" {
		l.detail("vet", vetSummary)
	}

	if vetResult != "" && vetResult != formatterResult {
		l.detail("vet result", vetResult)
	}
}
