package golang

import (
	"context"
	"fmt"
	"io"
	"os"
	"strings"

	driverconfig "go.ollin.sh/fmtkit/driver/config"
	"go.ollin.sh/fmtkit/driver/internal/gitfiles"
	report "go.ollin.sh/fmtkit/driver/report"
)

// Runner is the thin orchestration around Execute: it parses the command line,
// loads config, runs the core, renders the report, and returns the exit code.
//
// Scope is how much of the working tree the formatter covers. The zero value
// (SelectionAll) covers everything, which is what `fmtkit go` and `fmtkit
// check` want; a scoped runner narrows to the working tree's changes.
type Runner struct {
	Stdout io.Writer
	Stderr io.Writer
	Scope  gitfiles.Selection
}

// Run parses args for mode, executes the Go formatter and vet, renders the
// report, and returns the process exit code.
func (r Runner) Run(ctx context.Context, mode report.Mode, args []string) int {
	_, code := r.RunReport(ctx, mode, args)

	return code
}

// RunReport is Run that also returns the typed outcome so pipeline callers can
// derive their summary details from it rather than scraping the rendered text.
// On a setup failure it returns the zero Outcome and a non-zero code after
// reporting the problem to Stderr, so the outcome is only meaningful when the
// returned code is zero.
func (r Runner) RunReport(ctx context.Context, mode report.Mode, args []string) (Outcome, int) {
	inv, err := ParseInvocation(mode, args, r.Stderr)

	if err != nil {
		return Outcome{}, 1
	}

	workRoot, err := os.Getwd()

	if err != nil {
		r.errf("resolve cwd: %v\n", err)

		return Outcome{}, 1
	}

	reportRoot := workRoot

	if strings.TrimSpace(inv.ReportRoot) != "" {
		reportRoot = inv.ReportRoot
	}

	cfg, err := driverconfig.Load(reportRoot, inv.ConfigPath)

	if err != nil {
		r.errf("%v\n", err)

		return Outcome{}, 1
	}

	outcome, err := Execute(ctx, Request{
		Mode:   mode,
		Paths:  inv.Paths,
		Config: cfg.WithJobs(inv.Jobs),
		Root:   workRoot,
		Scope:  r.Scope,
	})

	if err != nil {
		r.errf("%v\n", err)

		return Outcome{}, 1
	}

	renderer := report.Renderer{Root: reportRoot, Mode: mode}

	if err := renderer.Render(r.Stdout, inv.Output, outcome.Combined); err != nil {
		r.errf("render report: %v\n", err)

		return Outcome{}, 1
	}

	return outcome, outcome.ExitCode()
}

func (r Runner) errf(format string, args ...any) {
	_, _ = fmt.Fprintf(r.Stderr, format, args...)
}
