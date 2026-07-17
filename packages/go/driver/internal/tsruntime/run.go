package tsruntime

import (
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

	Stdout io.Writer
	Stderr io.Writer
}

const (
	PipelineBinEnv  = "FMTKIT_TS_PIPELINE_BIN"
	OxfmtBinEnv     = "OXFMT_BIN"
	OxlintBinEnv    = "OXLINT_BIN"
	OxfmtConfigEnv  = "FMTKIT_OXFMTRC"
	OxlintConfigEnv = "FMTKIT_OXLINTRC"
	SourcesCwdEnv   = "FMTKIT_SOURCES_CWD"
)

// RunPipeline runs the full TS/Vue formatting pipeline (blank-lines -> oxfmt
// -> fluent-chains -> oxfmt -> validate-syntax).
func (s Support) RunPipeline(opts RunOptions) error {
	cwd, err := sourcesCwd()

	if err != nil {
		return err
	}

	formatFiles, warnings, err := collect(cwd, opts.Scopes, false)

	if err != nil {
		return err
	}

	for _, warning := range warnings {
		_, _ = fmt.Fprintf(opts.Stderr, "[sources] %s\n", warning)
	}

	syntaxFiles, _, err := collect(cwd, opts.Scopes, true)

	if err != nil {
		return err
	}

	oxfmtBin := os.Getenv(OxfmtBinEnv)

	args := []string{"pipeline"}

	if oxfmtBin != "" {
		args = append(args, "--oxfmt-bin", oxfmtBin)
	} else {
		args = append(args, "--oxfmt-bin", s.Sidecar())
	}

	if config := s.oxfmtConfigFor(cwd); config != "" {
		args = append(args, "--oxfmt-config", config)
	}

	args = append(args, "--format-files")
	args = append(args, formatFiles...)
	args = append(args, "--syntax-files")
	args = append(args, syntaxFiles...)

	return s.spawn(pipelineBin(s.Sidecar()), args, opts)
}

// RunLint lints the collected TS/Vue files with oxlint.
func (s Support) RunLint(opts RunOptions) error {
	cwd, err := sourcesCwd()

	if err != nil {
		return err
	}

	files, warnings, err := collect(cwd, opts.Scopes, false)

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

	bin := os.Getenv(OxlintBinEnv)

	if bin == "" {
		bin = s.Sidecar()
		args = append(args, "oxlint")
	}

	if config := s.oxlintConfigFor(cwd); config != "" {
		args = append(args, "--config", config)
	}

	args = append(args, files...)

	return s.spawn(bin, args, opts)
}

func pipelineBin(sidecar string) string {
	if bin := os.Getenv(PipelineBinEnv); bin != "" {
		return bin
	}

	return sidecar
}

func sourcesCwd() (string, error) {
	if cwd := os.Getenv(SourcesCwdEnv); cwd != "" {
		return cwd, nil
	}

	cwd, err := os.Getwd()

	if err != nil {
		return "", fmt.Errorf("resolve cwd: %w", err)
	}

	return cwd, nil
}

func collect(cwd string, scopes []string, includeDeclarations bool) ([]string, []string, error) {
	return sourcefiles.Collect(sourcefiles.Options{
		Cwd:                 cwd,
		IncludeDeclarations: includeDeclarations,
		Scopes:              scopes,
	})
}

// oxfmtConfigFor mirrors the entrypoint rule: a project-local .oxfmtrc.*
// wins via oxfmt's own auto-discovery, otherwise fall back to the bundled
// configuration.
func (s Support) oxfmtConfigFor(cwd string) string {
	if config := os.Getenv(OxfmtConfigEnv); config != "" {
		return existingFile(config)
	}

	if matches, err := filepath.Glob(filepath.Join(cwd, ".oxfmtrc.*")); err == nil && len(matches) > 0 {
		return ""
	}

	return s.OxfmtConfig()
}

// oxlintConfigFor treats both the extensionless .oxlintrc and .oxlintrc.* as
// project configuration.
func (s Support) oxlintConfigFor(cwd string) string {
	if config := os.Getenv(OxlintConfigEnv); config != "" {
		return existingFile(config)
	}

	if existingFile(filepath.Join(cwd, ".oxlintrc")) != "" {
		return ""
	}

	if matches, err := filepath.Glob(filepath.Join(cwd, ".oxlintrc.*")); err == nil && len(matches) > 0 {
		return ""
	}

	return s.OxlintConfig()
}

func (s Support) spawn(bin string, args []string, opts RunOptions) error {
	cmd := exec.Command(bin, args...)

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
