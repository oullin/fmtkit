package app

import (
	"bytes"
	"context"
	"errors"
	"fmt"
	"io"
	"os/exec"
	"strings"

	"go.ollin.sh/fmtkit/driver/internal/gitfiles"
	"go.ollin.sh/fmtkit/driver/internal/gotool"
	"go.ollin.sh/fmtkit/driver/internal/pipeline"
	"go.ollin.sh/fmtkit/driver/internal/sidecarproto"
	"go.ollin.sh/fmtkit/driver/internal/tsruntime"
	report "go.ollin.sh/fmtkit/driver/report"
	formatterengine "go.ollin.sh/fmtkit/formatter/engine"
	"go.ollin.sh/fmtkit/vet"
)

// stepSelection selects which parts of the format pipeline run; the zero value
// (no --ts/--go flags) runs everything.
type stepSelection struct {
	TS bool
	Go bool
}

func (s stepSelection) normalized() stepSelection {
	if !s.TS && !s.Go {
		return stepSelection{TS: true, Go: true}
	}

	return s
}

// formatSteps builds the ordered pipeline steps for the selection. Lint runs
// first so the formatting passes normalize whatever oxlint rewrites.
func (d *deps) formatSteps(paths []string, selected stepSelection, selection gitfiles.Selection) []pipeline.Step {
	selected = selected.normalized()

	var steps []pipeline.Step

	if selected.TS {
		steps = append(steps,
			tsLintStep{version: d.version, paths: paths, selection: selection},
			tsFormatStep{version: d.version, paths: paths, selection: selection},
		)
	}

	if selected.Go {
		steps = append(steps, goFormatStep{paths: paths, selection: selection})
	}

	return steps
}

// Driver-owned bookkeeping lines the TS steps recognize in their captured
// output. The sidecar's own wire lines are parsed by sidecarproto; these are
// notices the Go driver prints around the sidecar, so they stay here.
const (
	sourcesMissingPrefix  = "[sources] path not found, skipping:"
	lintNothingToLintLine = "[lint] no TS/Vue files to lint."
)

// tsLintStep lints TS/Vue files, applying oxlint's safe fixes (--fix).
type tsLintStep struct {
	version   string
	paths     []string
	selection gitfiles.Selection
}

func (s tsLintStep) Label() string { return "Running TS/Vue lint" }

func (s tsLintStep) Run(ctx context.Context, output io.Writer) pipeline.Result {
	var captured bytes.Buffer

	err := invokeTS(s.version, io.MultiWriter(output, &captured), func(invoker tsruntime.Invoker, w io.Writer) error {
		return invoker.RunLint(ctx, tsruntime.Request{Scopes: s.paths, Selection: s.selection, Fix: true, Stdout: w, Stderr: w})
	})

	if code := tsExitCode(err, output); code != 0 {
		return pipeline.Result{ExitCode: code}
	}

	return pipeline.Result{Details: tsLintDetails(captured.String())}
}

// tsFormatStep runs the full TS/Vue formatting pipeline (oxfmt plus the project
// passes).
type tsFormatStep struct {
	version   string
	paths     []string
	selection gitfiles.Selection
}

func (s tsFormatStep) Label() string { return "Running TS/Vue formatting" }

func (s tsFormatStep) Run(ctx context.Context, output io.Writer) pipeline.Result {
	var captured bytes.Buffer

	err := invokeTS(s.version, io.MultiWriter(output, &captured), func(invoker tsruntime.Invoker, w io.Writer) error {
		return invoker.RunPipeline(ctx, tsruntime.Request{Scopes: s.paths, Selection: s.selection, Stdout: w, Stderr: w})
	})

	if code := tsExitCode(err, output); code != 0 {
		return pipeline.Result{ExitCode: code}
	}

	return pipeline.Result{Details: tsFormatDetails(captured.String())}
}

// goFormatStep formats Go files and runs go vet, deriving its details from the
// typed outcome rather than the rendered report text.
type goFormatStep struct {
	paths     []string
	selection gitfiles.Selection
}

func (s goFormatStep) Label() string { return "Running Go formatting" }

func (s goFormatStep) Run(ctx context.Context, output io.Writer) pipeline.Result {
	outcome, code := gotool.
		Runner{Stdout: output, Stderr: output, Scope: s.selection}.
		RunReport(ctx, report.ModeFormat, s.paths)

	if code != 0 {
		return pipeline.Result{ExitCode: code}
	}

	return pipeline.Result{Details: goFormatDetails(outcome)}
}

// invokeTS resolves the TS toolchain and invokes it through spawn, which
// receives the constructed Invoker and the writer to stream tool output to.
func invokeTS(version string, output io.Writer, spawn func(tsruntime.Invoker, io.Writer) error) error {
	assets, err := tsruntime.Resolve(version)

	if err != nil {
		return err
	}

	return spawn(tsruntime.NewInvoker(assets), output)
}

// tsExitCode maps a TS step error to its exit code. Failures that never
// produced tool output (a missing sidecar, an unreadable working tree) surface
// their message through output so they are visible both live and in the quiet
// failure dump.
func tsExitCode(err error, output io.Writer) int {
	if err == nil {
		return 0
	}

	var exit *exec.ExitError

	if errors.As(err, &exit) {
		return exit.ExitCode()
	}

	_, _ = io.WriteString(output, err.Error()+"\n")

	return 1
}

// tsLintDetails derives the oxlint summary line. A driver "no files" notice
// wins; otherwise oxlint's own result line; otherwise a clean fallback.
func tsLintDetails(log string) []pipeline.Detail {
	for _, line := range strings.Split(log, "\n") {
		if strings.HasPrefix(line, lintNothingToLintLine) {
			return []pipeline.Detail{{Label: "oxlint", Value: strings.TrimPrefix(lintNothingToLintLine, "[lint] ")}}
		}
	}

	if result := sidecarproto.ParseLintSummary(log).Result; result != "" {
		return []pipeline.Detail{{Label: "oxlint", Value: result}}
	}

	return []pipeline.Detail{{Label: "oxlint", Value: "no issues found"}}
}

// tsFormatDetails derives the TS pipeline's detail lines from the sidecar's
// progress output plus the driver's missing-source notices.
func tsFormatDetails(log string) []pipeline.Detail {
	summary := sidecarproto.ParsePipelineSummary(log)

	var details []pipeline.Detail

	if summary.BlankLines != "" {
		details = append(details, pipeline.Detail{Label: "blank-lines", Value: summary.BlankLines})
	}

	missing := 0

	for _, line := range strings.Split(log, "\n") {
		if strings.HasPrefix(line, sourcesMissingPrefix) {
			missing++
		}
	}

	if missing > 0 {
		details = append(details, pipeline.Detail{Label: "skipped", Value: fmt.Sprintf("%d missing tracked file(s)", missing)})
	}

	if summary.Oxfmt != "" {
		details = append(details, pipeline.Detail{Label: "oxfmt", Value: summary.Oxfmt})
	}

	if summary.FluentChains != "" {
		details = append(details, pipeline.Detail{Label: "fluent", Value: summary.FluentChains})
	}

	if summary.ValidateSyntax != "" {
		details = append(details, pipeline.Detail{Label: "validated", Value: summary.ValidateSyntax})
	}

	return details
}

// goFormatDetails computes the Go step's detail lines from the typed outcome,
// reproducing the exact strings the text report renders (which the pipeline
// previously scraped back out of that rendered text).
func goFormatDetails(outcome gotool.Outcome) []pipeline.Detail {
	fm := outcome.Combined.Formatter
	vt := outcome.Combined.Vet

	var details []pipeline.Detail

	if summary := goFileSummary(fm, outcome.Mode); summary != "" {
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

	vetResult := fmt.Sprintf("%s. %d error(s).", goVetStatus(vt), vt.ErrorCount())

	resultLine := formatterResult

	if resultLine == "" {
		resultLine = vetResult
	}

	details = append(details, pipeline.Detail{Label: "result", Value: resultLine})

	if summary := goVetSummary(vt); summary != "" {
		details = append(details, pipeline.Detail{Label: "vet", Value: summary})
	}

	if vetResult != resultLine {
		details = append(details, pipeline.Detail{Label: "vet result", Value: vetResult})
	}

	return details
}

// goFileSummary is the formatter's file-count line: "No Go files found." when it
// owns none, otherwise the mode's verb and count.
func goFileSummary(fm formatterengine.Report, mode report.Mode) string {
	if fm.Files == 0 {
		return "No Go files found."
	}

	action := "Checked"

	if mode == report.ModeFormat {
		action = "Formatted"
	}

	return fmt.Sprintf("%s %d file(s).", action, fm.Files)
}

// goVetStatus classifies the vet report the same way the text report does.
func goVetStatus(vt vet.Report) string {
	switch {
	case vt.Skipped || vt.Root == "":
		return "skipped"
	case vt.ErrorCount() > 0:
		return "fail"
	default:
		return "pass"
	}
}

// goVetSummary is the vet status line, or "" for a failure (whose per-error
// lines the text report shows instead of a one-line summary).
func goVetSummary(vt vet.Report) string {
	switch goVetStatus(vt) {
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
