package config

import (
	"reflect"
	"testing"

	formatterconfig "go.ollin.sh/fmtkit/formatter/config"
)

func TestDefaultComposesFormatterDefaults(t *testing.T) {
	cfg := Default()

	if !reflect.DeepEqual(cfg.Formatter(), formatterconfig.Default()) {
		t.Fatalf("Formatter() should equal formatter defaults\n got: %#v\nwant: %#v", cfg.Formatter(), formatterconfig.Default())
	}

	if !cfg.Vet.Enabled {
		t.Fatal("expected vet enabled by default")
	}
}

func TestVetConfigProjectsToggle(t *testing.T) {
	if got := Default().VetConfig().Enabled; !got {
		t.Fatal("expected default vet config enabled")
	}

	disabled := Default()
	disabled.Vet.Enabled = false

	if got := disabled.VetConfig().Enabled; got {
		t.Fatal("expected disabled vet config")
	}
}

func TestWithJobs(t *testing.T) {
	tests := []struct {
		name string
		jobs int
		want int
	}{
		{name: "unset leaves concurrency", jobs: -1, want: 7},
		{name: "zero pins numcpu sentinel", jobs: 0, want: 0},
		{name: "positive pins worker count", jobs: 4, want: 4},
	}

	for _, tt := range tests {
		tt := tt

		t.Run(tt.name, func(t *testing.T) {
			cfg := Default()
			cfg.Concurrency = 7

			if got := cfg.WithJobs(tt.jobs).Concurrency; got != tt.want {
				t.Fatalf("WithJobs(%d).Concurrency = %d, want %d", tt.jobs, got, tt.want)
			}
		})
	}
}
