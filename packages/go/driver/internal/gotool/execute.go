package gotool

import (
	"context"
	"fmt"
	"os"

	driverconfig "go.ollin.sh/fmtkit/driver/config"
	"go.ollin.sh/fmtkit/driver/internal/gitfiles"
	report "go.ollin.sh/fmtkit/driver/report"
	"go.ollin.sh/fmtkit/formatter"
	formatterconfig "go.ollin.sh/fmtkit/formatter/config"
	formatterengine "go.ollin.sh/fmtkit/formatter/engine"
	"go.ollin.sh/fmtkit/vet"
)

// Request is one Go check/format run: the mode, the paths to cover, the loaded
// config, the working-tree root git and vet run against, and how much of that
// tree the run scopes to.
type Request struct {
	Mode   report.Mode
	Paths  []string
	Config driverconfig.Config
	Root   string
	Scope  gitfiles.Selection
}

// Outcome is the combined formatter and vet report produced for a mode, ready
// to render or to reduce to an exit code.
type Outcome struct {
	Combined report.Combined
	Mode     report.Mode
}

// ExitCode reduces the outcome to a process exit code under its mode's policy.
func (o Outcome) ExitCode() int {
	return o.Combined.ExitCode(o.Mode)
}

// Execute runs the Go formatter (scoped as the request asks) and go vet, and
// returns the combined outcome. It is the reusable core the standalone runner
// and the umbrella pipeline both drive.
func Execute(ctx context.Context, req Request) (Outcome, error) {
	formatterReport, err := runFormatter(ctx, req, req.Config.Formatter())

	if err != nil {
		return Outcome{}, err
	}

	combined := report.Combined{
		Formatter: formatterReport,
		Vet:       vet.Run(ctx, req.Root, req.Config.VetConfig()),
	}

	return Outcome{Combined: combined, Mode: req.Mode}, nil
}

func runFormatter(ctx context.Context, req Request, cfg formatterconfig.Config) (formatterengine.Report, error) {
	if req.Scope == gitfiles.SelectionChanged {
		files, err := changedGoFiles(ctx, req.Root, req.Paths, cfg)

		if err != nil {
			return formatterengine.Report{}, err
		}

		switch req.Mode {
		case report.ModeCheck:
			return formatter.CheckFiles(ctx, files, cfg)
		case report.ModeFormat:
			return formatter.FormatFiles(ctx, files, cfg)
		default:
			return formatterengine.Report{}, fmt.Errorf("unsupported mode %q", req.Mode)
		}
	}

	switch req.Mode {
	case report.ModeCheck:
		return formatter.Check(ctx, req.Paths, cfg)
	case report.ModeFormat:
		return formatter.Format(ctx, req.Paths, cfg)
	default:
		return formatterengine.Report{}, fmt.Errorf("unsupported mode %q", req.Mode)
	}
}

// changedGoFiles narrows the files the formatter owns down to the ones the
// working tree has touched. The engine reports what it owns; gitfiles keeps only
// the subset git reports as changed (see Tree.IntersectChanged for why this is
// an intersection rather than a direct `git ls-files *.go`).
func changedGoFiles(ctx context.Context, root string, paths []string, cfg formatterconfig.Config) ([]string, error) {
	owned, err := formatterengine.CollectGoFiles(paths, cfg)

	if err != nil {
		return nil, err
	}

	if len(owned) == 0 {
		return nil, nil
	}

	if root == "" {
		cwd, err := os.Getwd()

		if err != nil {
			return nil, fmt.Errorf("resolve cwd: %w", err)
		}

		root = cwd
	}

	tree, err := gitfiles.NewTree(root)

	if err != nil {
		return nil, err
	}

	return tree.IntersectChanged(ctx, paths, owned)
}
