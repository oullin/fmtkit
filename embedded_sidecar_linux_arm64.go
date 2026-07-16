//go:build fmtkit_sidecar && linux && arm64

package fmtkit

import (
	"embed"
	"io/fs"
)

//go:embed infra/bin/linux_arm64
var sidecarAssets embed.FS

// SidecarAssets returns the TS toolchain staged for this platform.
func SidecarAssets() (fs.FS, bool) {
	sub, err := fs.Sub(sidecarAssets, "infra/bin/linux_arm64")

	if err != nil {
		panic(err)
	}

	return sub, true
}
