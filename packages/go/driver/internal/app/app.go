package app

import (
	"fmt"
	"io"

	"go.ollin.sh/fmtkit/driver/internal/cli"
)

// App is the fmtkit command surface. The version is injected by the binary so
// release builds keep stamping it through -X main.version.
type App struct {
	version string
	stdout  io.Writer
	stderr  io.Writer
}

func New(version string, stdout, stderr io.Writer) App {
	return App{
		version: version,
		stdout:  stdout,
		stderr:  stderr,
	}
}

// Run dispatches a subcommand to its handler; each mode lives in its own file.
func (a App) Run(args []string) int {
	if len(args) == 0 {
		printUsage(a.stderr)

		return 2
	}

	mode := args[0]
	rest := args[1:]

	switch mode {
	case "format":
		return a.runFormat(rest)
	case "format-all":
		return a.runFormatAll(rest)
	case "ts":
		return a.runTS(rest)
	case "lint":
		return a.runLint(rest)
	case "go":
		return a.runGo(rest)
	case "check":
		return cli.
			NewRunner(a.stdout, a.stderr).
			Run(cli.CheckMode, rest)
	case "version", "--version", "-version":
		return a.printVersion()
	case "help", "--help", "-h":
		printUsage(a.stderr)

		return 0
	default:
		_, _ = fmt.Fprintf(a.stderr, "unknown subcommand - {%q}\n\n", mode)

		printUsage(a.stderr)

		return 2
	}
}

func (a App) printVersion() int {
	_, _ = fmt.Fprintf(a.stdout, "fmtkit %s\n", a.version)

	return 0
}
