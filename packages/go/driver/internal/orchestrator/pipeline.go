// Package orchestrator drives a sequence of typed pipeline steps, rendering
// sectioned, colorized progress: each step's tool output streams live, indented
// under its section header, followed by the condensed detail lines the step
// derives from its typed result. It owns only the section/tee/quiet-failure-dump
// mechanics; the concrete steps (and their detail computation) live with the
// composition root that builds them.
package orchestrator

import (
	"bytes"
	"context"
	"io"

	"go.ollin.sh/fmtkit/driver/internal/console"
)

// Detail is one aligned label/value line shown under a step's section header.
type Detail struct {
	Label string
	Value string
}

// Result is what a Step reports: the process exit code it wants (0 on success)
// and, on success, the detail lines to render under its section.
type Result struct {
	ExitCode int
	Details  []Detail
}

// Step is one unit of pipeline work. Label is the section header; Run writes the
// tool's live output to output (a tee of the live stream and, when a step needs
// it, its own capture) and returns the typed Result.
type Step interface {
	Label() string
	Run(ctx context.Context, output io.Writer) Result
}

// Pipeline renders sectioned progress on Stderr while running the steps in
// order, short-circuiting on the first non-zero exit code.
type Pipeline struct {
	Steps []Step

	// Quiet restores the summary-only output; a step's live tool log then only
	// appears when it fails.
	Quiet bool

	// Printer renders every section, detail, and failure banner. The caller
	// constructs it once (resolving color at the boundary) and shares it.
	Printer *console.Printer

	Stderr io.Writer
}

// Run executes the steps in order, returning the first non-zero exit code or 0
// when they all pass.
func (p Pipeline) Run(ctx context.Context) int {
	for _, step := range p.Steps {
		if code := p.runStep(ctx, step); code != 0 {
			return code
		}
	}

	return 0
}

// runStep captures a step's combined output, streaming it live unless quiet,
// and prints either the step's detail lines or (on failure) the captured log.
func (p Pipeline) runStep(ctx context.Context, step Step) int {
	p.Printer.Section(step.Label())

	var captured bytes.Buffer

	output := io.Writer(&captured)

	var live io.WriteCloser

	if !p.Quiet {
		live = p.Printer.Stream()
		output = io.MultiWriter(&captured, live)
	}

	result := step.Run(ctx, output)

	if live != nil {
		_ = live.Close()
	}

	if result.ExitCode != 0 {
		p.Printer.Failure(step.Label() + " failed")

		if p.Quiet {
			_, _ = io.Copy(p.Stderr, bytes.NewReader(captured.Bytes()))
		}

		return result.ExitCode
	}

	for _, detail := range result.Details {
		p.Printer.Detail(detail.Label, detail.Value)
	}

	return 0
}
