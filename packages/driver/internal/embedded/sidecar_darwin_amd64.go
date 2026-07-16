//go:build fmtkit_sidecar && darwin && amd64

package embedded

import (
	"embed"
	"io/fs"
)

// all: keeps the bundled .oxfmtrc.json and .oxlintrc.json, which a directory
// pattern would drop for starting with a dot.
//
//go:embed all:bin/darwin_amd64
var sidecarAssets embed.FS

// SidecarAssets returns the TS toolchain staged for this platform.
func SidecarAssets() (fs.FS, bool) {
	sub, err := fs.Sub(sidecarAssets, "bin/darwin_amd64")

	if err != nil {
		panic(err)
	}

	return sub, true
}
