package app

import (
	"context"
	"fmt"
	"io"

	"go.ollin.sh/fmtkit/driver/internal/gitfiles"
	"go.ollin.sh/fmtkit/driver/internal/gotool"
	"go.ollin.sh/fmtkit/driver/internal/orchestrator"
	"go.ollin.sh/fmtkit/driver/internal/tsruntime"
	report "go.ollin.sh/fmtkit/driver/report"
)

// runFormat formats what diverges from HEAD — modified files, staged or not,
// plus untracked ones — so an everyday format stays proportional to the diff.
// Use format-all to cover every file.
func (d *deps) runFormat(ctx context.Context, args []string) int {
	opts, paths, err := parseFormatArgs(args)

	if err != nil {
		_, _ = fmt.Fprintf(d.stderr, "%v\n\n", err)

		d.usage(d.stderr)

		return 2
	}

	return d.runPipeline(ctx, paths, opts, gitfiles.SelectionChanged)
}

// runFormatAll covers every non-ignored file rather than just the working
// tree's changes, pinned to the current directory, so it takes flags but
// rejects paths.
func (d *deps) runFormatAll(ctx context.Context, args []string) int {
	opts, extra, err := parseFormatArgs(args)

	if err != nil || len(extra) != 0 {
		if err != nil {
			_, _ = fmt.Fprintf(d.stderr, "%v\n\n", err)
		}

		d.usage(d.stderr)

		return 2
	}

	return d.runPipeline(ctx, []string{"."}, opts, gitfiles.SelectionAll)
}

func (d *deps) runPipeline(ctx context.Context, paths []string, opts formatOptions, selection gitfiles.Selection) int {
	pipeline := orchestrator.Pipeline{
		Tools: orchestrator.Tools{
			TS: func(ctx context.Context, scopes []string, output io.Writer) error {
				assets, err := tsruntime.Resolve(d.version)

				if err != nil {
					return err
				}

				return tsruntime.NewInvoker(assets).RunPipeline(ctx, tsruntime.Request{Scopes: scopes, Selection: selection, Stdout: output, Stderr: output})
			},
			Lint: func(ctx context.Context, scopes []string, output io.Writer) error {
				assets, err := tsruntime.Resolve(d.version)

				if err != nil {
					return err
				}

				return tsruntime.NewInvoker(assets).RunLint(ctx, tsruntime.Request{Scopes: scopes, Selection: selection, Fix: true, Stdout: output, Stderr: output})
			},
			Go: func(ctx context.Context, args []string, output io.Writer) int {
				return gotool.
					Runner{Stdout: output, Stderr: output, Scope: selection}.
					Run(ctx, report.ModeFormat, args[1:])
			},
		},
		Steps:  opts.steps,
		Quiet:  opts.quiet,
		Stderr: d.stderr,
	}

	return pipeline.RunFormat(ctx, paths)
}
