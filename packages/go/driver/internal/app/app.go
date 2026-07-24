package app

import (
	"context"
	"fmt"
	"io"

	"go.ollin.sh/fmtkit/driver/internal/command"
	"go.ollin.sh/fmtkit/driver/internal/golang"
	"go.ollin.sh/fmtkit/driver/internal/sourcefiles"
	report "go.ollin.sh/fmtkit/driver/report"
)

// deps carries what every command handler needs: the version stamped by the
// binary and the output streams. It is a pointer so the usage printer can be
// wired in after the Set is built.
type deps struct {
	version string
	stdout  io.Writer
	stderr  io.Writer

	// usage prints the enclosing Set's usage text; wired after the Set exists so
	// the flag-parsing handlers can reprint it on a bad argument.
	usage func(io.Writer)
}

// umbrellaHeader is the top line of the umbrella usage; the per-command lines
// follow from each Command's Usage.
const umbrellaHeader = "usage: fmtkit <format|format-all|go|ts|lint|check|version|help> [args...]\n"

// Umbrella builds the fmtkit command surface: the pipeline commands plus the
// embedded Go formatter CLI reached through `fmtkit go`.
func Umbrella(version string, stdout, stderr io.Writer) command.Set {
	d := &deps{version: version, stdout: stdout, stderr: stderr}

	// The Go CLI reached through `fmtkit go` prints "fmtkit go ..." usage and
	// adopts the umbrella's exit code for bad subcommands.
	goSet := d.goCommandSet("fmtkit go", 2)

	set := command.Set{
		Name:    "fmtkit",
		Header:  umbrellaHeader,
		ErrExit: 2,
		Stderr:  stderr,
		Commands: []command.Command{
			{
				Name:  "format",
				Usage: "  format [--ts] [--go] [--quiet] [paths...]  format changed files (vs HEAD) and untracked ones\n",
				Run:   d.runFormat,
			},
			{
				Name:  "format-all",
				Usage: "  format-all [--ts] [--go] [--quiet]       format every file, against .\n      --ts   only TS/Vue lint + formatting; --go   only Go formatting; default: all\n",
				Run:   d.runFormatAll,
			},
			{
				Name:  "go",
				Usage: "  go <check|format|sources|version|help>  run the Go formatter CLI\n",
				Run:   goSet.Dispatch,
			},
			{
				Name:  "ts",
				Usage: "  ts [paths...]                            run TS/Vue formatting support and oxfmt\n",
				Run:   d.runTS,
			},
			{
				Name:  "lint",
				Usage: "  lint [paths...]                          lint TS/Vue files with oxlint\n",
				Run:   d.runLint,
			},
			{
				Name:  "check",
				Usage: "  check [args...]                          run the Go formatter in check mode\n",
				Run:   d.runCheck,
			},
			{
				Name:    "version",
				Aliases: []string{"--version", "-version"},
				Usage:   "  version                                  print the fmtkit version\n",
				Run:     d.runVersion,
			},
		},
	}

	d.usage = set.PrintUsage

	return set
}

// GoCLI builds the standalone fmtkit-go command surface: check, format,
// sources, version, and help, exiting 1 on a bad subcommand.
func GoCLI(version string, stdout, stderr io.Writer) command.Set {
	d := &deps{version: version, stdout: stdout, stderr: stderr}

	set := d.goCommandSet("fmtkit", 1)

	d.usage = set.PrintUsage

	return set
}

// goCommandSet builds the Go formatter command group. name is the usage prefix
// ("fmtkit" standalone, "fmtkit go" under the umbrella) and errExit is the
// exit code for an empty or unknown subcommand.
func (d *deps) goCommandSet(name string, errExit int) command.Set {
	usage := func(sub string) string {
		return name + " " + sub + "\n\n"
	}

	return command.Set{
		Name:    name,
		ErrExit: errExit,
		Stderr:  d.stderr,
		Commands: []command.Command{
			{
				Name:  "check",
				Usage: usage("check [paths...]"),
				Run:   d.runCheck,
			},
			{
				Name:  "format",
				Usage: usage("format [paths...]"),
				Run:   d.runGoFormat,
			},
			{
				Name:  "sources",
				Usage: usage("sources [--include-declarations] [paths...]"),
				Run: func(ctx context.Context, args []string) int {
					return sourcefiles.Run(ctx, args, d.stdout, d.stderr)
				},
			},
			{
				Name:    "version",
				Aliases: []string{"--version", "-version"},
				Run:     d.runVersion,
			},
		},
	}
}

// goRunner is the unscoped Go formatter runner shared by `check` and the
// standalone `format`.
func (d *deps) goRunner() golang.Runner {
	return golang.Runner{Stdout: d.stdout, Stderr: d.stderr}
}

func (d *deps) runCheck(ctx context.Context, args []string) int {
	return d.goRunner().Run(ctx, report.ModeCheck, args)
}

func (d *deps) runGoFormat(ctx context.Context, args []string) int {
	return d.goRunner().Run(ctx, report.ModeFormat, args)
}

func (d *deps) runVersion(_ context.Context, _ []string) int {
	_, _ = fmt.Fprintf(d.stdout, "fmtkit %s\n", d.version)

	return 0
}
