package config

import (
	"os"
	"path/filepath"
	"reflect"
	"testing"
)

func writeConfig(t *testing.T, content string) string {
	t.Helper()

	dir := t.TempDir()

	if err := os.WriteFile(filepath.Join(dir, DefaultFileName), []byte(content), 0o644); err != nil {
		t.Fatalf("write config: %v", err)
	}

	return dir
}

// TestLoadFullConfigRoundTrips pins that every top-level key in the config
// schema decodes onto its field. It guards the schema contract the loader must
// keep byte-compatible.
func TestLoadFullConfigRoundTrips(t *testing.T) {
	dir := writeConfig(t, "rules:\n  spacing:\n    enabled: false\nvet:\n  enabled: false\nformatters:\n  gofmt: false\n  goimports: false\nexclude:\n  - build\nnot_path:\n  - generated\nnot_name:\n  - '*.pb.go'\nconcurrency: 4\n")

	cfg, err := Load(dir, "")

	if err != nil {
		t.Fatalf("load config: %v", err)
	}

	if cfg.Rules.Spacing.Enabled {
		t.Fatalf("expected spacing disabled")
	}

	if cfg.Vet.Enabled {
		t.Fatalf("expected vet disabled")
	}

	if cfg.Formatters.Gofmt || cfg.Formatters.Goimports {
		t.Fatalf("expected formatters disabled: %#v", cfg.Formatters)
	}

	if !reflect.DeepEqual(cfg.Exclude, []string{"build"}) {
		t.Fatalf("unexpected exclude: %#v", cfg.Exclude)
	}

	if !reflect.DeepEqual(cfg.NotPath, []string{"generated"}) {
		t.Fatalf("unexpected not_path: %#v", cfg.NotPath)
	}

	if !reflect.DeepEqual(cfg.NotName, []string{"*.pb.go"}) {
		t.Fatalf("unexpected not_name: %#v", cfg.NotName)
	}

	if cfg.Concurrency != 4 {
		t.Fatalf("unexpected concurrency: %d", cfg.Concurrency)
	}
}

// TestLoadPartialKeepsDefaults pins that keys absent from the file retain their
// Default() values while the present key wins. This is the mapstructure
// leave-absent-keys-untouched behavior the loader relies on.
func TestLoadPartialKeepsDefaults(t *testing.T) {
	dir := writeConfig(t, "rules:\n  spacing:\n    enabled: false\n")

	cfg, err := Load(dir, "")

	if err != nil {
		t.Fatalf("load config: %v", err)
	}

	if cfg.Rules.Spacing.Enabled {
		t.Fatalf("expected spacing disabled from file")
	}

	if !cfg.Vet.Enabled {
		t.Fatalf("expected vet to keep default enabled")
	}

	if !cfg.Formatters.Gofmt || !cfg.Formatters.Goimports {
		t.Fatalf("expected formatters to keep defaults: %#v", cfg.Formatters)
	}

	if !reflect.DeepEqual(cfg.Exclude, []string{".git", "node_modules", "vendor"}) {
		t.Fatalf("expected exclude to keep default: %#v", cfg.Exclude)
	}
}

// TestLoadExplicitEmptyExcludeOverridesDefault pins that an explicit empty list
// clears the default exclude list rather than being treated as absent.
func TestLoadExplicitEmptyExcludeOverridesDefault(t *testing.T) {
	dir := writeConfig(t, "exclude: []\n")

	cfg, err := Load(dir, "")

	if err != nil {
		t.Fatalf("load config: %v", err)
	}

	if len(cfg.Exclude) != 0 {
		t.Fatalf("expected explicit empty exclude to override default, got %#v", cfg.Exclude)
	}
}

// TestLoadEmptyFileYieldsDefaults pins that a present-but-empty config file
// leaves every field at its Default() value.
func TestLoadEmptyFileYieldsDefaults(t *testing.T) {
	dir := writeConfig(t, "")

	cfg, err := Load(dir, "")

	if err != nil {
		t.Fatalf("load config: %v", err)
	}

	if !reflect.DeepEqual(cfg, Default()) {
		t.Fatalf("expected defaults for empty file\n got: %#v\nwant: %#v", cfg, Default())
	}
}

// TestLoadExplicitMissingPathErrors pins that a missing explicit config path is
// an error, unlike a missing default-location file which falls back to defaults.
func TestLoadExplicitMissingPathErrors(t *testing.T) {
	if _, err := Load(t.TempDir(), filepath.Join(t.TempDir(), "missing.yml")); err == nil {
		t.Fatal("expected error for missing explicit config path")
	}
}

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
