package sidecarproto

import "testing"

// TestWireConstantsAreFrozen pins every wire value byte-for-byte. The TS sidecar
// parses these; a change here that is not mirrored in packages/ts/sidecar breaks
// compatibility silently, so this test is the tripwire.
func TestWireConstantsAreFrozen(t *testing.T) {
	cases := map[string]string{
		"SidecarName":     SidecarName,
		"OxfmtRCName":     OxfmtRCName,
		"OxlintRCName":    OxlintRCName,
		"ModePipeline":    ModePipeline,
		"ModeOxfmt":       ModeOxfmt,
		"ModeOxlint":      ModeOxlint,
		"SupportDirEnv":   SupportDirEnv,
		"SidecarModeEnv":  SidecarModeEnv,
		"PipelineBinEnv":  PipelineBinEnv,
		"OxfmtBinEnv":     OxfmtBinEnv,
		"OxlintBinEnv":    OxlintBinEnv,
		"OxfmtConfigEnv":  OxfmtConfigEnv,
		"OxlintConfigEnv": OxlintConfigEnv,
		"SourcesCwdEnv":   SourcesCwdEnv,
	}

	want := map[string]string{
		"SidecarName":     "fmtkit-ts-sidecar",
		"OxfmtRCName":     ".oxfmtrc.json",
		"OxlintRCName":    ".oxlintrc.json",
		"ModePipeline":    "pipeline",
		"ModeOxfmt":       "oxfmt",
		"ModeOxlint":      "oxlint",
		"SupportDirEnv":   "FMTKIT_SUPPORT_DIR",
		"SidecarModeEnv":  "FMTKIT_SIDECAR_MODE",
		"PipelineBinEnv":  "FMTKIT_TS_PIPELINE_BIN",
		"OxfmtBinEnv":     "OXFMT_BIN",
		"OxlintBinEnv":    "OXLINT_BIN",
		"OxfmtConfigEnv":  "FMTKIT_OXFMTRC",
		"OxlintConfigEnv": "FMTKIT_OXLINTRC",
		"SourcesCwdEnv":   "FMTKIT_SOURCES_CWD",
	}

	for name, got := range cases {
		if got != want[name] {
			t.Errorf("%s = %q, want %q", name, got, want[name])
		}
	}
}

func TestReadOverridesReadsEveryVar(t *testing.T) {
	t.Setenv(PipelineBinEnv, "/bin/pipeline")
	t.Setenv(OxfmtBinEnv, "/bin/oxfmt")
	t.Setenv(OxlintBinEnv, "/bin/oxlint")
	t.Setenv(OxfmtConfigEnv, "/cfg/oxfmt.json")
	t.Setenv(OxlintConfigEnv, "/cfg/oxlint.json")
	t.Setenv(SourcesCwdEnv, "/work")

	want := Overrides{
		PipelineBin:  "/bin/pipeline",
		OxfmtBin:     "/bin/oxfmt",
		OxlintBin:    "/bin/oxlint",
		OxfmtConfig:  "/cfg/oxfmt.json",
		OxlintConfig: "/cfg/oxlint.json",
		SourcesCwd:   "/work",
	}

	if got := ReadOverrides(); got != want {
		t.Fatalf("ReadOverrides() = %+v, want %+v", got, want)
	}
}

func TestReadOverridesDefaultsToEmpty(t *testing.T) {
	for _, name := range []string{
		PipelineBinEnv, OxfmtBinEnv, OxlintBinEnv,
		OxfmtConfigEnv, OxlintConfigEnv, SourcesCwdEnv,
	} {
		t.Setenv(name, "")
	}

	if got := ReadOverrides(); got != (Overrides{}) {
		t.Fatalf("ReadOverrides() = %+v, want zero value", got)
	}
}
