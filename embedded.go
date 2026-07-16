// Package fmtkit exposes repository-level assets embedded into the fmtkit
// binaries. It lives at the module root because go:embed cannot reference
// files above the embedding package; that also covers the TS toolchain
// staged under infra/bin/ by infra/scripts/release/stage-ts-assets.sh, which
// only builds tagged fmtkit_sidecar embed (see embedded_sidecar_*.go).
package fmtkit

import _ "embed"

// OxfmtConfig is the bundled oxfmt configuration, used when a target project
// has no .oxfmtrc.* of its own.
//
//go:embed .oxfmtrc.json
var OxfmtConfig []byte

// OxlintConfig is the bundled oxlint configuration, used when a target
// project has no .oxlintrc* of its own.
//
//go:embed .oxlintrc.json
var OxlintConfig []byte
