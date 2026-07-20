package orchestrator

import (
	"bytes"
	"context"
	"errors"
	"io"
	"os/exec"
	"strings"
)

// Tools carries the three pipeline steps. The TS steps return an error whose
// exec.ExitError code propagates; the Go step reports its exit code directly.
type Tools struct {
	TS   func(ctx context.Context, scopes []string, output io.Writer) error
	Lint func(ctx context.Context, scopes []string, output io.Writer) error
	Go   func(ctx context.Context, args []string, output io.Writer) int
}

// Steps selects which parts of the pipeline run; the zero value (no
// selection flags) runs everything.
type Steps struct {
	TS bool
	Go bool
}

// Pipeline renders sectioned progress on Stderr while running the steps.
type Pipeline struct {
	Tools Tools
	Steps Steps

	// Quiet restores the entrypoint's summary-only output; tool logs then
	// only appear when a step fails.
	Quiet bool

	Stderr io.Writer
}

func (s Steps) normalized() Steps {
	if !s.TS && !s.Go {
		return Steps{TS: true, Go: true}
	}

	return s
}

// RunFormat runs TS/Vue formatting, TS/Vue lint, and Go formatting against
// the given paths.
func (p Pipeline) RunFormat(ctx context.Context, paths []string) int {
	if len(paths) == 0 {
		paths = []string{"."}
	}

	log := newLogger(p.Stderr, p.Quiet)

	log.section("Formatting target(s)")
	log.detail("paths", strings.Join(paths, " "))

	selected := p.Steps.normalized()

	type step struct {
		label     string
		summarize func(string, *logger)
		run       func(ctx context.Context, output io.Writer) int
	}

	var steps []step

	if selected.TS {
		steps = append(steps,
			step{
				label:     "Running TS/Vue formatting",
				summarize: summarizeTSFormat,
				run: func(ctx context.Context, output io.Writer) int {
					return exitCode(p.Tools.TS(ctx, paths, output), output)
				},
			},
			step{
				label:     "Running TS/Vue lint",
				summarize: summarizeTSLint,
				run: func(ctx context.Context, output io.Writer) int {
					return exitCode(p.Tools.Lint(ctx, paths, output), output)
				},
			},
		)
	}

	if selected.Go {
		steps = append(steps, step{
			label:     "Running Go formatting",
			summarize: summarizeGoFormat,
			run: func(ctx context.Context, output io.Writer) int {
				return p.Tools.Go(ctx, append([]string{"format"}, paths...), output)
			},
		})
	}

	for _, step := range steps {
		if code := p.runStep(ctx, log, step.label, step.summarize, step.run); code != 0 {
			return code
		}
	}

	log.section("Formatting complete")
	log.successDetail("status", "done")

	return 0
}

// runStep captures a step's combined output, streaming it live unless quiet,
// and prints either its summary details or (on failure) the captured log.
func (p Pipeline) runStep(ctx context.Context, log *logger, label string, summarize func(string, *logger), run func(context.Context, io.Writer) int) int {
	log.section(label)

	var captured bytes.Buffer

	output := io.Writer(&captured)

	var live io.WriteCloser

	if !p.Quiet {
		live = log.stream()
		output = io.MultiWriter(&captured, live)
	}

	code := run(ctx, output)

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
