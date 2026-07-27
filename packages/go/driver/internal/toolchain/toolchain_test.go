package toolchain

import (
	"context"
	"fmt"
	"io"
	"testing"

	"go.ollin.sh/fmtkit/driver/internal/pipeline"
)

// fakeChain is a minimal Toolchain that records its name and a single labelled
// step, enough to assert the registry's selection and ordering.
type fakeChain struct {
	name string
}

// labelStep is a Step whose Label is its name, so a selection can be read back
// as a list of names.
type labelStep string

func (c fakeChain) Name() string { return c.name }

func (c fakeChain) Steps(Request) []pipeline.Step {
	return []pipeline.Step{labelStep(c.name)}
}

func (s labelStep) Label() string { return string(s) }

func (s labelStep) Run(context.Context, io.Writer) pipeline.Result { return pipeline.Result{} }

func names(chains []Toolchain) []string {
	out := make([]string, 0, len(chains))

	for _, chain := range chains {
		out = append(out, chain.Name())
	}

	return out
}

func TestSelectEmptyReturnsAllInOrder(t *testing.T) {
	reg := NewRegistry(fakeChain{"ts"}, fakeChain{"go"})

	if got := fmt.Sprint(names(reg.Select())); got != fmt.Sprint([]string{"ts", "go"}) {
		t.Fatalf("Select() = %s, want [ts go]", got)
	}
}

func TestSelectByNamePreservesRegistrationOrder(t *testing.T) {
	reg := NewRegistry(fakeChain{"ts"}, fakeChain{"go"})

	// Ask in the opposite order; the registry still returns registration order.
	if got := fmt.Sprint(names(reg.Select("go", "ts"))); got != fmt.Sprint([]string{"ts", "go"}) {
		t.Fatalf("Select(go, ts) = %s, want [ts go]", got)
	}
}

func TestSelectSingleName(t *testing.T) {
	reg := NewRegistry(fakeChain{"ts"}, fakeChain{"go"})

	if got := fmt.Sprint(names(reg.Select("ts"))); got != fmt.Sprint([]string{"ts"}) {
		t.Fatalf("Select(ts) = %s, want [ts]", got)
	}

	if got := fmt.Sprint(names(reg.Select("go"))); got != fmt.Sprint([]string{"go"}) {
		t.Fatalf("Select(go) = %s, want [go]", got)
	}
}

func TestSelectUnknownNamesAreIgnored(t *testing.T) {
	reg := NewRegistry(fakeChain{"ts"}, fakeChain{"go"})

	if got := names(reg.Select("rust")); len(got) != 0 {
		t.Fatalf("Select(rust) = %v, want empty", got)
	}

	// A known name mixed with an unknown one keeps only the known lane.
	if got := fmt.Sprint(names(reg.Select("go", "rust"))); got != fmt.Sprint([]string{"go"}) {
		t.Fatalf("Select(go, rust) = %s, want [go]", got)
	}
}

func TestSelectStepsComeFromChosenLanes(t *testing.T) {
	reg := NewRegistry(fakeChain{"ts"}, fakeChain{"go"})

	var labels []string

	for _, chain := range reg.Select() {
		for _, step := range chain.Steps(Request{}) {
			labels = append(labels, step.Label())
		}
	}

	if got := fmt.Sprint(labels); got != fmt.Sprint([]string{"ts", "go"}) {
		t.Fatalf("step labels = %s, want [ts go]", got)
	}
}
