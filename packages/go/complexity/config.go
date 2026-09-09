package complexity

import (
	"path/filepath"
	"slices"
	"strings"
)

// Lane names the language a key belongs to. An allow entry is only evaluated
// by the lane that owns its file extension, so a single-lane run never trips
// over the other lane's baseline.
type Lane string

const (
	// LaneGo owns .go keys.
	LaneGo Lane = "go"

	// LaneTS owns the TypeScript/JavaScript family.
	LaneTS Lane = "ts"
)

const (
	// DefaultCyclomatic is the per-function cyclomatic limit.
	DefaultCyclomatic = 15

	// DefaultCognitive is the per-function cognitive limit.
	DefaultCognitive = 20
)

const (
	// RuleCyclomatic labels a cyclomatic-limit finding.
	RuleCyclomatic = "complexity/cyclomatic"

	// RuleCognitive labels a cognitive-limit finding.
	RuleCognitive = "complexity/cognitive"

	// RuleAllow labels an allow entry that matched no function.
	RuleAllow = "complexity/allow"
)

// tsExtensions are the TypeScript-family and JavaScript extensions the TS lane
// scores. It mirrors the sidecar's own list; a key outside both lanes is
// ignored rather than guessed at.
var tsExtensions = []string{".ts", ".tsx", ".mts", ".cts", ".js", ".jsx"}

// AllowEntry exempts one function key from both limits until it is refactored.
// Reason is documentation only; the check never reads it.
type AllowEntry struct {
	Key    string `mapstructure:"key"`
	Reason string `mapstructure:"reason"`
}

// Config is the complexity policy: the two per-function limits and the keyed
// baseline that holds today's offenders while they are burnt down.
type Config struct {
	Cyclomatic int
	Cognitive  int
	Allow      []AllowEntry
}

// Default returns the shipped policy.
func Default() Config {
	return Config{Cyclomatic: DefaultCyclomatic, Cognitive: DefaultCognitive}
}

// Allowed reports whether a function key is exempt.
func (c Config) Allowed(key string) bool {
	return slices.ContainsFunc(c.Allow, func(entry AllowEntry) bool {
		return entry.Key == key
	})
}

// KeyFile splits a "<path>#<name>" key back into its file part. A key without
// a separator is treated as a bare path.
func KeyFile(key string) string {
	file, _, found := strings.Cut(key, "#")

	if !found {
		return key
	}

	return file
}

// LaneOf resolves the lane that owns a key, or "" when no lane does.
func LaneOf(key string) Lane {
	extension := strings.ToLower(filepath.Ext(KeyFile(key)))

	if extension == ".go" {
		return LaneGo
	}

	if slices.Contains(tsExtensions, extension) {
		return LaneTS
	}

	return ""
}
