package complexity_test

import (
	"testing"

	"go.ollin.sh/fmtkit/complexity"
)

func TestDefaultCarriesTheShippedLimits(t *testing.T) {
	cfg := complexity.Default()

	if cfg.Cyclomatic != 15 || cfg.Cognitive != 20 || len(cfg.Allow) != 0 {
		t.Fatalf("default = %#v", cfg)
	}
}

func TestLaneOfRoutesKeysByExtension(t *testing.T) {
	cases := map[string]complexity.Lane{
		"internal/envcfg/envcfg.go#(*Source).Read": complexity.LaneGo,
		"src/app.ts#read":                          complexity.LaneTS,
		"src/Screen.tsx#Screen":                    complexity.LaneTS,
		"src/app.mts#read":                         complexity.LaneTS,
		"src/app.cts#read":                         complexity.LaneTS,
		"src/app.js#read":                          complexity.LaneTS,
		"src/app.jsx#read":                         complexity.LaneTS,
		"docs/notes.md#read":                       "",
	}

	for key, want := range cases {
		if got := complexity.LaneOf(key); got != want {
			t.Errorf("LaneOf(%q) = %q, want %q", key, got, want)
		}
	}
}

func TestKeyFileSplitsOnTheFirstSeparator(t *testing.T) {
	if got := complexity.KeyFile("src/app.ts#outer#inner"); got != "src/app.ts" {
		t.Errorf("KeyFile = %q", got)
	}

	if got := complexity.KeyFile("src/app.ts"); got != "src/app.ts" {
		t.Errorf("KeyFile without a name = %q", got)
	}
}

func TestAllowedMatchesTheWholeKey(t *testing.T) {
	cfg := complexity.Config{Allow: []complexity.AllowEntry{{Key: "src/app.ts#read"}}}

	if !cfg.Allowed("src/app.ts#read") {
		t.Error("exact key was not allowed")
	}

	if cfg.Allowed("src/app.ts#reader") {
		t.Error("a longer key was allowed by prefix")
	}
}
