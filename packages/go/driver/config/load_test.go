package config

import (
	"os"
	"path/filepath"
	"testing"
)

func TestLoadDefaultsWhenConfigDoesNotExist(t *testing.T) {
	dir := t.TempDir()

	cfg, err := Load(dir, "")

	if err != nil {
		t.Fatalf("load config: %v", err)
	}

	if !cfg.Rules.Spacing.Enabled {
		t.Fatalf("expected spacing rule enabled by default")
	}

	if !cfg.Vet.Enabled {
		t.Fatalf("expected vet enabled by default")
	}

	if !cfg.Formatters.Gofmt || !cfg.Formatters.Goimports {
		t.Fatalf("expected gofmt and goimports enabled by default")
	}
}

func TestLoadYAMLOverridesDefaults(t *testing.T) {
	dir := t.TempDir()
	configPath := filepath.Join(dir, DefaultFileName)
	content := []byte("rules:\n  spacing:\n    enabled: false\nvet:\n  enabled: false\nformatters:\n  gofmt: true\n  goimports: false\nexclude:\n  - build\nnot_path:\n  - generated\nnot_name:\n  - '*.pb.go'\n")

	if err := os.WriteFile(configPath, content, 0o644); err != nil {
		t.Fatalf("write config: %v", err)
	}

	cfg, err := Load(dir, "")

	if err != nil {
		t.Fatalf("load config: %v", err)
	}

	if cfg.Rules.Spacing.Enabled {
		t.Fatalf("expected spacing rule disabled")
	}

	if cfg.Vet.Enabled {
		t.Fatalf("expected vet disabled")
	}

	if cfg.Formatters.Goimports {
		t.Fatalf("expected goimports disabled")
	}

	if len(cfg.Exclude) != 1 || cfg.Exclude[0] != "build" {
		t.Fatalf("unexpected exclude list: %#v", cfg.Exclude)
	}

	if len(cfg.NotPath) != 1 || cfg.NotPath[0] != "generated" {
		t.Fatalf("unexpected not_path list: %#v", cfg.NotPath)
	}

	if len(cfg.NotName) != 1 || cfg.NotName[0] != "*.pb.go" {
		t.Fatalf("unexpected not_name list: %#v", cfg.NotName)
	}
}
