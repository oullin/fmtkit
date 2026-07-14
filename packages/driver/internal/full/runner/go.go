package runner

import (
	"github.com/oullin/fmtkit/packages/driver/internal/cli"
	"github.com/oullin/fmtkit/packages/runtimex"
)

func (r Runner) runGo(args []string) int {
	if len(args) == 0 {
		r.printGoUsage()

		return 2
	}

	switch args[0] {
	case "check":
		return r.runGoCLI(cli.CheckMode, args[1:])
	case "format":
		return r.runGoCLI(cli.FormatMode, args[1:])
	case "sources":
		return cli.RunSources(args[1:], r.stdout, r.stderr)
	case "version", "--version", "-version":
		writef(r.stdout, "go-fmt %s\n", r.version)

		return 0
	case "help", "--help", "-h":
		r.printGoUsage()

		return 0
	default:
		writef(r.stderr, "unknown go subcommand - {%q}\n\n", args[0])
		r.printGoUsage()

		return 2
	}
}

func (r Runner) runGoCLI(mode cli.Mode, args []string) int {
	runtime, err := runtimex.Ensure()

	if err != nil {
		writef(r.stderr, "%v\n", err)

		return 1
	}

	restore := runtime.ApplyGoEnvironment()

	defer restore()

	return cli.NewRunner(r.stdout, r.stderr).Run(mode, args)
}
