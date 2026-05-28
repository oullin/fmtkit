package sourcefiles

import (
	"flag"
	"fmt"
	"io"
	"os"
)

func Run(args []string, stdout, stderr io.Writer) int {
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
			fmt.Fprintf(stderr, "resolve cwd: %v\n", err)

			return 1
		}
	}

	files, warnings, err := Collect(Options{
		Cwd:                 cwd,
		IncludeDeclarations: *includeDeclarations,
		Scopes:              fs.Args(),
	})

	for _, warning := range warnings {
		fmt.Fprintf(stderr, "[sources] %s\n", warning)
	}

	if err != nil {
		fmt.Fprintf(stderr, "[sources] %v\n", err)

		return 1
	}

	for _, file := range files {
		fmt.Fprintf(stdout, "%s%c", file, 0)
	}

	return 0
}
