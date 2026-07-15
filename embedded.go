// Package fmtkit exposes repository-level assets embedded into the fmtkit
// binaries. It lives at the module root because go:embed cannot reference
// files above the embedding package.
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
