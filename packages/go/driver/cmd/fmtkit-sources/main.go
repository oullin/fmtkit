package main

import (
	"os"

	"go.ollin.sh/fmtkit/driver/internal/sourcefiles"
)

func main() {
	os.Exit(sourcefiles.Run(os.Args[1:], os.Stdout, os.Stderr))
}
