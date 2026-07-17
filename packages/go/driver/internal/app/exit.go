package app

import (
	"errors"
	"fmt"
	"os/exec"
)

// reportError maps a tool failure onto an exit code, propagating the child's
// own code when it already reported the problem itself.
func (a App) reportError(err error) int {
	if err == nil {
		return 0
	}

	var exit *exec.ExitError

	if errors.As(err, &exit) {
		return exit.ExitCode()
	}

	_, _ = fmt.Fprintf(a.stderr, "fmtkit: %v\n", err)

	return 1
}
