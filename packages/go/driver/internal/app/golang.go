package app

import (
	"context"
	"fmt"

	"go.ollin.sh/fmtkit/driver/internal/cli"
)

// runGo mirrors the fmtkit-go command surface so `fmtkit go ...` behaves like
// the container's Go formatter CLI.
func (a App) runGo(ctx context.Context, args []string) int {
	if len(args) == 0 {
		printGoUsage(a.stderr)

		return 2
	}

	switch args[0] {
	case "check":
		return cli.
			NewRunner(a.stdout, a.stderr).
			Run(ctx, cli.CheckMode, args[1:])
	case "format":
		return cli.
			NewRunner(a.stdout, a.stderr).
			Run(ctx, cli.FormatMode, args[1:])
	case "sources":
		return cli.RunSources(ctx, args[1:], a.stdout, a.stderr)
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
