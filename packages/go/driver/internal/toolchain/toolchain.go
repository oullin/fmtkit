// Package toolchain is the contract and registry that separate the format
// pipeline into per-language lanes. Each Toolchain contributes the ordered
// pipeline steps for one language (TS, Go); the Registry holds them in
// registration order, which is the order they run, and resolves the --ts/--go
// selection down to the lanes that should execute.
package toolchain

import (
	"go.ollin.sh/fmtkit/driver/internal/gitfiles"
	"go.ollin.sh/fmtkit/driver/internal/pipeline"
)

// Request carries what a lane needs to build its steps: the binary version
// (the TS lane extracts a per-version toolchain cache from it), the target
// paths, and how much of the working tree the run scopes to.
type Request struct {
	Version   string
	Paths     []string
	Selection gitfiles.Selection
}

// A Toolchain contributes the pipeline steps for one language lane. Name is the
// lane's selector, matching the --ts/--go flags; Steps builds the ordered steps
// for a request (TS returns [lint, format]; Go returns [format]).
type Toolchain interface {
	Name() string
	Steps(req Request) []pipeline.Step
}

// Registry holds the registered lanes in registration order, which is also
// their execution order.
type Registry struct {
	chains []Toolchain
}

// NewRegistry registers the given lanes in order. The composition root
// constructs and registers them explicitly; there is no init()-based
// self-registration, so registration order is whatever the caller passes.
func NewRegistry(chains ...Toolchain) Registry {
	return Registry{chains: chains}
}

// Select resolves a set of lane names to the lanes that should run. With no
// names it returns every registered lane (the no-flag "everything" default);
// otherwise it returns the registered lanes whose Name is among names. Either
// way the result preserves registration order, and names that match no
// registered lane are ignored.
func (r Registry) Select(names ...string) []Toolchain {
	if len(names) == 0 {
		out := make([]Toolchain, len(r.chains))
		copy(out, r.chains)

		return out
	}

	want := make(map[string]struct{}, len(names))

	for _, name := range names {
		want[name] = struct{}{}
	}

	var out []Toolchain

	for _, chain := range r.chains {
		if _, ok := want[chain.Name()]; ok {
			out = append(out, chain)
		}
	}

	return out
}
