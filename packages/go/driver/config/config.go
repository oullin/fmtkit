package config

import (
	"go.ollin.sh/fmtkit/complexity"
	formatterconfig "go.ollin.sh/fmtkit/formatter/config"
	"go.ollin.sh/fmtkit/vet"
)

// Toggle enables or disables a config section.
type Toggle struct {
	Enabled bool `mapstructure:"enabled"`
}

// Complexity is the per-function complexity policy: the two limits and the
// keyed baseline that exempts today's offenders while they are burnt down.
// A limit of zero or less turns that metric off.
type Complexity struct {
	Cyclomatic int                     `mapstructure:"cyclomatic"`
	Cognitive  int                     `mapstructure:"cognitive"`
	Allow      []complexity.AllowEntry `mapstructure:"allow"`
}

// Config controls CLI formatting and vet behavior. It embeds the formatter
// config as the single source of truth for formatting options and adds the vet
// toggle the CLI owns. The squash tag flattens the embedded formatter keys to
// the top level so the on-disk schema stays a flat set of keys.
type Config struct {
	formatterconfig.Config `mapstructure:",squash"`

	Vet Toggle `mapstructure:"vet"`

	Complexity Complexity `mapstructure:"complexity"`
}

// Default returns the default CLI configuration: the formatter defaults with
// vet enabled.
func Default() Config {
	return Config{
		Config: formatterconfig.Default(),
		Vet:    Toggle{Enabled: true},
		Complexity: Complexity{
			Cyclomatic: complexity.DefaultCyclomatic,
			Cognitive:  complexity.DefaultCognitive,
		},
	}
}

// Formatter returns the embedded formatter configuration.
func (c Config) Formatter() formatterconfig.Config {
	return c.Config
}

// VetConfig projects CLI config into the public vet config type.
func (c Config) VetConfig() vet.Config {
	return vet.Config{Enabled: c.Vet.Enabled}
}

// ComplexityConfig projects CLI config into the public complexity config type.
func (c Config) ComplexityConfig() complexity.Config {
	return complexity.Config{
		Cyclomatic: c.Complexity.Cyclomatic,
		Cognitive:  c.Complexity.Cognitive,
		Allow:      c.Complexity.Allow,
	}
}

// WithJobs applies a --jobs override to the formatter concurrency. A jobs value
// of -1 means "unset" and returns the config unchanged; any other value pins
// Concurrency (0 selects runtime.NumCPU()), matching the CLI's jobs-override
// semantics.
func (c Config) WithJobs(jobs int) Config {
	if jobs != -1 {
		c.Concurrency = jobs
	}

	return c
}
