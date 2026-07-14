// Package runner orchestrates the full formatter command.
package runner

import "io"

type Runner struct {
	stdout  io.Writer
	stderr  io.Writer
	version string
}

func New(stdout io.Writer, stderr io.Writer, version string) Runner {
	return Runner{stdout: stdout, stderr: stderr, version: version}
}

func (r Runner) Run(args []string) int {
	request, code := r.validateRequest(args)

	if code != 0 {
		return code
	}

	switch request.command {
	case "format":
		return r.runFormat(request.args)
	case "format-all":
		return r.runFormat([]string{"."})
	case "go":
		return r.runGo(request.args)
	case "ts":
		return r.runTS(request.args, true)
	case "check":
		return r.runGo(append([]string{"check"}, request.args...))
	case "version", "--version", "-version":
		writef(r.stdout, "go-fmt %s\n", r.version)

		return 0
	case "help", "--help", "-h":
		r.printUsage()

		return 0
	}

	panic("validated command was not dispatched")
}
