// Command fmtkit-go is the standalone Go formatter CLI. Its command surface
// lives in internal/app (app.GoCLI); this entrypoint only carries the version
// stamped in by -X main.version and the signal handling.
package main

import (
	"context"
	"os"
	"os/signal"
	"syscall"

	"go.ollin.sh/fmtkit/driver/internal/app"
)

var version = "dev"

func main() {
	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)

	// os.Exit skips deferred calls, so release the signal handler explicitly
	// before exiting with the captured code.
	code := app.
		GoCLI(version, os.Stdout, os.Stderr).
		Dispatch(ctx, os.Args[1:])

	stop()
	os.Exit(code)
}
