package app

import (
	"fmt"
	"testing"

	"go.ollin.sh/fmtkit/driver/internal/pipeline"
)

func TestStepSelectionNormalized(t *testing.T) {
	if got := (stepSelection{}).normalized(); !got.TS || !got.Go {
		t.Fatalf("zero selection = %+v, want both set", got)
	}

	if got := (stepSelection{TS: true}).normalized(); !got.TS || got.Go {
		t.Fatalf("TS-only selection = %+v, want TS only", got)
	}

	if got := (stepSelection{Go: true}).normalized(); got.TS || !got.Go {
		t.Fatalf("Go-only selection = %+v, want Go only", got)
	}
}

func TestFormatStepsSelection(t *testing.T) {
	d := &deps{version: "dev"}

	labels := func(steps []pipeline.Step) []string {
		out := make([]string, 0, len(steps))

		for _, s := range steps {
			out = append(out, s.Label())
		}

		return out
	}

	all := labels(d.formatSteps([]string{"."}, stepSelection{}, 0))

	if fmt.Sprint(all) != fmt.Sprint([]string{"Running TS/Vue lint", "Running TS/Vue formatting", "Running Go formatting"}) {
		t.Fatalf("default steps = %v", all)
	}

	tsOnly := labels(d.formatSteps([]string{"."}, stepSelection{TS: true}, 0))

	if fmt.Sprint(tsOnly) != fmt.Sprint([]string{"Running TS/Vue lint", "Running TS/Vue formatting"}) {
		t.Fatalf("--ts steps = %v", tsOnly)
	}

	goOnly := labels(d.formatSteps([]string{"."}, stepSelection{Go: true}, 0))

	if fmt.Sprint(goOnly) != fmt.Sprint([]string{"Running Go formatting"}) {
		t.Fatalf("--go steps = %v", goOnly)
	}
}
