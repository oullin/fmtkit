package app

import (
	"go.ollin.sh/fmtkit/driver/internal/tsruntime"
)

func (a App) runTS(paths []string) int {
	support, err := tsruntime.Resolve(a.version)

	if err != nil {
		return a.reportError(err)
	}

	return a.reportError(support.RunPipeline(tsruntime.RunOptions{Scopes: paths, Stdout: a.stdout, Stderr: a.stderr}))
}

func (a App) runLint(paths []string) int {
	support, err := tsruntime.Resolve(a.version)

	if err != nil {
		return a.reportError(err)
	}

	return a.reportError(support.RunLint(tsruntime.RunOptions{Scopes: paths, Stdout: a.stdout, Stderr: a.stderr}))
}
