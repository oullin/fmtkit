package orchestrator

import (
	"bytes"
	"errors"
	"io"
	"os/exec"
	"strings"
)

// Tools carries the three pipeline steps. The TS steps return an error whose
// exec.ExitError code propagates; the Go step reports its exit code directly.
type Tools struct {
	TS   func(scopes []string, output io.Writer) error
	Lint func(scopes []string, output io.Writer) error
	Go   func(args []string, output io.Writer) int
}

// Pipeline renders sectioned progress on Stderr while running the steps.
type Pipeline struct {
	Tools Tools

	// Quiet restores the entrypoint's summary-only output; tool logs then
	// only appear when a step fails.
	Quiet bool

	Stderr io.Writer
}

// RunFormat runs TS/Vue formatting, TS/Vue lint, and Go formatting against
// the given paths, the Go port of run_format_pipeline in infra/bin/fmtkit.
func (p Pipeline) RunFormat(paths []string) int {
	if len(paths) == 0 {
		paths = []string{"."}
	}

	log := newLogger(p.Stderr, p.Quiet)

	log.section("Formatting target(s)")
	log.detail("paths", strings.Join(paths, " "))

	steps := []struct {
		label     string
		summarize func(string, *logger)
		run       func(output io.Writer) int
	}{
		{
			label:     "Running TS/Vue formatting",
			summarize: summarizeTSFormat,
			run: func(output io.Writer) int {
				return exitCode(p.Tools.TS(paths, output), output)
			},
		},
		{
			label:     "Running TS/Vue lint",
			summarize: summarizeTSLint,
			run: func(output io.Writer) int {
				return exitCode(p.Tools.Lint(paths, output), output)
			},
		},
		{
			label:     "Running Go formatting",
			summarize: summarizeGoFormat,
			run: func(output io.Writer) int {
				return p.Tools.Go(append([]string{"format"}, paths...), output)
			},
		},
	}

	for _, step := range steps {
		if code := p.runStep(log, step.label, step.summarize, step.run); code != 0 {
			return code
		}
	}

	log.section("Formatting complete")
	log.successDetail("status", "done")

	return 0
}

// runStep captures a step's combined output, streaming it live unless quiet,
// and prints either its summary details or (on failure) the captured log.
func (p Pipeline) runStep(log *logger, label string, summarize func(string, *logger), run func(io.Writer) int) int {
	log.section(label)

	var captured bytes.Buffer

	output := io.Writer(&captured)

	var live io.WriteCloser

	if !p.Quiet {
		live = log.stream()
		output = io.MultiWriter(&captured, live)
	}

	code := run(output)

	if live != nil {
		_ = live.Close()
	}

	if code != 0 {
		log.failure(label + " failed")

		if p.Quiet {
			_, _ = io.Copy(p.Stderr, bytes.NewReader(captured.Bytes()))
		}

		return code
	}

	summarize(captured.String(), log)

	return 0
}

// exitCode maps a step error to its exit code. Failures that never produced
// tool output (a missing sidecar, an unreadable working tree) surface their
// message through the step's output writer so they are visible both live and
// in the failure dump.
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
