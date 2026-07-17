package app

import (
	"fmt"
	"strings"

	"github.com/oullin/fmtkit/packages/driver/internal/orchestrator"
)

type formatOptions struct {
	steps orchestrator.Steps
	quiet bool
}

// parseFormatArgs splits the format/format-all flags from the paths. With no
// step flags the whole pipeline runs; --ts and --go narrow it.
func parseFormatArgs(args []string) (formatOptions, []string, error) {
	var opts formatOptions

	var paths []string

	for _, arg := range args {
		switch arg {
		case "--ts":
			opts.steps.TS = true
		case "--go":
			opts.steps.Go = true
		case "--quiet", "-q":
			opts.quiet = true
		default:
			if strings.HasPrefix(arg, "-") {
				return formatOptions{}, nil, fmt.Errorf("unknown flag - {%q}", arg)
			}

			paths = append(paths, arg)
		}
	}

	return opts, paths, nil
}
