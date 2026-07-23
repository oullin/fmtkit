package tsruntime

import (
	"context"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"

	"go.ollin.sh/fmtkit/driver/internal/sourcefiles"
)

// RunOptions describes one TS toolchain invocation.
type RunOptions struct {
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

// overrides carries the environment overrides a TS toolchain invocation
// honours, resolved once at the entry points rather than ad hoc deep in the
// call paths.
type overrides struct {
	pipelineBin  string
	oxfmtBin     string
	oxlintBin    string
	oxfmtConfig  string
	oxlintConfig string
	sourcesCwd   string
}

const (
	PipelineBinEnv  = "FMTKIT_TS_PIPELINE_BIN"
	OxfmtBinEnv     = "OXFMT_BIN"
	OxlintBinEnv    = "OXLINT_BIN"
	OxfmtConfigEnv  = "FMTKIT_OXFMTRC"
	OxlintConfigEnv = "FMTKIT_OXLINTRC"
	SourcesCwdEnv   = "FMTKIT_SOURCES_CWD"
)

// readOverrides gathers every environment override in one place.
func readOverrides() overrides {
	return overrides{
		pipelineBin:  os.Getenv(PipelineBinEnv),
		oxfmtBin:     os.Getenv(OxfmtBinEnv),
		oxlintBin:    os.Getenv(OxlintBinEnv),
		oxfmtConfig:  os.Getenv(OxfmtConfigEnv),
		oxlintConfig: os.Getenv(OxlintConfigEnv),
		sourcesCwd:   os.Getenv(SourcesCwdEnv),
	}
}

// RunPipeline runs the full TS/Vue formatting pipeline (blank-lines -> oxfmt
// -> fluent-chains -> blank-lines -> validate-syntax). oxfmt is an internal
// normalising step, not the last word: the project passes run after it and
// own the final style.
func (s Support) RunPipeline(ctx context.Context, opts RunOptions) error {
	env := readOverrides()

	cwd, err := sourcesCwd(env)

	if err != nil {
		return err
	}

	formatFiles, warnings, err := collect(ctx, cwd, opts.Scopes, false, opts.Selection)

	if err != nil {
		return err
	}

	for _, warning := range warnings {
		_, _ = fmt.Fprintf(opts.Stderr, "[sources] %s\n", warning)
	}

	syntaxFiles, _, err := collect(ctx, cwd, opts.Scopes, true, opts.Selection)

	if err != nil {
		return err
	}

	args := []string{"pipeline"}

	if env.oxfmtBin != "" {
		args = append(args, "--oxfmt-bin", env.oxfmtBin)
	} else {
		args = append(args, "--oxfmt-bin", s.Sidecar())
	}

	if config := s.oxfmtConfigFor(cwd, env); config != "" {
		args = append(args, "--oxfmt-config", config)
	}

	args = append(args, "--format-files")
	args = append(args, formatFiles...)
	args = append(args, "--syntax-files")
	args = append(args, syntaxFiles...)

	return s.spawn(ctx, pipelineBin(env, s.Sidecar()), args, opts)
}

// RunLint lints the collected TS/Vue files with oxlint. With opts.Fix it applies
// oxlint's safe fixes in place; otherwise it only reports violations.
func (s Support) RunLint(ctx context.Context, opts RunOptions) error {
	env := readOverrides()

	cwd, err := sourcesCwd(env)

	if err != nil {
		return err
	}

	files, warnings, err := collect(ctx, cwd, opts.Scopes, false, opts.Selection)

	if err != nil {
		return err
	}

	for _, warning := range warnings {
		_, _ = fmt.Fprintf(opts.Stderr, "[sources] %s\n", warning)
	}

	if len(files) == 0 {
		_, _ = fmt.Fprintln(opts.Stdout, "[lint] no TS/Vue files to lint.")

		return nil
	}

	var args []string

	bin := env.oxlintBin

	if bin == "" {
		bin = s.Sidecar()
		args = append(args, "oxlint")
	}

	if opts.Fix {
		args = append(args, "--fix")
	}

	if config := s.oxlintConfigFor(cwd, env); config != "" {
		args = append(args, "--config", config)
	}

	args = append(args, files...)

	return s.spawn(ctx, bin, args, opts)
}

func pipelineBin(env overrides, sidecar string) string {
	if env.pipelineBin != "" {
		return env.pipelineBin
	}

	return sidecar
}

func sourcesCwd(env overrides) (string, error) {
	if env.sourcesCwd != "" {
		return env.sourcesCwd, nil
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

// oxfmtConfigFor mirrors the entrypoint rule: a project-local .oxfmtrc.*
// wins via oxfmt's own auto-discovery, otherwise fall back to the bundled
// configuration.
func (s Support) oxfmtConfigFor(cwd string, env overrides) string {
	if env.oxfmtConfig != "" {
		return existingFile(env.oxfmtConfig)
	}

	if matches, err := filepath.Glob(filepath.Join(cwd, ".oxfmtrc.*")); err == nil && len(matches) > 0 {
		return ""
	}

	return s.OxfmtConfig()
}

// oxlintConfigFor treats both the extensionless .oxlintrc and .oxlintrc.* as
// project configuration.
func (s Support) oxlintConfigFor(cwd string, env overrides) string {
	if env.oxlintConfig != "" {
		return existingFile(env.oxlintConfig)
	}

	if existingFile(filepath.Join(cwd, ".oxlintrc")) != "" {
		return ""
	}

	if matches, err := filepath.Glob(filepath.Join(cwd, ".oxlintrc.*")); err == nil && len(matches) > 0 {
		return ""
	}

	return s.OxlintConfig()
}

func (s Support) spawn(ctx context.Context, bin string, args []string, opts RunOptions) error {
	cmd := exec.CommandContext(ctx, bin, args...)

	cmd.Stdout = opts.Stdout
	cmd.Stderr = opts.Stderr

	// Match the container entrypoints: let git treat any working tree as safe
	// so file collection inside bind mounts and caches works.
	cmd.Env = append(os.Environ(),
		"GIT_CONFIG_COUNT=1",
		"GIT_CONFIG_KEY_0=safe.directory",
		"GIT_CONFIG_VALUE_0=*",
	)

	return cmd.Run()
}
