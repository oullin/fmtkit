package main

import (
	"os"

	"github.com/oullin/fmtkit/packages/driver/internal/full"
)

var version = "dev"

func main() {
	os.Exit(full.NewRunner(os.Stdout, os.Stderr, version).Run(os.Args[1:]))
}
