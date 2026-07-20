// Command fmtkit is the self-contained fmtkit binary distributed through
// GitHub Releases and Homebrew. The command surface lives in internal/app; this
// entrypoint only carries the version stamped in by -X main.version.
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
		New(version, os.Stdout, os.Stderr).
		Run(ctx, os.Args[1:])

	stop()
	os.Exit(code)
}
