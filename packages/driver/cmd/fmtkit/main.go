package main

import (
	"os"

	"github.com/oullin/fmtkit/packages/driver/internal/full/runner"
)

var version = "dev"

func main() {
	os.Exit(runner.New(os.Stdout, os.Stderr, version).Run(os.Args[1:]))
}
