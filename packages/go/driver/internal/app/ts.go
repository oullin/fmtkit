package app

import (
	"context"
	"os"

	"go.ollin.sh/fmtkit/complexity"
	driverconfig "go.ollin.sh/fmtkit/driver/config"
	"go.ollin.sh/fmtkit/driver/internal/typescript/runtime"
	report "go.ollin.sh/fmtkit/driver/report"
)

func (d *deps) runTS(ctx context.Context, paths []string) int {
	assets, err := runtime.Resolve(d.version)

	if err != nil {
		return d.reportError(err)
	}

	return d.reportError(runtime.NewInvoker(assets).RunPipeline(ctx, runtime.Request{Scopes: paths, Stdout: d.stdout, Stderr: d.stderr}))
}

// runLint lints the TS/Vue sources and then scores their complexity, so a
// consumer with only the TS lane keeps one command for both gates. The lint
// failure is reported first and neither result hides the other.
func (d *deps) runLint(ctx context.Context, paths []string) int {
	assets, err := runtime.Resolve(d.version)

	if err != nil {
		return d.reportError(err)
	}

	code := d.reportError(runtime.NewInvoker(assets).RunLint(ctx, runtime.Request{Scopes: paths, Stdout: d.stdout, Stderr: d.stderr}))

	if complexityCode := d.lintComplexity(ctx, paths); code == 0 {
		code = complexityCode
	}

	return code
}

// lintComplexity runs the TS complexity lane with the same paths the lint took.
func (d *deps) lintComplexity(ctx context.Context, paths []string) int {
	root, err := os.Getwd()

	if err != nil {
		return d.reportError(err)
	}

	cfg, err := driverconfig.Load(root, "")

	if err != nil {
		return d.reportError(err)
	}

	scan, err := d.scanTypeScript(ctx, defaultPaths(paths))

	if err != nil {
		return d.reportError(err)
	}

	outcome := complexity.Evaluate(root, cfg.ComplexityConfig(), []complexity.Scan{scan})

	return d.renderComplexity(root, complexityOptions{output: report.FormatText, quiet: true}, outcome)
}

// defaultPaths pins an empty scope list to the working directory, matching the
// format commands.
func defaultPaths(paths []string) []string {
	if len(paths) == 0 {
		return []string{"."}
	}

	return paths
}
