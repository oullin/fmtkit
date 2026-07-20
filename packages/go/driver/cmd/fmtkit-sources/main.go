package main

import (
	"context"
	"os"
	"os/signal"
	"syscall"

	"go.ollin.sh/fmtkit/driver/internal/sourcefiles"
)

func main() {
	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)

	// os.Exit skips deferred calls, so release the signal handler explicitly
	// before exiting with the captured code.
	code := sourcefiles.Run(ctx, os.Args[1:], os.Stdout, os.Stderr)

	stop()
	os.Exit(code)
}
