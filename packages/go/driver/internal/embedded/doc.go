// Package embedded carries the TS toolchain baked into release binaries.
//
// The assets are staged under bin/<goos>_<goarch>/ by
// infra/scripts/release/stage-ts-assets.sh and are only compiled in under the
// fmtkit_sidecar build tag (see sidecar_*.go); ordinary builds get the
// sidecar_dev.go stub instead, so the staged directories need not exist.
package embedded
