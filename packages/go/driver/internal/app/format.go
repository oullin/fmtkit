package app

import (
	"fmt"
	"io"

	"go.ollin.sh/fmtkit/driver/internal/cli"
	"go.ollin.sh/fmtkit/driver/internal/orchestrator"
	"go.ollin.sh/fmtkit/driver/internal/sourcefiles"
	"go.ollin.sh/fmtkit/driver/internal/tsruntime"
)

// runFormat formats what diverges from HEAD — modified files, staged or not,
// plus untracked ones — so an everyday format stays proportional to the diff.
// Use format-all to cover every file.
func (a App) runFormat(args []string) int {
	opts, paths, err := parseFormatArgs(args)

	if err != nil {
		_, _ = fmt.Fprintf(a.stderr, "%v\n\n", err)

		printUsage(a.stderr)

		return 2
	}

	return a.runPipeline(paths, opts, sourcefiles.SelectionChanged)
}

// runFormatAll covers every non-ignored file rather than just the working
// tree's changes, pinned to the current directory, so it takes flags but
// rejects paths.
func (a App) runFormatAll(args []string) int {
	opts, extra, err := parseFormatArgs(args)

	if err != nil || len(extra) != 0 {
		if err != nil {
			_, _ = fmt.Fprintf(a.stderr, "%v\n\n", err)
		}

		printUsage(a.stderr)

		return 2
	}

	return a.runPipeline([]string{"."}, opts, sourcefiles.SelectionAll)
}

func (a App) runPipeline(paths []string, opts formatOptions, selection sourcefiles.Selection) int {
	pipeline := orchestrator.Pipeline{
		Tools: orchestrator.Tools{
			TS: func(scopes []string, output io.Writer) error {
				support, err := tsruntime.Resolve(a.version)

				if err != nil {
					return err
				}

				return support.RunPipeline(tsruntime.RunOptions{Scopes: scopes, Selection: selection, Stdout: output, Stderr: output})
			},
			Lint: func(scopes []string, output io.Writer) error {
				support, err := tsruntime.Resolve(a.version)

				if err != nil {
					return err
				}

				return support.RunLint(tsruntime.RunOptions{Scopes: scopes, Selection: selection, Stdout: output, Stderr: output})
			},
			Go: func(args []string, output io.Writer) int {
				return cli.
					NewScopedRunner(output, output, selection).
					Run(cli.FormatMode, args[1:])
			},
		},
		Steps:  opts.steps,
		Quiet:  opts.quiet,
		Stderr: a.stderr,
	}

	return pipeline.RunFormat(paths)
}
