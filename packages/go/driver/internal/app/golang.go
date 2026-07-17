package app

import (
	"fmt"

	"github.com/oullin/fmtkit/packages/driver/internal/cli"
)

// runGo mirrors the fmtkit-go command surface so `fmtkit go ...` behaves like
// the container's Go formatter CLI.
func (a App) runGo(args []string) int {
	if len(args) == 0 {
		printGoUsage(a.stderr)

		return 2
	}

	switch args[0] {
	case "check":
		return cli.
			NewRunner(a.stdout, a.stderr).
			Run(cli.CheckMode, args[1:])
	case "format":
		return cli.
			NewRunner(a.stdout, a.stderr).
			Run(cli.FormatMode, args[1:])
	case "sources":
		return cli.RunSources(args[1:], a.stdout, a.stderr)
	case "version", "--version", "-version":
		return a.printVersion()
	case "help", "--help", "-h":
		printGoUsage(a.stderr)

		return 0
	default:
		_, _ = fmt.Fprintf(a.stderr, "unknown subcommand - {%q}\n\n", args[0])

		printGoUsage(a.stderr)

		return 2
	}
}
