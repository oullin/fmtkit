// Package typescript is the TS/Vue lane: it lints (oxlint) and formats (the
// oxfmt pipeline plus the project passes) TS, Vue, HTML, and Markdown files,
// contributing the lint and format steps to the pipeline. The lane's machinery
// is split across subpackages — runtime (toolchain extraction and spawning),
// proto (the wire protocol), sourcefiles/filetypes/prettierignore (file
// discovery), and embedded (the assets baked into release binaries) — while
// this package builds the pipeline steps that drive them.
package typescript

import (
	"bytes"
	"context"
	"errors"
	"fmt"
	"io"
	"os/exec"
	"strings"

	"go.ollin.sh/fmtkit/driver/internal/gitfiles"
	"go.ollin.sh/fmtkit/driver/internal/pipeline"
	"go.ollin.sh/fmtkit/driver/internal/toolchain"
	"go.ollin.sh/fmtkit/driver/internal/typescript/proto"
	"go.ollin.sh/fmtkit/driver/internal/typescript/runtime"
)

// Toolchain is the TS/Vue lane.
type Toolchain struct{}

type lintStep struct {
	version   string
	paths     []string
	selection gitfiles.Selection
}

type formatStep struct {
	version   string
	paths     []string
	selection gitfiles.Selection
}

// New builds the TS toolchain.
func New() Toolchain { return Toolchain{} }

// Name is the lane's selector, matching the --ts flag.
func (Toolchain) Name() string { return "ts" }

// Steps returns the TS lane's ordered steps. Lint runs first so the formatting
// passes normalize whatever oxlint rewrites.
func (Toolchain) Steps(req toolchain.Request) []pipeline.Step {
	return []pipeline.Step{
		LintStep(req.Version, req.Paths, req.Selection),
		FormatStep(req.Version, req.Paths, req.Selection),
	}
}

// LintStep builds the step that lints TS/Vue files, applying oxlint's safe
// fixes (--fix).
func LintStep(version string, paths []string, selection gitfiles.Selection) pipeline.Step {
	return lintStep{version: version, paths: paths, selection: selection}
}

// FormatStep builds the step that runs the full TS/Vue formatting pipeline
// (oxfmt plus the project passes).
func FormatStep(version string, paths []string, selection gitfiles.Selection) pipeline.Step {
	return formatStep{version: version, paths: paths, selection: selection}
}

// Driver-owned bookkeeping lines the TS steps recognize in their captured
// output. The sidecar's own wire lines are parsed by the proto package; these
// are notices the Go driver prints around the sidecar, so they stay here.
const (
	sourcesMissingPrefix  = "[sources] path not found, skipping:"
	lintNothingToLintLine = "[lint] no TS/Vue files to lint."
)

func (s lintStep) Label() string { return "Running TS/Vue lint" }

func (s lintStep) Run(ctx context.Context, output io.Writer) pipeline.Result {
	var captured bytes.Buffer

	err := invoke(s.version, io.MultiWriter(output, &captured), func(invoker runtime.Invoker, w io.Writer) error {
		return invoker.RunLint(ctx, runtime.Request{Scopes: s.paths, Selection: s.selection, Fix: true, Stdout: w, Stderr: w})
	})

	if code := exitCode(err, output); code != 0 {
		return pipeline.Result{ExitCode: code}
	}

	return pipeline.Result{Details: lintDetails(captured.String())}
}

func (s formatStep) Label() string { return "Running TS/Vue formatting" }

func (s formatStep) Run(ctx context.Context, output io.Writer) pipeline.Result {
	var captured bytes.Buffer

	err := invoke(s.version, io.MultiWriter(output, &captured), func(invoker runtime.Invoker, w io.Writer) error {
		return invoker.RunPipeline(ctx, runtime.Request{Scopes: s.paths, Selection: s.selection, Stdout: w, Stderr: w})
	})

	if code := exitCode(err, output); code != 0 {
		return pipeline.Result{ExitCode: code}
	}

	return pipeline.Result{Details: formatDetails(captured.String())}
}

// invoke resolves the TS toolchain and invokes it through spawn, which receives
// the constructed Invoker and the writer to stream tool output to.
func invoke(version string, output io.Writer, spawn func(runtime.Invoker, io.Writer) error) error {
	assets, err := runtime.Resolve(version)

	if err != nil {
		return err
	}

	return spawn(runtime.NewInvoker(assets), output)
}

// exitCode maps a TS step error to its exit code. Failures that never produced
// tool output (a missing sidecar, an unreadable working tree) surface their
// message through output so they are visible both live and in the quiet failure
// dump.
func exitCode(err error, output io.Writer) int {
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

// lintDetails derives the oxlint summary line. A driver "no files" notice wins;
// otherwise oxlint's own result line; otherwise a clean fallback.
func lintDetails(log string) []pipeline.Detail {
	for _, line := range strings.Split(log, "\n") {
		if strings.HasPrefix(line, lintNothingToLintLine) {
			return []pipeline.Detail{{Label: "oxlint", Value: strings.TrimPrefix(lintNothingToLintLine, "[lint] ")}}
		}
	}

	if result := proto.ParseLintSummary(log).Result; result != "" {
		return []pipeline.Detail{{Label: "oxlint", Value: result}}
	}

	return []pipeline.Detail{{Label: "oxlint", Value: "no issues found"}}
}

// formatDetails derives the TS pipeline's detail lines from the sidecar's
// progress output plus the driver's missing-source notices.
func formatDetails(log string) []pipeline.Detail {
	summary := proto.ParsePipelineSummary(log)

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
