package full

import (
	"fmt"
	"io"
	"os/exec"
	"strings"

	"github.com/oullin/fmtkit/packages/driver/internal/cli"
)

type Runner struct {
	stdout  io.Writer
	stderr  io.Writer
	version string
}

func NewRunner(stdout io.Writer, stderr io.Writer, version string) Runner {
	return Runner{
		stdout:  stdout,
		stderr:  stderr,
		version: version,
	}
}

func (r Runner) Run(args []string) int {
	if len(args) == 0 {
		r.printUsage()

		return 2
	}

	mode := args[0]
	rest := args[1:]

	switch mode {
	case "format":
		return r.runFormat(rest)
	case "format-all":
		if len(rest) != 0 {
			r.printUsage()

			return 2
		}

		return r.runFormat([]string{"."})
	case "go":
		return r.runGo(rest)
	case "ts":
		return r.runTS(rest, true)
	case "check":
		return r.runGo(append([]string{"check"}, rest...))
	case "version", "--version", "-version":
		fmt.Fprintf(r.stdout, "go-fmt %s\n", r.version)

		return 0
	case "help", "--help", "-h":
		r.printUsage()

		return 0
	default:
		fmt.Fprintf(r.stderr, "unknown subcommand - {%q}\n\n", mode)
		r.printUsage()

		return 2
	}
}

func (r Runner) runFormat(paths []string) int {
	if len(paths) == 0 {
		paths = []string{"."}
	}

	tools, err := ensureToolRuntime()

	if err != nil {
		fmt.Fprintf(r.stderr, "%v\n", err)

		return 1
	}

	r.section("Formatting target(s)")
	r.detail("paths", strings.Join(paths, " "))

	if status := r.runTool("Running TS/Vue formatting", tools.formatTSBin(), paths, tools.env(), true); status != 0 {
		return status
	}

	if status := r.runTool("Running TS/Vue lint", tools.lintTSBin(), paths, tools.env(), true); status != 0 {
		return status
	}

	r.section("Running Go formatting")

	restore := tools.applyGoEnv()

	defer restore()

	if status := cli.NewRunner(r.stdout, r.stderr).Run(cli.FormatMode, append([]string{"--vet=false"}, paths...)); status != 0 {
		return status
	}

	r.section("Formatting complete")
	r.detail("status", "done")

	return 0
}

func (r Runner) runTS(args []string, streamToStderr bool) int {
	if len(args) == 0 {
		args = []string{"."}
	}

	tools, err := ensureToolRuntime()

	if err != nil {
		fmt.Fprintf(r.stderr, "%v\n", err)

		return 1
	}

	return r.runTool("Running TS/Vue formatting", tools.formatTSBin(), args, tools.env(), streamToStderr)
}

func (r Runner) runGo(args []string) int {
	if len(args) == 0 {
		r.printGoUsage()

		return 2
	}

	switch args[0] {
	case "check":
		restore, ok := r.applyRuntimeForGo()

		if !ok {
			return 1
		}

		defer restore()

		return cli.NewRunner(r.stdout, r.stderr).Run(cli.CheckMode, args[1:])
	case "format":
		restore, ok := r.applyRuntimeForGo()

		if !ok {
			return 1
		}

		defer restore()

		return cli.NewRunner(r.stdout, r.stderr).Run(cli.FormatMode, args[1:])
	case "sources":
		return cli.RunSources(args[1:], r.stdout, r.stderr)
	case "version", "--version", "-version":
		fmt.Fprintf(r.stdout, "go-fmt %s\n", r.version)

		return 0
	case "help", "--help", "-h":
		r.printGoUsage()

		return 0
	default:
		fmt.Fprintf(r.stderr, "unknown go subcommand - {%q}\n\n", args[0])
		r.printGoUsage()

		return 2
	}
}

func (r Runner) applyRuntimeForGo() (func(), bool) {
	tools, err := ensureToolRuntime()

	if err != nil {
		fmt.Fprintf(r.stderr, "%v\n", err)

		return func() {}, false
	}

	return tools.applyGoEnv(), true
}

func (r Runner) runTool(label string, bin string, args []string, env []string, streamToStderr bool) int {
	r.section(label)

	cmd := exec.Command(bin, args...)
	cmd.Env = env

	if streamToStderr {
		cmd.Stdout = r.stderr
	} else {
		cmd.Stdout = r.stdout
	}

	cmd.Stderr = r.stderr

	if err := cmd.Run(); err != nil {
		var exitErr *exec.ExitError

		if ok := errorAs(err, &exitErr); ok {
			r.failure(label + " failed")

			return exitErr.ExitCode()
		}

		fmt.Fprintf(r.stderr, "%s: %v\n", label, err)

		return 1
	}

	return 0
}

func (r Runner) section(label string) {
	fmt.Fprintf(r.stderr, "\n==> %s\n", label)
}

func (r Runner) detail(label string, value string) {
	fmt.Fprintf(r.stderr, "    %-12s %s\n", label, value)
}

func (r Runner) failure(label string) {
	fmt.Fprintf(r.stderr, "\n!! %s\n", label)
}

func (r Runner) printUsage() {
	fmt.Fprintln(r.stderr, "usage: fmt-all <format|format-all|go|ts|check|version|help> [args...]")
	fmt.Fprintln(r.stderr, "  format [paths...]                        run TS/Vue support + lint, then Go formatting")
	fmt.Fprintln(r.stderr, "  format-all                               run the full formatter pipeline against .")
	fmt.Fprintln(r.stderr, "  go [check|format|sources|version|help] [args...] run the Go formatter CLI")
	fmt.Fprintln(r.stderr, "  ts [paths...]                            run TS/Vue formatting support and oxfmt")
	fmt.Fprintln(r.stderr, "  check|version|help [args...]             run the matching Go formatter CLI command")
}

func (r Runner) printGoUsage() {
	fmt.Fprintln(r.stderr, "go-fmt check [--host-path /absolute/host/path] [paths...]")
	fmt.Fprintln(r.stderr)
	fmt.Fprintln(r.stderr, "go-fmt format [--host-path /absolute/host/path] [paths...]")
	fmt.Fprintln(r.stderr)
	fmt.Fprintln(r.stderr, "go-fmt sources [--include-declarations] [paths...]")
}
