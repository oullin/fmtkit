package app

import (
	"context"

	"go.ollin.sh/fmtkit/driver/internal/tsruntime"
)

func (d *deps) runTS(ctx context.Context, paths []string) int {
	assets, err := tsruntime.Resolve(d.version)

	if err != nil {
		return d.reportError(err)
	}

	return d.reportError(tsruntime.NewInvoker(assets).RunPipeline(ctx, tsruntime.Request{Scopes: paths, Stdout: d.stdout, Stderr: d.stderr}))
}

func (d *deps) runLint(ctx context.Context, paths []string) int {
	assets, err := tsruntime.Resolve(d.version)

	if err != nil {
		return d.reportError(err)
	}

	return d.reportError(tsruntime.NewInvoker(assets).RunLint(ctx, tsruntime.Request{Scopes: paths, Stdout: d.stdout, Stderr: d.stderr}))
}
