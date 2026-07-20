package formatter

import (
	"context"

	"go.ollin.sh/fmtkit/formatter/config"
	"go.ollin.sh/fmtkit/formatter/engine"
	"go.ollin.sh/fmtkit/formatter/internal/step"
	"go.ollin.sh/fmtkit/formatter/rules"
	"go.ollin.sh/fmtkit/formatter/rules/spacing"
)

func buildRules(cfg config.Config) []rules.Rule {
	var out []rules.Rule

	if cfg.Rules.Spacing.Enabled {
		out = append(out, spacing.New())
	}

	return out
}

func buildFormatters(cfg config.Config) []engine.Formatter {
	var out []engine.Formatter

	if cfg.Formatters.Gofmt {
		out = append(out, step.NewGofmt())
	}

	if cfg.Formatters.Goimports {
		out = append(out, step.NewGoimports())
	}

	return out
}

func newEngine(cfg config.Config) *engine.Engine {
	return engine.New(cfg, buildRules(cfg), buildFormatters(cfg))
}

// Check reports formatting changes without writing them to disk.
func Check(ctx context.Context, paths []string, cfg config.Config) (engine.Report, error) {
	return newEngine(cfg).Check(ctx, paths)
}

// Format applies formatting changes and writes them to disk.
func Format(ctx context.Context, paths []string, cfg config.Config) (engine.Report, error) {
	return newEngine(cfg).Format(ctx, paths)
}

// CheckFiles reports formatting changes for an explicit list of Go files,
// skipping the walk that Check does. The caller owns the list, so it is
// responsible for having applied cfg's exclusions — see engine.CollectGoFiles.
func CheckFiles(ctx context.Context, files []string, cfg config.Config) (engine.Report, error) {
	return newEngine(cfg).CheckFiles(ctx, files)
}

// FormatFiles applies formatting changes to an explicit list of Go files,
// skipping the walk that Format does. The caller owns the list, so it is
// responsible for having applied cfg's exclusions — see engine.CollectGoFiles.
func FormatFiles(ctx context.Context, files []string, cfg config.Config) (engine.Report, error) {
	return newEngine(cfg).FormatFiles(ctx, files)
}
