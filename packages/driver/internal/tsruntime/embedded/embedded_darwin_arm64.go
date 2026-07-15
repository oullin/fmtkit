//go:build fmtkit_sidecar && darwin && arm64

package embedded

import (
	"embed"
	"io/fs"
)

//go:embed dist/darwin_arm64
var assets embed.FS

// Assets returns the TS toolchain staged for this platform.
func Assets() (fs.FS, bool) {
	sub, err := fs.Sub(assets, "dist/darwin_arm64")

	if err != nil {
		panic(err)
	}

	return sub, true
}
