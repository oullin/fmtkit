// Command fmtkit is the self-contained fmtkit binary distributed through
// GitHub Releases and Homebrew: the pipeline orchestration that
// infra/bin/fmtkit provides in the container images, fused with the Go
// formatter CLI and the embedded TS toolchain (see internal/tsruntime).
package main

import (
	"errors"
	"fmt"
	"io"
	"os"
	"os/exec"

	"github.com/oullin/fmtkit/packages/driver/internal/cli"
	"github.com/oullin/fmtkit/packages/driver/internal/orchestrator"
	"github.com/oullin/fmtkit/packages/driver/internal/tsruntime"
)

var version = "dev"

func main() {
	os.Exit(run(os.Args[1:], os.Stdout, os.Stderr))
}

func run(args []string, stdout, stderr io.Writer) int {
	if len(args) == 0 {
		printUsage(stderr)

		return 2
	}

	mode := args[0]
	rest := args[1:]

	switch mode {
	case "format":
		quiet, paths := splitQuiet(rest)

		return runPipeline(paths, quiet, stderr)
	case "format-all":
		quiet, extra := splitQuiet(rest)

		if len(extra) != 0 {
			printUsage(stderr)

			return 2
		}

		return runPipeline([]string{"."}, quiet, stderr)
	case "ts":
		return runTS(rest, stdout, stderr)
	case "lint":
		return runLint(rest, stdout, stderr)
	case "go":
		return runGo(rest, stdout, stderr)
	case "check":
		return cli.
			NewRunner(stdout, stderr).
			Run(cli.CheckMode, rest)
	case "version", "--version", "-version":
		_, _ = fmt.Fprintf(stdout, "fmtkit %s\n", version)

		return 0
	case "help", "--help", "-h":
		printUsage(stderr)

		return 0
	default:
		_, _ = fmt.Fprintf(stderr, "unknown subcommand - {%q}\n\n", mode)

		printUsage(stderr)

		return 2
	}
}

func runPipeline(paths []string, quiet bool, stderr io.Writer) int {
	pipeline := orchestrator.Pipeline{
		Tools: orchestrator.Tools{
			TS: func(scopes []string, output io.Writer) error {
				support, err := tsruntime.Resolve(version)

				if err != nil {
					return err
				}

				return support.RunPipeline(tsruntime.RunOptions{Scopes: scopes, Stdout: output, Stderr: output})
			},
			Lint: func(scopes []string, output io.Writer) error {
				support, err := tsruntime.Resolve(version)

				if err != nil {
					return err
				}

				return support.RunLint(tsruntime.RunOptions{Scopes: scopes, Stdout: output, Stderr: output})
			},
			Go: func(args []string, output io.Writer) int {
				return cli.
					NewRunner(output, output).
					Run(cli.FormatMode, args[1:])
			},
		},
		Quiet:  quiet,
		Stderr: stderr,
	}

	return pipeline.RunFormat(paths)
}

func runTS(paths []string, stdout, stderr io.Writer) int {
	support, err := tsruntime.Resolve(version)

	if err != nil {
		return reportError(err, stderr)
	}

	return reportError(support.RunPipeline(tsruntime.RunOptions{Scopes: paths, Stdout: stdout, Stderr: stderr}), stderr)
}

func runLint(paths []string, stdout, stderr io.Writer) int {
	support, err := tsruntime.Resolve(version)

	if err != nil {
		return reportError(err, stderr)
	}

	return reportError(support.RunLint(tsruntime.RunOptions{Scopes: paths, Stdout: stdout, Stderr: stderr}), stderr)
}

// runGo mirrors the fmtkit-go command surface so `fmtkit go ...` behaves like
// the container's Go formatter CLI.
func runGo(args []string, stdout, stderr io.Writer) int {
	if len(args) == 0 {
		printGoUsage(stderr)

		return 2
	}

	switch args[0] {
	case "check":
		return cli.
			NewRunner(stdout, stderr).
			Run(cli.CheckMode, args[1:])
	case "format":
		return cli.
			NewRunner(stdout, stderr).
			Run(cli.FormatMode, args[1:])
	case "sources":
		return cli.RunSources(args[1:], stdout, stderr)
	case "version", "--version", "-version":
		_, _ = fmt.Fprintf(stdout, "fmtkit %s\n", version)

		return 0
	case "help", "--help", "-h":
		printGoUsage(stderr)

		return 0
	default:
		_, _ = fmt.Fprintf(stderr, "unknown subcommand - {%q}\n\n", args[0])

		printGoUsage(stderr)

		return 2
	}
}

func splitQuiet(args []string) (bool, []string) {
	quiet := false

	var rest []string

	for _, arg := range args {
		if arg == "--quiet" || arg == "-q" {
			quiet = true

			continue
		}

		rest = append(rest, arg)
	}

	return quiet, rest
}

func reportError(err error, stderr io.Writer) int {
	if err == nil {
		return 0
	}

	var exit *exec.ExitError

	if errors.As(err, &exit) {
		return exit.ExitCode()
	}

	_, _ = fmt.Fprintf(stderr, "fmtkit: %v\n", err)

	return 1
}

func printUsage(w io.Writer) {
	_, _ = fmt.Fprintf(w, "usage: fmtkit <format|format-all|go|ts|lint|check|version|help> [args...]\n")
	_, _ = fmt.Fprintf(w, "  format [--quiet] [paths...]              run TS/Vue support + lint, then Go formatting\n")
	_, _ = fmt.Fprintf(w, "  format-all [--quiet]                     run the full formatter pipeline against .\n")
	_, _ = fmt.Fprintf(w, "  go <check|format|sources|version|help>  run the Go formatter CLI\n")
	_, _ = fmt.Fprintf(w, "  ts [paths...]                            run TS/Vue formatting support and oxfmt\n")
	_, _ = fmt.Fprintf(w, "  lint [paths...]                          lint TS/Vue files with oxlint\n")
	_, _ = fmt.Fprintf(w, "  check [args...]                          run the Go formatter in check mode\n")
	_, _ = fmt.Fprintf(w, "  version                                  print the fmtkit version\n")
}

func printGoUsage(w io.Writer) {
	_, _ = fmt.Fprintf(w, "fmtkit go check [--host-path /absolute/host/path] [paths...]\n\n")
	_, _ = fmt.Fprintf(w, "fmtkit go format [--host-path /absolute/host/path] [paths...]\n\n")
	_, _ = fmt.Fprintf(w, "fmtkit go sources [--include-declarations] [paths...]\n\n")
}
