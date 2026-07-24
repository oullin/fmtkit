package app

import (
	"go.ollin.sh/fmtkit/driver/internal/gitfiles"
	"go.ollin.sh/fmtkit/driver/internal/golang"
	"go.ollin.sh/fmtkit/driver/internal/pipeline"
	"go.ollin.sh/fmtkit/driver/internal/typescript"
)

// stepSelection selects which parts of the format pipeline run; the zero value
// (no --ts/--go flags) runs everything.
type stepSelection struct {
	TS bool
	Go bool
}

func (s stepSelection) normalized() stepSelection {
	if !s.TS && !s.Go {
		return stepSelection{TS: true, Go: true}
	}

	return s
}

// formatSteps builds the ordered pipeline steps for the selection. Lint runs
// first so the formatting passes normalize whatever oxlint rewrites.
func (d *deps) formatSteps(paths []string, selected stepSelection, selection gitfiles.Selection) []pipeline.Step {
	selected = selected.normalized()

	var steps []pipeline.Step

	if selected.TS {
		steps = append(steps,
			typescript.LintStep(d.version, paths, selection),
			typescript.FormatStep(d.version, paths, selection),
		)
	}

	if selected.Go {
		steps = append(steps, golang.FormatStep(paths, selection))
	}

	return steps
}
