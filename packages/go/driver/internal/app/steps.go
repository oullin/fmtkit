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
	"go.ollin.sh/fmtkit/driver/internal/golang"
	"go.ollin.sh/fmtkit/driver/internal/pipeline"
	"go.ollin.sh/fmtkit/driver/internal/sidecarproto"
	"go.ollin.sh/fmtkit/driver/internal/tsruntime"
)

// stepSelection selects which parts of the format pipeline run; the zero value
// (no --ts/--go flags) runs everything.
type stepSelection struct {
	TS bool
	Go bool
}

// tsLintStep lints TS/Vue files, applying oxlint's safe fixes (--fix).
type tsLintStep struct {
	version   string
	paths     []string
	selection gitfiles.Selection
}

// tsFormatStep runs the full TS/Vue formatting pipeline (oxfmt plus the project
// passes).
type tsFormatStep struct {
	version   string
	paths     []string
	selection gitfiles.Selection
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
		steps = append(steps, golang.FormatStep(paths, selection))
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
