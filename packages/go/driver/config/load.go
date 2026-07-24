package config

import (
	"errors"
	"fmt"
	"os"
	"path/filepath"

	"github.com/spf13/viper"
)

// DefaultFileName is the config file name discovered in the working tree.
const DefaultFileName = "config.yml"

// Load reads CLI configuration from disk or returns defaults when none exists.
func Load(cwd, explicitPath string) (Config, error) {
	// cfg starts fully populated with Default(); viper unmarshals the file onto
	// it, and mapstructure leaves keys absent from the file untouched, so
	// defaults survive without restating them as viper SetDefault calls.
	cfg := Default()

	v := viper.New()

	if explicitPath != "" {
		v.SetConfigFile(explicitPath)
	} else {
		v.SetConfigFile(filepath.Join(cwd, DefaultFileName))
	}

	if err := v.ReadInConfig(); err != nil {
		var notFound viper.ConfigFileNotFoundError

		if explicitPath == "" && (errors.As(err, &notFound) || os.IsNotExist(err)) {
			return cfg, nil
		}

		return Config{}, fmt.Errorf("load config: %w", err)
	}

	if err := v.Unmarshal(&cfg); err != nil {
		return Config{}, fmt.Errorf("decode config: %w", err)
	}

	return cfg, nil
}
