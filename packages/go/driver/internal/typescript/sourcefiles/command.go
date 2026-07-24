package sourcefiles

import (
	"context"
	"flag"
	"fmt"
	"io"
	"os"

	"go.ollin.sh/fmtkit/driver/internal/gitfiles"
)

// Run parses the `sources` subcommand flags, collects the formattable files
// under the given scopes, and prints them NUL-separated to stdout.
func Run(ctx context.Context, args []string, stdout, stderr io.Writer) int {
	fs := flag.NewFlagSet("sources", flag.ContinueOnError)
	fs.SetOutput(stderr)

	cwdFlag := fs.String("cwd", "", "Working tree root to collect source files from")
	includeDeclarations := fs.Bool("include-declarations", false, "Include .d.ts declaration files")

	if err := fs.Parse(args); err != nil {
		return 1
	}

	cwd := *cwdFlag

	if cwd == "" {
		var err error

		cwd, err = os.Getwd()

		if err != nil {
			_, _ = fmt.Fprintf(stderr, "resolve cwd: %v\n", err)

			return 1
		}
	}

	collector, err := New(cwd, gitfiles.SelectionAll, *includeDeclarations)

	if err != nil {
		_, _ = fmt.Fprintf(stderr, "[sources] %v\n", err)

		return 1
	}

	files, warnings, err := collector.Formattable(ctx, fs.Args())

	for _, warning := range warnings {
		_, _ = fmt.Fprintf(stderr, "[sources] %s\n", warning)
	}

	if err != nil {
		_, _ = fmt.Fprintf(stderr, "[sources] %v\n", err)

		return 1
	}

	for _, file := range files {
		_, _ = fmt.Fprintf(stdout, "%s%c", file, 0)
	}

	return 0
}
