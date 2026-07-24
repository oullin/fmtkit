package app

import (
	"context"

	"go.ollin.sh/fmtkit/driver/internal/typescript/runtime"
)

func (d *deps) runTS(ctx context.Context, paths []string) int {
	assets, err := runtime.Resolve(d.version)

	if err != nil {
		return d.reportError(err)
	}

	return d.reportError(runtime.NewInvoker(assets).RunPipeline(ctx, runtime.Request{Scopes: paths, Stdout: d.stdout, Stderr: d.stderr}))
}

func (d *deps) runLint(ctx context.Context, paths []string) int {
	assets, err := runtime.Resolve(d.version)

	if err != nil {
		return d.reportError(err)
	}

	return d.reportError(runtime.NewInvoker(assets).RunLint(ctx, runtime.Request{Scopes: paths, Stdout: d.stdout, Stderr: d.stderr}))
}
