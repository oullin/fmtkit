package main

import (
	"flag"
	"fmt"
	"io"
	"os"
	"strings"

	"github.com/oullin/fmtkit/packages/runtimex/integrityx"
)

func main() {
	os.Exit(run(os.Args[1:], os.Stderr))
}

func run(args []string, stderr io.Writer) int {
	flags := flag.NewFlagSet("runtime-manifest", flag.ContinueOnError)
	flags.SetOutput(io.Discard)
	root := flags.String("root", "", "runtime directory to hash")
	archive := flags.String("archive", "", "runtime archive to hash")
	output := flags.String("output", "", "manifest output path")
	required := flags.String("required", "", "comma-separated required runtime files")
	goos := flags.String("goos", "", "runtime Go operating system")
	goarch := flags.String("goarch", "", "runtime Go architecture")

	if err := flags.Parse(args); err != nil {
		_, _ = fmt.Fprintln(stderr, err)

		return 2
	}

	if *root == "" || *archive == "" || *output == "" || *required == "" || strings.TrimSpace(*goos) == "" || strings.TrimSpace(*goarch) == "" {
		_, _ = fmt.Fprintln(stderr, "usage: runtime-manifest --root DIR --archive FILE --output FILE --goos GOOS --goarch GOARCH --required path[,path...]")

		return 2
	}

	manifest, err := integrityx.Build(*root, *archive, *goos, *goarch, strings.Split(*required, ","))

	if err != nil {
		_, _ = fmt.Fprintf(stderr, "build runtime manifest: %v\n", err)

		return 1
	}

	content, err := integrityx.Marshal(manifest)

	if err != nil {
		_, _ = fmt.Fprintf(stderr, "marshal runtime manifest: %v\n", err)

		return 1
	}

	if err := os.WriteFile(*output, content, 0o600); err != nil {
		_, _ = fmt.Fprintf(stderr, "write runtime manifest: %v\n", err)

		return 1
	}

	return 0
}
