package orchestrator

import (
	"fmt"
	"regexp"
	"strings"
)

// The summarizers distill a step's captured output into the aligned detail
// lines shown under its section header.

var (
	lintResultPattern     = regexp.MustCompile(`Found [0-9]+ warning|[0-9]+ error`)
	goFileSummaryPattern  = regexp.MustCompile(`^  (Formatted|Checked) [0-9]+ file\(s\)\.$|^  No Go files found\.$`)
	goVetSummaryPattern   = regexp.MustCompile(`^  go vet \./\.\.\. passed\.$|^  Skipped automatic go vet `)
	sourcesMissingPrefix  = "[sources] path not found, skipping:"
	blankLinesPrefix      = "[blank-lines] processed "
	fluentChainsPrefix    = "[fluent-chains] processed "
	oxfmtFinishedPrefix   = "Finished in "
	validateSyntaxPrefix  = "[validate-syntax] checked "
	lintNothingToLintLine = "[lint] no TS/Vue files to lint."
	goResultPrefix        = "  Result: "
)

func lines(log string) []string {
	return strings.Split(log, "\n")
}

func lastWithPrefix(log, prefix string) string {
	var match string

	for _, line := range lines(log) {
		if strings.HasPrefix(line, prefix) {
			match = line
		}
	}

	return match
}

func summarizeTSFormat(log string, l *logger) {
	missing := 0

	for _, line := range lines(log) {
		if strings.HasPrefix(line, sourcesMissingPrefix) {
			missing++
		}
	}

	if line := lastWithPrefix(log, blankLinesPrefix); line != "" {
		l.detail("blank-lines", strings.TrimPrefix(line, "[blank-lines] "))
	}

	if missing > 0 {
		l.detail("skipped", fmt.Sprintf("%d missing tracked file(s)", missing))
	}

	if line := lastWithPrefix(log, oxfmtFinishedPrefix); line != "" {
		l.detail("oxfmt", line)
	}

	if line := lastWithPrefix(log, fluentChainsPrefix); line != "" {
		l.detail("fluent", strings.TrimPrefix(line, "[fluent-chains] "))
	}

	if line := lastWithPrefix(log, validateSyntaxPrefix); line != "" {
		l.detail("validated", strings.TrimPrefix(line, "[validate-syntax] "))
	}
}

func summarizeTSLint(log string, l *logger) {
	if lastWithPrefix(log, lintNothingToLintLine) != "" {
		l.detail("oxlint", strings.TrimPrefix(lintNothingToLintLine, "[lint] "))

		return
	}

	var match string

	for _, line := range lines(log) {
		if lintResultPattern.MatchString(line) {
			match = line
		}
	}

	if match != "" {
		l.detail("oxlint", match)

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
