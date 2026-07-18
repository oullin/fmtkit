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

	defer stop()

	os.Exit(sourcefiles.Run(ctx, os.Args[1:], os.Stdout, os.Stderr))
}
