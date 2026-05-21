package config

// RuleToggle enables or disables an individual rule.
type RuleToggle struct {
	Enabled bool `mapstructure:"enabled"`
}

// Rules configures rule-level behavior.
type Rules struct {
	Spacing RuleToggle `mapstructure:"spacing"`
}

// Formatters configures formatter passes applied after rules.
type Formatters struct {
	Gofmt     bool `mapstructure:"gofmt"`
	Goimports bool `mapstructure:"goimports"`
}

// Config controls the default formatter pipeline.
type Config struct {
	Rules      Rules      `mapstructure:"rules"`
	Formatters Formatters `mapstructure:"formatters"`
	Exclude    []string   `mapstructure:"exclude"`
	NotPath    []string   `mapstructure:"not_path"`
	NotName    []string   `mapstructure:"not_name"`
	// Concurrency caps the number of files processed in parallel.
	// Zero means use runtime.NumCPU().
	Concurrency int `mapstructure:"concurrency"`
}

// Default returns the default formatter configuration.
func Default() Config {
	return Config{
		Rules: Rules{
			Spacing: RuleToggle{Enabled: true},
		},
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
