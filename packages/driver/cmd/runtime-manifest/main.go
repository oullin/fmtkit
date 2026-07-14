package main

import (
	"flag"
	"fmt"
	"os"
	"strings"

	"github.com/oullin/fmtkit/packages/runtimeintegrity"
)

func main() {
	root := flag.String("root", "", "runtime directory to hash")
	archive := flag.String("archive", "", "runtime archive to hash")
	output := flag.String("output", "", "manifest output path")
	required := flag.String("required", "", "comma-separated required runtime files")
	flag.Parse()

	if *root == "" || *archive == "" || *output == "" || *required == "" {
		fmt.Fprintln(os.Stderr, "usage: runtime-manifest --root DIR --archive FILE --output FILE --required path[,path...]")
		os.Exit(2)
	}

	manifest, err := runtimeintegrity.Build(*root, *archive, strings.Split(*required, ","))

	if err != nil {
		fmt.Fprintf(os.Stderr, "build runtime manifest: %v\n", err)
		os.Exit(1)
	}

	content, err := runtimeintegrity.Marshal(manifest)

	if err != nil {
		fmt.Fprintf(os.Stderr, "marshal runtime manifest: %v\n", err)
		os.Exit(1)
	}

	if err := os.WriteFile(*output, content, 0o600); err != nil {
		fmt.Fprintf(os.Stderr, "write runtime manifest: %v\n", err)
		os.Exit(1)
	}
}
