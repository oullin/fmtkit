package main

import (
	"os"

	"github.com/oullin/go-fmt/packages/driver/internal/sourcefiles"
)

func main() {
	os.Exit(sourcefiles.Run(os.Args[1:], os.Stdout, os.Stderr))
}
