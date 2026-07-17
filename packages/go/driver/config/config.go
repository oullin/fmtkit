package config

import (
	formatterconfig "github.com/oullin/fmtkit/packages/formatter/config"
	"github.com/oullin/fmtkit/packages/vet"
)

// Toggle enables or disables a config section.
type Toggle struct {
	Enabled bool `mapstructure:"enabled"`
}

// Rules configures rule toggles exposed by the CLI.
type Rules struct {
	Spacing Toggle `mapstructure:"spacing"`
}

// Formatters configures formatter passes exposed by the CLI.
type Formatters struct {
	Gofmt     bool `mapstructure:"gofmt"`
	Goimports bool `mapstructure:"goimports"`
}

// Config controls CLI formatting and vet behavior.
type Config struct {
	Rules      Rules      `mapstructure:"rules"`
	Vet        Toggle     `mapstructure:"vet"`
	Formatters Formatters `mapstructure:"formatters"`
	Exclude    []string   `mapstructure:"exclude"`
	NotPath    []string   `mapstructure:"not_path"`
	NotName    []string   `mapstructure:"not_name"`
	// Concurrency caps the number of files processed in parallel.
	// Zero means use runtime.NumCPU().
	Concurrency int `mapstructure:"concurrency"`
}

// Default returns the default CLI configuration.
func Default() Config {
	return Config{
		Rules: Rules{
			Spacing: Toggle{Enabled: true},
		},
		Vet: Toggle{Enabled: true},
		Formatters: Formatters{
			Gofmt:     true,
			Goimports: true,
		},
		Exclude: []string{
			".git",
			"node_modules",
			"vendor",
		},
		NotPath: []string{},
		NotName: []string{},
	}
}

// FormatterConfig projects CLI config into the public formatter config type.
func (c Config) FormatterConfig() formatterconfig.Config {
	return formatterconfig.Config{
		Rules: formatterconfig.Rules{
			Spacing: formatterconfig.RuleToggle{Enabled: c.Rules.Spacing.Enabled},
		},
		Formatters: formatterconfig.Formatters{
			Gofmt:     c.Formatters.Gofmt,
			Goimports: c.Formatters.Goimports,
		},
		Exclude:     c.Exclude,
		NotPath:     c.NotPath,
		NotName:     c.NotName,
		Concurrency: c.Concurrency,
	}
}

// VetConfig projects CLI config into the public vet config type.
func (c Config) VetConfig() vet.Config {
	return vet.Config{Enabled: c.Vet.Enabled}
}
