package app

import (
	"context"

	"go.ollin.sh/fmtkit/driver/internal/tsruntime"
)

func (a App) runTS(ctx context.Context, paths []string) int {
	assets, err := tsruntime.Resolve(a.version)

	if err != nil {
		return a.reportError(err)
	}

	return a.reportError(tsruntime.NewInvoker(assets).RunPipeline(ctx, tsruntime.Request{Scopes: paths, Stdout: a.stdout, Stderr: a.stderr}))
}

func (a App) runLint(ctx context.Context, paths []string) int {
	assets, err := tsruntime.Resolve(a.version)

	if err != nil {
		return a.reportError(err)
	}

	return a.reportError(tsruntime.NewInvoker(assets).RunLint(ctx, tsruntime.Request{Scopes: paths, Stdout: a.stdout, Stderr: a.stderr}))
}
