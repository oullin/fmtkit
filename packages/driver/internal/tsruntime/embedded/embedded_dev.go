//go:build !fmtkit_sidecar

// Package embedded carries the per-platform TS toolchain assets that
// infra/scripts/release/stage-ts-assets.sh places under dist/. Only builds
// tagged fmtkit_sidecar embed them; regular builds stay lightweight and rely
// on FMTKIT_SUPPORT_DIR instead.
package embedded

import "io/fs"

// Assets reports that this binary carries no TS toolchain: it was built
// without the fmtkit_sidecar build tag.
func Assets() (fs.FS, bool) {
	return nil, false
}
