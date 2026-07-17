package formatter

import (
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
func Check(paths []string, cfg config.Config) (engine.Report, error) {
	return newEngine(cfg).Check(paths)
}

// Format applies formatting changes and writes them to disk.
func Format(paths []string, cfg config.Config) (engine.Report, error) {
	return newEngine(cfg).Format(paths)
}

// CheckFiles reports formatting changes for an explicit list of Go files,
// skipping the walk that Check does. The caller owns the list, so it is
// responsible for having applied cfg's exclusions — see engine.CollectGoFiles.
func CheckFiles(files []string, cfg config.Config) (engine.Report, error) {
	return newEngine(cfg).CheckFiles(files)
}

// FormatFiles applies formatting changes to an explicit list of Go files,
// skipping the walk that Format does. The caller owns the list, so it is
// responsible for having applied cfg's exclusions — see engine.CollectGoFiles.
func FormatFiles(files []string, cfg config.Config) (engine.Report, error) {
	return newEngine(cfg).FormatFiles(files)
}
