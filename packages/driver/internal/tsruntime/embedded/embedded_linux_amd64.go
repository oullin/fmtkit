//go:build fmtkit_sidecar && linux && amd64

package embedded

import (
	"embed"
	"io/fs"
)

//go:embed dist/linux_amd64
var assets embed.FS

// Assets returns the TS toolchain staged for this platform.
func Assets() (fs.FS, bool) {
	sub, err := fs.Sub(assets, "dist/linux_amd64")

	if err != nil {
		panic(err)
	}

	return sub, true
}
