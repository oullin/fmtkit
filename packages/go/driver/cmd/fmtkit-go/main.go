package main

import (
	"context"
	"fmt"
	"io"
	"os"
	"os/signal"
	"syscall"

	"go.ollin.sh/fmtkit/driver/internal/cli"
)

var version = "dev"

func main() {
	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)

	defer stop()

	os.Exit(run(ctx, os.Args[1:], os.Stdout, os.Stderr))
}

func run(ctx context.Context, args []string, stdout, stderr io.Writer) int {
	if len(args) == 0 {
		printUsage(stderr)

		return 1
	}

	switch args[0] {
	case "check":
		return cli.
			NewRunner(stdout, stderr).
			Run(ctx, cli.CheckMode, args[1:])
	case "format":
		return cli.
			NewRunner(stdout, stderr).
			Run(ctx, cli.FormatMode, args[1:])
	case "sources":
		return cli.RunSources(ctx, args[1:], stdout, stderr)
	case "version", "--version", "-version":
		_, _ = fmt.Fprintf(stdout, "fmtkit %s\n", version)

		return 0
	case "help", "--help", "-h":
		printUsage(stderr)

		return 0
	default:
		_, _ = fmt.Fprintf(stderr, "unknown subcommand - {%q}\n\n", args[0])

		printUsage(stderr)

		return 1
	}
}

func printUsage(w io.Writer) {
	_, _ = fmt.Fprintf(w, "fmtkit check [paths...]\n\n")
	_, _ = fmt.Fprintf(w, "fmtkit format [paths...]\n\n")
	_, _ = fmt.Fprintf(w, "fmtkit sources [--include-declarations] [paths...]\n\n")
}
