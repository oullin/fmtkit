package config

import (
	formatterconfig "go.ollin.sh/fmtkit/formatter/config"
	"go.ollin.sh/fmtkit/vet"
)

// Toggle enables or disables a config section.
type Toggle struct {
	Enabled bool `mapstructure:"enabled"`
}

// Config controls CLI formatting and vet behavior. It embeds the formatter
// config as the single source of truth for formatting options and adds the vet
// toggle the CLI owns. The squash tag flattens the embedded formatter keys to
// the top level so the on-disk schema stays a flat set of keys.
type Config struct {
	formatterconfig.Config `mapstructure:",squash"`

	Vet Toggle `mapstructure:"vet"`
}

// Default returns the default CLI configuration: the formatter defaults with
// vet enabled.
func Default() Config {
	return Config{
		Config: formatterconfig.Default(),
		Vet:    Toggle{Enabled: true},
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
