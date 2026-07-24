package tsruntime

import (
	"context"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"

	"go.ollin.sh/fmtkit/driver/internal/sidecarproto"
	"go.ollin.sh/fmtkit/driver/internal/sourcefiles"
)

// Request describes one TS toolchain invocation.
type Request struct {
	// Scopes are the paths to process, defaulting to ".".
	Scopes []string

	// Selection is how much of the working tree to cover within Scopes. It
	// defaults to sourcefiles.SelectionAll.
	Selection sourcefiles.Selection

	// Fix, when set, lets RunLint apply oxlint's safe fixes (--fix) rather
	// than only reporting violations.
	Fix bool

	Stdout io.Writer
	Stderr io.Writer
}

// Invoker spawns the TS toolchain against an extracted Assets directory. It
// resolves the environment overrides once at construction rather than ad hoc
// deep in the call paths.
type Invoker struct {
	Assets Assets
	Env    sidecarproto.Overrides
}

// NewInvoker builds an Invoker for the given assets, reading the environment
// overrides once.
func NewInvoker(a Assets) Invoker {
	return Invoker{Assets: a, Env: sidecarproto.ReadOverrides()}
}

// RunPipeline runs the full TS/Vue formatting pipeline (blank-lines -> oxfmt
// -> fluent-chains -> blank-lines -> validate-syntax). oxfmt is an internal
// normalising step, not the last word: the project passes run after it and
// own the final style.
func (i Invoker) RunPipeline(ctx context.Context, req Request) error {
	cwd, err := i.sourcesCwd()

	if err != nil {
		return err
	}

	formatFiles, warnings, err := collect(ctx, cwd, req.Scopes, false, req.Selection)

	if err != nil {
		return err
	}

	for _, warning := range warnings {
		_, _ = fmt.Fprintf(req.Stderr, "[sources] %s\n", warning)
	}

	syntaxFiles, _, err := collect(ctx, cwd, req.Scopes, true, req.Selection)

	if err != nil {
		return err
	}

	oxfmtBin := i.Env.OxfmtBin

	if oxfmtBin == "" {
		oxfmtBin = i.Assets.Sidecar()
	}

	command := sidecarproto.PipelineCommand{
		OxfmtBin:    oxfmtBin,
		OxfmtConfig: i.oxfmtConfigFor(ctx, cwd, req.Stderr),
		FormatFiles: formatFiles,
		SyntaxFiles: syntaxFiles,
	}

	return i.spawn(ctx, i.pipelineExecutable(), command.Argv(), req)
}

// RunLint lints the collected TS/Vue files with oxlint. With req.Fix it applies
// oxlint's safe fixes in place; otherwise it only reports violations.
func (i Invoker) RunLint(ctx context.Context, req Request) error {
	cwd, err := i.sourcesCwd()

	if err != nil {
		return err
	}

	files, warnings, err := collectLintable(ctx, cwd, req.Scopes, false, req.Selection)

	if err != nil {
		return err
	}

	for _, warning := range warnings {
		_, _ = fmt.Fprintf(req.Stderr, "[sources] %s\n", warning)
	}

	if len(files) == 0 {
		_, _ = fmt.Fprintln(req.Stdout, "[lint] no TS/Vue files to lint.")

		return nil
	}

	bin := i.Env.OxlintBin
	viaSidecar := bin == ""

	if viaSidecar {
		bin = i.Assets.Sidecar()
	}

	command := sidecarproto.OxlintCommand{
		ViaSidecar: viaSidecar,
		Fix:        req.Fix,
		Config:     i.oxlintConfigFor(cwd),
		Files:      files,
	}

	return i.spawn(ctx, bin, command.Argv(), req)
}

// pipelineExecutable resolves the executable spawned for the pipeline: a
// FMTKIT_TS_PIPELINE_BIN override, otherwise the sidecar.
func (i Invoker) pipelineExecutable() string {
	if i.Env.PipelineBin != "" {
		return i.Env.PipelineBin
	}

	return i.Assets.Sidecar()
}

func (i Invoker) sourcesCwd() (string, error) {
	if i.Env.SourcesCwd != "" {
		return i.Env.SourcesCwd, nil
	}

	cwd, err := os.Getwd()

	if err != nil {
		return "", fmt.Errorf("resolve cwd: %w", err)
	}

	return cwd, nil
}

func collect(ctx context.Context, cwd string, scopes []string, includeDeclarations bool, selection sourcefiles.Selection) ([]string, []string, error) {
	return sourcefiles.Collect(ctx, sourcefiles.Options{
		Cwd:                 cwd,
		IncludeDeclarations: includeDeclarations,
		Scopes:              scopes,
		Selection:           selection,
	})
}

func collectLintable(ctx context.Context, cwd string, scopes []string, includeDeclarations bool, selection sourcefiles.Selection) ([]string, []string, error) {
	return sourcefiles.CollectLintable(ctx, sourcefiles.Options{
		Cwd:                 cwd,
		IncludeDeclarations: includeDeclarations,
		Scopes:              scopes,
		Selection:           selection,
	})
}

// oxfmtConfigFor resolves the oxfmt config by precedence: the FMTKIT_OXFMTRC
// override, then a project-local .oxfmtrc.* (via oxfmt's own auto-discovery,
// signalled by ""), then a config derived from the project's Prettier
// configuration, and finally the bundled default.
func (i Invoker) oxfmtConfigFor(ctx context.Context, cwd string, stderr io.Writer) string {
	if i.Env.OxfmtConfig != "" {
		return existingFile(i.Env.OxfmtConfig)
	}

	if matches, err := filepath.Glob(filepath.Join(cwd, ".oxfmtrc.*")); err == nil && len(matches) > 0 {
		return ""
	}

	if derived := i.migration().DerivedConfig(ctx, cwd, stderr); derived != "" {
		return derived
	}

	return i.Assets.OxfmtConfig()
}

// oxlintConfigFor treats both the extensionless .oxlintrc and .oxlintrc.* as
// project configuration.
func (i Invoker) oxlintConfigFor(cwd string) string {
	if i.Env.OxlintConfig != "" {
		return existingFile(i.Env.OxlintConfig)
	}

	if existingFile(filepath.Join(cwd, ".oxlintrc")) != "" {
		return ""
	}

	if matches, err := filepath.Glob(filepath.Join(cwd, ".oxlintrc.*")); err == nil && len(matches) > 0 {
		return ""
	}

	return i.Assets.OxlintConfig()
}

// migration views this invoker as the PrettierMigration that shares its assets
// and environment. The two carry the same data; if their fields ever diverge
// the compiler rejects this conversion, which is the intended tripwire.
func (i Invoker) migration() PrettierMigration {
	return PrettierMigration(i)
}

func (i Invoker) spawn(ctx context.Context, bin string, args []string, req Request) error {
	cmd := exec.CommandContext(ctx, bin, args...)

	cmd.Stdout = req.Stdout
	cmd.Stderr = req.Stderr

	// Match the container entrypoints: let git treat any working tree as safe
	// so file collection inside bind mounts and caches works.
	cmd.Env = append(os.Environ(),
		"GIT_CONFIG_COUNT=1",
		"GIT_CONFIG_KEY_0=safe.directory",
		"GIT_CONFIG_VALUE_0=*",
	)

	return cmd.Run()
}
