//go:build fmtkit_sidecar && linux && arm64

package embedded

import (
	"embed"
	"io/fs"
)

// all: keeps the bundled .oxfmtrc.json and .oxlintrc.json, which a directory
// pattern would drop for starting with a dot.
//
//go:embed all:bin/linux_arm64
var sidecarAssets embed.FS

// SidecarAssets returns the TS toolchain staged for this platform.
func SidecarAssets() (fs.FS, bool) {
	sub, err := fs.Sub(sidecarAssets, "bin/linux_arm64")

	if err != nil {
		panic(err)
	}

	return sub, true
}
