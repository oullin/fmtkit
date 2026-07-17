package cli

import (
	"io"

	"go.ollin.sh/fmtkit/driver/internal/sourcefiles"
)

func RunSources(args []string, stdout, stderr io.Writer) int {
	return sourcefiles.Run(args, stdout, stderr)
}
