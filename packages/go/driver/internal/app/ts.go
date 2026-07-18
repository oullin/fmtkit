package app

import (
	"context"

	"go.ollin.sh/fmtkit/driver/internal/tsruntime"
)

func (a App) runTS(ctx context.Context, paths []string) int {
	support, err := tsruntime.Resolve(a.version)

	if err != nil {
		return a.reportError(err)
	}

	return a.reportError(support.RunPipeline(ctx, tsruntime.RunOptions{Scopes: paths, Stdout: a.stdout, Stderr: a.stderr}))
}

func (a App) runLint(ctx context.Context, paths []string) int {
	support, err := tsruntime.Resolve(a.version)

	if err != nil {
		return a.reportError(err)
	}

	return a.reportError(support.RunLint(ctx, tsruntime.RunOptions{Scopes: paths, Stdout: a.stdout, Stderr: a.stderr}))
}
