package runner

import (
	"errors"
	"os/exec"
)

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

		if errors.As(err, &exitErr) {
			r.failure(label + " failed")

			return exitErr.ExitCode()
		}

		writef(r.stderr, "%s: %v\n", label, err)

		return 1
	}

	return 0
}
