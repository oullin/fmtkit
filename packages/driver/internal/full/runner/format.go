package runner

import (
	"strings"

	"github.com/oullin/fmtkit/packages/driver/internal/cli"
	"github.com/oullin/fmtkit/packages/driver/internal/full/planner"
	"github.com/oullin/fmtkit/packages/runtimex"
)

func (r Runner) runFormat(paths []string, family languageFamily) int {
	plan, err := planner.Build(planner.Options{Scopes: paths})

	if err != nil {
		writef(r.stderr, "%v\n", err)

		return 1
	}

	if family == goLanguage {
		plan.TS = nil
	}

	if family == tsLanguage {
		plan.Go = nil
	}

	if len(plan.Go) == 0 && len(plan.TS) == 0 {
		return 0
	}

	runtime, err := runtimex.Ensure()

	if err != nil {
		writef(r.stderr, "%v\n", err)

		return 1
	}

	r.section("Formatting target(s)")

	if len(plan.TS) != 0 {
		r.detail("ts/vue", strings.Join(plan.TS, " "))

		if status := r.runTool("Running TS/Vue formatting", runtime.FormatTSBinary(), plan.TS, runtime.Environment(), true); status != 0 {
			return status
		}

		if status := r.runTool("Running TS/Vue lint", runtime.LintTSBinary(), plan.TS, runtime.Environment(), true); status != 0 {
			return status
		}
	}

	if len(plan.Go) != 0 {
		r.detail("go", strings.Join(plan.Go, " "))
		r.section("Running Go formatting")
		restore := runtime.ApplyGoEnvironment()
		status := cli.NewRunner(r.stdout, r.stderr).Run(cli.FormatMode, append([]string{"--vet=false"}, plan.Go...))
		restore()

		if status != 0 {
			return status
		}
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
