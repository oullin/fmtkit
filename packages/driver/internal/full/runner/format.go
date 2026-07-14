package runner

import (
	"strings"

	"github.com/oullin/fmtkit/packages/driver/internal/cli"
	"github.com/oullin/fmtkit/packages/runtimex"
)

func (r Runner) runFormat(paths []string) int {
	if len(paths) == 0 {
		paths = []string{"."}
	}

	runtime, err := runtimex.Ensure()

	if err != nil {
		writef(r.stderr, "%v\n", err)

		return 1
	}

	r.section("Formatting target(s)")
	r.detail("paths", strings.Join(paths, " "))

	if status := r.runTool("Running TS/Vue formatting", runtime.FormatTSBinary(), paths, runtime.Environment(), true); status != 0 {
		return status
	}

	if status := r.runTool("Running TS/Vue lint", runtime.LintTSBinary(), paths, runtime.Environment(), true); status != 0 {
		return status
	}

	r.section("Running Go formatting")
	restore := runtime.ApplyGoEnvironment()

	defer restore()

	if status := cli.NewRunner(r.stdout, r.stderr).Run(cli.FormatMode, append([]string{"--vet=false"}, paths...)); status != 0 {
		return status
	}

	r.section("Formatting complete")
	r.detail("status", "done")

	return 0
}

func (r Runner) runTS(paths []string, streamToStderr bool) int {
	if len(paths) == 0 {
		paths = []string{"."}
	}

	runtime, err := runtimex.Ensure()

	if err != nil {
		writef(r.stderr, "%v\n", err)

		return 1
	}

	return r.runTool("Running TS/Vue formatting", runtime.FormatTSBinary(), paths, runtime.Environment(), streamToStderr)
}
