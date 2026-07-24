package sidecarproto

import (
	"regexp"
	"strings"
)

// PipelineSummary holds the sidecar's pipeline progress, each field carrying the
// detail text the caller shows (already stripped of its scrape prefix, except
// Oxfmt which is oxfmt's own full line). Empty fields mean the sidecar printed
// no such line.
type PipelineSummary struct {
	// BlankLines is the last "[blank-lines] processed ..." line, without its
	// "[blank-lines] " prefix.
	BlankLines string

	// Oxfmt is the last "Finished in ..." line oxfmt printed, verbatim.
	Oxfmt string

	// FluentChains is the last "[fluent-chains] processed ..." line, without its
	// "[fluent-chains] " prefix.
	FluentChains string

	// ValidateSyntax is the last "[validate-syntax] checked ..." line, without
	// its "[validate-syntax] " prefix.
	ValidateSyntax string
}

// LintSummary holds the sidecar's oxlint result line.
type LintSummary struct {
	// Result is the last line matching oxlint's summary pattern, verbatim, or
	// "" when the log carries none.
	Result string
}

// The sidecar prints progress lines the driver scrapes into a step summary.
// These prefixes and the oxlint result pattern are the sidecar's output
// contract; only the lines the TS toolchain itself emits live here. Lines the
// Go driver prints about its own bookkeeping (source-collection warnings, the
// no-files notice, the Go formatter report) stay with the orchestrator.
const (
	blankLinesMatch = "[blank-lines] processed "
	blankLinesTrim  = "[blank-lines] "

	oxfmtFinishedMatch = "Finished in "

	fluentChainsMatch = "[fluent-chains] processed "
	fluentChainsTrim  = "[fluent-chains] "

	validateSyntaxMatch = "[validate-syntax] checked "
	validateSyntaxTrim  = "[validate-syntax] "
)

// lintResultPattern matches oxlint's summary line, e.g.
// "Found 0 warnings and 0 errors."
var lintResultPattern = regexp.MustCompile(`Found [0-9]+ warning|[0-9]+ error`)

// ParsePipelineSummary scrapes the sidecar's pipeline output, taking the last
// occurrence of each progress line.
func ParsePipelineSummary(log string) PipelineSummary {
	logLines := lines(log)

	summary := PipelineSummary{}

	if line := lastWithPrefix(logLines, blankLinesMatch); line != "" {
		summary.BlankLines = strings.TrimPrefix(line, blankLinesTrim)
	}

	if line := lastWithPrefix(logLines, oxfmtFinishedMatch); line != "" {
		summary.Oxfmt = line
	}

	if line := lastWithPrefix(logLines, fluentChainsMatch); line != "" {
		summary.FluentChains = strings.TrimPrefix(line, fluentChainsTrim)
	}

	if line := lastWithPrefix(logLines, validateSyntaxMatch); line != "" {
		summary.ValidateSyntax = strings.TrimPrefix(line, validateSyntaxTrim)
	}

	return summary
}

// ParseLintSummary scrapes the sidecar's oxlint output, taking the last line
// matching oxlint's summary pattern.
func ParseLintSummary(log string) LintSummary {
	var match string

	for _, line := range lines(log) {
		if lintResultPattern.MatchString(line) {
			match = line
		}
	}

	return LintSummary{Result: match}
}

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
