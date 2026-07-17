package formatter

import (
	"github.com/oullin/fmtkit/packages/formatter/config"
	"github.com/oullin/fmtkit/packages/formatter/engine"
	"github.com/oullin/fmtkit/packages/formatter/internal/step"
	"github.com/oullin/fmtkit/packages/formatter/rules"
	"github.com/oullin/fmtkit/packages/formatter/rules/spacing"
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
func Check(paths []string, cfg config.Config) (engine.Report, error) {
	return newEngine(cfg).Check(paths)
}

// Format applies formatting changes and writes them to disk.
func Format(paths []string, cfg config.Config) (engine.Report, error) {
	return newEngine(cfg).Format(paths)
}
