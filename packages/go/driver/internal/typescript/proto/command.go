package proto

// The command types below build the exact argument vectors each sidecar mode
// expects. The bin resolution (which executable to spawn) is the caller's
// concern; these types own only the argv the sidecar itself parses.

// PipelineCommand describes a full-pipeline invocation. OxfmtBin is the
// already-resolved oxfmt executable the sidecar shells out to, and OxfmtConfig
// is the resolved config path, or "" to let oxfmt auto-discover.
type PipelineCommand struct {
	OxfmtBin    string
	OxfmtConfig string
	FormatFiles []string
	SyntaxFiles []string
}

// OxlintCommand describes an oxlint invocation. ViaSidecar is set when the
// sidecar dispatches oxlint (and so must be told the mode); a direct OXLINT_BIN
// override clears it. Config is the resolved config path, or "" for
// auto-discovery.
type OxlintCommand struct {
	ViaSidecar bool
	Fix        bool
	Config     string
	Files      []string
}

// ComplexityCommand describes a complexity-scan invocation. Root is the
// directory the reported paths are made relative to, and FilesFrom names a
// NUL-separated listing of the sources to score — a file rather than an argv
// tail, because a whole repository's worth of paths does not fit in one.
type ComplexityCommand struct {
	Root      string
	FilesFrom string
}

// MigrateCommand describes an `oxfmt --migrate=prettier` invocation. ViaSidecar
// is set when the sidecar dispatches oxfmt; a direct OXFMT_BIN override clears
// it.
type MigrateCommand struct {
	ViaSidecar bool
}

// Argv returns the pipeline mode's argument vector.
func (c PipelineCommand) Argv() []string {
	args := []string{ModePipeline}

	args = append(args, "--oxfmt-bin", c.OxfmtBin)

	if c.OxfmtConfig != "" {
		args = append(args, "--oxfmt-config", c.OxfmtConfig)
	}

	args = append(args, "--format-files")
	args = append(args, c.FormatFiles...)
	args = append(args, "--syntax-files")
	args = append(args, c.SyntaxFiles...)

	return args
}

// Argv returns oxlint's argument vector.
func (c OxlintCommand) Argv() []string {
	var args []string

	if c.ViaSidecar {
		args = append(args, ModeOxlint)
	}

	if c.Fix {
		args = append(args, "--fix")
	}

	if c.Config != "" {
		args = append(args, "--config", c.Config)
	}

	args = append(args, c.Files...)

	return args
}

// Argv returns the complexity mode's argument vector.
func (c ComplexityCommand) Argv() []string {
	return []string{ModeComplexity, "--root", c.Root, "--files-from", c.FilesFrom}
}

// Argv returns the migration argument vector.
func (c MigrateCommand) Argv() (args []string) {
	if c.ViaSidecar {
		args = append(args, ModeOxfmt)
	}

	args = append(args, "--migrate=prettier")

	return args
}
