package cli

import (
	"io"

	"github.com/oullin/go-fmt/packages/driver/internal/sourcefiles"
)

func RunSources(args []string, stdout, stderr io.Writer) int {
	return sourcefiles.Run(args, stdout, stderr)
}
