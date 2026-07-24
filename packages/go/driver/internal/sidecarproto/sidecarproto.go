// Package sidecarproto is the single source of truth for the stringly-typed
// wire protocol between the Go driver and the bun-compiled TS sidecar: the
// asset filenames, the sidecar's dispatch modes, the environment variables that
// override toolchain resolution, the exact argument vectors each mode expects,
// and the summary lines the sidecar prints back.
//
// Every value here is frozen byte-for-byte: the TS sidecar parses these argv
// forms and reads these environment names, and CI's smoke test plus the Go
// fake-bin tests prove both ends agree. Change a constant here only in lockstep
// with packages/ts/sidecar.
package sidecarproto

import "os"

// Overrides carries every environment override a TS toolchain invocation
// honours, resolved once rather than ad hoc deep in the call paths.
type Overrides struct {
	PipelineBin  string
	OxfmtBin     string
	OxlintBin    string
	OxfmtConfig  string
	OxlintConfig string
	SourcesCwd   string
}

// Asset filenames staged alongside the sidecar and read by both ends.
const (
	// SidecarName is the multiplexed toolchain executable's filename.
	SidecarName = "fmtkit-ts-sidecar"

	// OxfmtRCName is the bundled oxfmt configuration filename.
	OxfmtRCName = ".oxfmtrc.json"

	// OxlintRCName is the bundled oxlint configuration filename.
	OxlintRCName = ".oxlintrc.json"
)

// Dispatch modes: the sidecar selects a toolchain by its first positional
// argument (process.argv[2]) or, equivalently, by SidecarModeEnv.
const (
	ModePipeline = "pipeline"
	ModeOxfmt    = "oxfmt"
	ModeOxlint   = "oxlint"
)

// Environment variable names that cross the Go/TS boundary or steer toolchain
// resolution. These are the complete set the driver honours; ReadOverrides is
// the only place the process environment is consulted for the override subset.
const (
	// SupportDirEnv points at a pre-extracted toolchain directory, skipping
	// both the embedded assets and the per-version cache.
	SupportDirEnv = "FMTKIT_SUPPORT_DIR"

	// SidecarModeEnv is the sidecar's alternate mode selector, read by the TS
	// entrypoint when no positional mode is supplied.
	SidecarModeEnv = "FMTKIT_SIDECAR_MODE"

	// PipelineBinEnv overrides the executable spawned for the pipeline mode.
	PipelineBinEnv = "FMTKIT_TS_PIPELINE_BIN"

	// OxfmtBinEnv runs oxfmt directly instead of through the sidecar.
	OxfmtBinEnv = "OXFMT_BIN"

	// OxlintBinEnv runs oxlint directly instead of through the sidecar.
	OxlintBinEnv = "OXLINT_BIN"

	// OxfmtConfigEnv forces a specific oxfmt configuration path.
	OxfmtConfigEnv = "FMTKIT_OXFMTRC"

	// OxlintConfigEnv forces a specific oxlint configuration path.
	OxlintConfigEnv = "FMTKIT_OXLINTRC"

	// SourcesCwdEnv overrides the working directory file collection runs in.
	SourcesCwdEnv = "FMTKIT_SOURCES_CWD"
)

// ReadOverrides gathers every environment override in one place. It is the sole
// os.Getenv site for the override variables above.
func ReadOverrides() Overrides {
	return Overrides{
		PipelineBin:  os.Getenv(PipelineBinEnv),
		OxfmtBin:     os.Getenv(OxfmtBinEnv),
		OxlintBin:    os.Getenv(OxlintBinEnv),
		OxfmtConfig:  os.Getenv(OxfmtConfigEnv),
		OxlintConfig: os.Getenv(OxlintConfigEnv),
		SourcesCwd:   os.Getenv(SourcesCwdEnv),
	}
}
