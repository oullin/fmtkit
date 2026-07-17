// Command fmtkit is the self-contained fmtkit binary distributed through
// GitHub Releases and Homebrew. The command surface lives in internal/app; this
// entrypoint only carries the version stamped in by -X main.version.
package main

import (
	"os"

	"go.ollin.sh/fmtkit/driver/internal/app"
)

var version = "dev"

func main() {
	os.Exit(app.
		New(version, os.Stdout, os.Stderr).
		Run(os.Args[1:]))
}
