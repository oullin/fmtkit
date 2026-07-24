package cli

import (
	"context"
	"fmt"
	"io"
	"os"
	"strings"

	driverconfig "go.ollin.sh/fmtkit/driver/config"
	"go.ollin.sh/fmtkit/driver/internal/gitfiles"
	"go.ollin.sh/fmtkit/driver/internal/sourcefiles"
	driverreport "go.ollin.sh/fmtkit/driver/report"
	"go.ollin.sh/fmtkit/formatter"
	formatterconfig "go.ollin.sh/fmtkit/formatter/config"
	formatterengine "go.ollin.sh/fmtkit/formatter/engine"
	"go.ollin.sh/fmtkit/vet"
)

type Runner struct {
	stdout io.Writer
	stderr io.Writer
	parser parser

	// selection is how much of the working tree the formatter covers. The zero
	// value covers everything, which is what `fmtkit go` and `fmtkit check` want.
	selection sourcefiles.Selection
}

func NewRunner(stdout, stderr io.Writer) Runner {
	return Runner{
		stdout: stdout,
		stderr: stderr,
		parser: newParser(stderr),
	}
}

// NewScopedRunner returns a Runner whose formatter covers only the part of the
// working tree that selection names.
func NewScopedRunner(stdout, stderr io.Writer, selection sourcefiles.Selection) Runner {
	runner := NewRunner(stdout, stderr)
	runner.selection = selection

	return runner
}

func (r Runner) Run(ctx context.Context, mode Mode, args []string) int {
	opts, err := r.parser.Parse(mode, args)

	if err != nil {
		return 1
	}

	workRoot, err := os.Getwd()

	if err != nil {
		r.writeError("resolve cwd: %v\n", err)

		return 1
	}

	reportRoot := workRoot

	if strings.TrimSpace(opts.reportRoot) != "" {
		reportRoot = opts.reportRoot
	}

	cfg, err := driverconfig.Load(reportRoot, opts.configPath)

	if err != nil {
		r.writeError("%v\n", err)

		return 1
	}

	runPaths := opts.positional

	formatterCfg := cfg.Formatter()

	if opts.jobs != -1 {
		formatterCfg.Concurrency = opts.jobs
	}

	formatterReport, err := r.runFormatter(ctx, mode, runPaths, formatterCfg)

	if err != nil {
		r.writeError("%v\n", err)

		return 1
	}

	result := driverreport.Combined{
		Formatter: formatterReport,
		Vet:       vet.Run(ctx, workRoot, cfg.VetConfig()),
	}

	if err := driverreport.Render(r.stdout, opts.outputFormat, reportRoot, mode.String(), result); err != nil {
		r.writeError("render report: %v\n", err)

		return 1
	}

	return exitCode(mode, result)
}

func (r Runner) runFormatter(ctx context.Context, mode Mode, paths []string, cfg formatterconfig.Config) (formatterengine.Report, error) {
	if r.selection == sourcefiles.SelectionChanged {
		files, err := changedGoFiles(ctx, paths, cfg)

		if err != nil {
			return formatterengine.Report{}, err
		}

		switch mode {
		case CheckMode:
			return formatter.CheckFiles(ctx, files, cfg)
		case FormatMode:
			return formatter.FormatFiles(ctx, files, cfg)
		default:
			return formatterengine.Report{}, fmt.Errorf("unsupported mode %q", mode)
		}
	}

	switch mode {
	case CheckMode:
		return formatter.Check(ctx, paths, cfg)
	case FormatMode:
		return formatter.Format(ctx, paths, cfg)
	default:
		return formatterengine.Report{}, fmt.Errorf("unsupported mode %q", mode)
	}
}

// changedGoFiles narrows the files the formatter owns down to the ones the
// working tree has touched. The engine reports what it owns; gitfiles keeps only
// the subset git reports as changed (see Tree.IntersectChanged for why this is
// an intersection rather than a direct `git ls-files *.go`).
func changedGoFiles(ctx context.Context, paths []string, cfg formatterconfig.Config) ([]string, error) {
	owned, err := formatterengine.CollectGoFiles(paths, cfg)

	if err != nil {
		return nil, err
	}

	if len(owned) == 0 {
		return nil, nil
	}

	cwd, err := os.Getwd()

	if err != nil {
		return nil, fmt.Errorf("resolve cwd: %w", err)
	}

	tree, err := gitfiles.NewTree(cwd)

	if err != nil {
		return nil, err
	}

	return tree.IntersectChanged(ctx, paths, owned)
}

func exitCode(mode Mode, result driverreport.Combined) int {
	if result.Vet.ErrorCount() > 0 {
		return 1
	}

	if mode == CheckMode {
		if result.Formatter.Result == formatterengine.ResultPass {
			return 0
		}

		return 1
	}

	if result.Formatter.ErrorCount() > 0 {
		return 1
	}

	return 0
}

func (r Runner) writeError(format string, args ...any) {
	_, _ = fmt.Fprintf(r.stderr, format, args...)
}
