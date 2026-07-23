package app

import (
	"context"
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
func (a App) runFormat(ctx context.Context, args []string) int {
	opts, paths, err := parseFormatArgs(args)

	if err != nil {
		_, _ = fmt.Fprintf(a.stderr, "%v\n\n", err)

		printUsage(a.stderr)

		return 2
	}

	return a.runPipeline(ctx, paths, opts, sourcefiles.SelectionChanged)
}

// runFormatAll covers every non-ignored file rather than just the working
// tree's changes, pinned to the current directory, so it takes flags but
// rejects paths.
func (a App) runFormatAll(ctx context.Context, args []string) int {
	opts, extra, err := parseFormatArgs(args)

	if err != nil || len(extra) != 0 {
		if err != nil {
			_, _ = fmt.Fprintf(a.stderr, "%v\n\n", err)
		}

		printUsage(a.stderr)

		return 2
	}

	return a.runPipeline(ctx, []string{"."}, opts, sourcefiles.SelectionAll)
}

func (a App) runPipeline(ctx context.Context, paths []string, opts formatOptions, selection sourcefiles.Selection) int {
	pipeline := orchestrator.Pipeline{
		Tools: orchestrator.Tools{
			TS: func(ctx context.Context, scopes []string, output io.Writer) error {
				support, err := tsruntime.Resolve(a.version)

				if err != nil {
					return err
				}

				return support.RunPipeline(ctx, tsruntime.RunOptions{Scopes: scopes, Selection: selection, Stdout: output, Stderr: output})
			},
			Lint: func(ctx context.Context, scopes []string, output io.Writer) error {
				support, err := tsruntime.Resolve(a.version)

				if err != nil {
					return err
				}

				return support.RunLint(ctx, tsruntime.RunOptions{Scopes: scopes, Selection: selection, Fix: true, Stdout: output, Stderr: output})
			},
			Go: func(ctx context.Context, args []string, output io.Writer) int {
				return cli.
					NewScopedRunner(output, output, selection).
					Run(ctx, cli.FormatMode, args[1:])
			},
		},
		Steps:  opts.steps,
		Quiet:  opts.quiet,
		Stderr: a.stderr,
	}

	return pipeline.RunFormat(ctx, paths)
}
