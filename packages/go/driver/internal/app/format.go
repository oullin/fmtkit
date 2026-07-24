package app

import (
	"context"
	"fmt"
	"strings"

	"go.ollin.sh/fmtkit/driver/internal/console"
	"go.ollin.sh/fmtkit/driver/internal/gitfiles"
	"go.ollin.sh/fmtkit/driver/internal/orchestrator"
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

// runPipeline frames the format run (target header, completion footer) around
// the typed steps it builds for the selection, handing them to the generic
// orchestrator. Color is resolved once here, at the composition root.
func (d *deps) runPipeline(ctx context.Context, paths []string, opts formatOptions, selection gitfiles.Selection) int {
	if len(paths) == 0 {
		paths = []string{"."}
	}

	printer := console.NewPrinter(d.stderr, console.DetectColor(d.stderr))

	printer.Section("Formatting target(s)")
	printer.Detail("paths", strings.Join(paths, " "))

	pipeline := orchestrator.Pipeline{
		Steps:   d.formatSteps(paths, opts.steps, selection),
		Quiet:   opts.quiet,
		Printer: printer,
		Stderr:  d.stderr,
	}

	if code := pipeline.Run(ctx); code != 0 {
		return code
	}

	printer.Section("Formatting complete")
	printer.SuccessDetail("status", "done")

	return 0
}
