//go:build !fmtkit_sidecar

package fmtkit

import "io/fs"

// SidecarAssets reports that this binary carries no TS toolchain: it was
// built without the fmtkit_sidecar build tag.
func SidecarAssets() (fs.FS, bool) {
	return nil, false
}
