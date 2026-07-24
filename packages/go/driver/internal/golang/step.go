package golang

import (
	"context"
	"fmt"
	"io"

	"go.ollin.sh/fmtkit/driver/internal/gitfiles"
	"go.ollin.sh/fmtkit/driver/internal/pipeline"
	"go.ollin.sh/fmtkit/driver/internal/toolchain"
	report "go.ollin.sh/fmtkit/driver/report"
	formatterengine "go.ollin.sh/fmtkit/formatter/engine"
	"go.ollin.sh/fmtkit/vet"
)

// Toolchain is the Go lane: it formats Go files and runs go vet, contributing a
// single format step to the pipeline.
type Toolchain struct{}

type formatStep struct {
	paths     []string
	selection gitfiles.Selection
}

// New builds the Go toolchain.
func New() Toolchain { return Toolchain{} }

// Name is the lane's selector, matching the --go flag.
func (Toolchain) Name() string { return "go" }

// Steps returns the Go lane's ordered steps: just the format step.
func (Toolchain) Steps(req toolchain.Request) []pipeline.Step {
	return []pipeline.Step{FormatStep(req.Paths, req.Selection)}
}

// FormatStep builds the pipeline step that formats Go files and runs go vet,
// deriving its details from the typed outcome rather than the rendered report
// text.
func FormatStep(paths []string, selection gitfiles.Selection) pipeline.Step {
	return formatStep{paths: paths, selection: selection}
}

func (s formatStep) Label() string { return "Running Go formatting" }

func (s formatStep) Run(ctx context.Context, output io.Writer) pipeline.Result {
	outcome, code := Runner{Stdout: output, Stderr: output, Scope: s.selection}.
		RunReport(ctx, report.ModeFormat, s.paths)

	if code != 0 {
		return pipeline.Result{ExitCode: code}
	}

	return pipeline.Result{Details: formatDetails(outcome)}
}

// formatDetails computes the Go step's detail lines from the typed outcome,
// reproducing the exact strings the text report renders (which the pipeline
// previously scraped back out of that rendered text).
func formatDetails(outcome Outcome) []pipeline.Detail {
	fm := outcome.Combined.Formatter
	vt := outcome.Combined.Vet

	var details []pipeline.Detail

	if summary := fileSummary(fm, outcome.Mode); summary != "" {
		details = append(details, pipeline.Detail{Label: "fmtkit", Value: summary})
	}

	// The formatter renders a Result line unless it found no files and hit no
	// errors; the vet Result line always renders. The "result" detail is the
	// first Result line (the formatter's when present, else the vet's), matching
	// the text report's top-to-bottom order.
	formatterResult := ""

	if fm.Files != 0 || len(fm.Errors) != 0 {
		formatterResult = fmt.Sprintf("%s. %d changed, %d violation(s), %d error(s).", fm.Result, fm.Changed, fm.ViolationCount(), fm.ErrorCount())
	}

	vetResult := fmt.Sprintf("%s. %d error(s).", vetStatus(vt), vt.ErrorCount())

	resultLine := formatterResult

	if resultLine == "" {
		resultLine = vetResult
	}

	details = append(details, pipeline.Detail{Label: "result", Value: resultLine})

	if summary := vetSummary(vt); summary != "" {
		details = append(details, pipeline.Detail{Label: "vet", Value: summary})
	}

	if vetResult != resultLine {
		details = append(details, pipeline.Detail{Label: "vet result", Value: vetResult})
	}

	return details
}

// fileSummary is the formatter's file-count line: "No Go files found." when it
// owns none, otherwise the mode's verb and count.
func fileSummary(fm formatterengine.Report, mode report.Mode) string {
	if fm.Files == 0 {
		return "No Go files found."
	}

	action := "Checked"

	if mode == report.ModeFormat {
		action = "Formatted"
	}

	return fmt.Sprintf("%s %d file(s).", action, fm.Files)
}

// vetStatus classifies the vet report the same way the text report does.
func vetStatus(vt vet.Report) string {
	switch {
	case vt.Skipped || vt.Root == "":
		return "skipped"
	case vt.ErrorCount() > 0:
		return "fail"
	default:
		return "pass"
	}
}

// vetSummary is the vet status line, or "" for a failure (whose per-error lines
// the text report shows instead of a one-line summary).
func vetSummary(vt vet.Report) string {
	switch vetStatus(vt) {
	case "skipped":
		reason := "no Go module or workspace was detected"

		if vt.Skipped {
			reason = "the Go toolchain is not available"
		}

		return "Skipped automatic go vet ./... because " + reason + "."
	case "pass":
		return "go vet ./... passed."
	default:
		return ""
	}
}
