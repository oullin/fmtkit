package config_test

import (
	"reflect"
	"testing"

	"go.ollin.sh/fmtkit/formatter/config"
)

func TestDefault(t *testing.T) {
	cfg := config.Default()

	if !cfg.Rules.Spacing.Enabled {
		t.Fatal("expected spacing rule to be enabled")
	}

	if !cfg.Formatters.Gofmt {
		t.Fatal("expected gofmt formatter to be enabled")
	}

	if !cfg.Formatters.Goimports {
		t.Fatal("expected goimports formatter to be enabled")
	}

	if cfg.Concurrency != 0 {
		t.Fatalf("expected default concurrency 0, got %d", cfg.Concurrency)
	}

	if !reflect.DeepEqual(cfg.Exclude, []string{".git", "node_modules", "vendor"}) {
		t.Fatalf("unexpected excludes: %#v", cfg.Exclude)
	}

	if len(cfg.NotPath) != 0 {
		t.Fatalf("expected no default path exclusions, got %#v", cfg.NotPath)
	}

	if len(cfg.NotName) != 0 {
		t.Fatalf("expected no default name exclusions, got %#v", cfg.NotName)
	}
}
