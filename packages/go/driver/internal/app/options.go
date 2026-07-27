package app

import (
	"fmt"
	"strings"
)

type formatOptions struct {
	// toolchains names the lanes to run, as the registry selects them. Empty
	// means every lane (the no-flag default); --ts and --go narrow it.
	toolchains []string
	quiet      bool
}

// parseFormatArgs splits the format/format-all flags from the paths. With no
// lane flags every lane runs; --ts and --go narrow it.
func parseFormatArgs(args []string) (formatOptions, []string, error) {
	var opts formatOptions

	var paths []string

	for _, arg := range args {
		switch arg {
		case "--ts":
			opts.toolchains = append(opts.toolchains, "ts")
		case "--go":
			opts.toolchains = append(opts.toolchains, "go")
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
