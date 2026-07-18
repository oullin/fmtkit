package cli

import (
	"context"
	"io"

	"go.ollin.sh/fmtkit/driver/internal/sourcefiles"
)

func RunSources(ctx context.Context, args []string, stdout, stderr io.Writer) int {
	return sourcefiles.Run(ctx, args, stdout, stderr)
}
