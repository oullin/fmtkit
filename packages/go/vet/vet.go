package vet

import (
	"bytes"
	"context"
	"errors"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
)

// Config controls whether automatic go vet checks run.
type Config struct {
	Enabled bool
}

// ErrorResult describes a go vet failure for a module or workspace.
type ErrorResult struct {
	File    string `json:"file,omitempty"`
	Message string `json:"message"`
}

// Report summarizes the automatic go vet run.
type Report struct {
	Root    string        `json:"root,omitempty"`
	Skipped bool          `json:"skipped,omitempty"`
	Errors  []ErrorResult `json:"errors,omitempty"`
}

// toolchain abstracts the Go toolchain invocations the vet run needs, so tests
// can drive run with a fake instead of swapping package-level state.
type toolchain interface {
	LookGo() (string, error)
	EnvOutput(ctx context.Context, dir string, keys ...string) ([]byte, error)
	ListModulesOutput(ctx context.Context, dir string) ([]byte, error)
	VetOutput(ctx context.Context, dir string) ([]byte, error)
}

// execToolchain is the exec-backed toolchain used outside tests.
type execToolchain struct{}

// LookGo resolves the go executable on PATH.
func (execToolchain) LookGo() (string, error) {
	return exec.LookPath("go")
}

// EnvOutput runs `go env` for the given keys in dir.
func (t execToolchain) EnvOutput(ctx context.Context, dir string, keys ...string) ([]byte, error) {
	args := append([]string{"env"}, keys...)
	cmd := exec.CommandContext(ctx, t.goBinary(), args...)
	cmd.Dir = dir

	return cmd.Output()
}

// ListModulesOutput lists the module directories of the module or workspace
// rooted at dir.
func (t execToolchain) ListModulesOutput(ctx context.Context, dir string) ([]byte, error) {
	cmd := exec.CommandContext(ctx, t.goBinary(), "list", "-f", "{{.Dir}}", "-m")
	cmd.Dir = dir

	return cmd.Output()
}

// VetOutput runs `go vet ./...` in dir and returns its combined output.
func (t execToolchain) VetOutput(ctx context.Context, dir string) ([]byte, error) {
	cmd := exec.CommandContext(ctx, t.goBinary(), "vet", "./...")
	cmd.Dir = dir

	return cmd.CombinedOutput()
}

// goBinary resolves the go executable via LookGo so the toolchain location is
// sourced consistently, falling back to the bare "go" name when resolution
// fails.
func (t execToolchain) goBinary() string {
	if path, err := t.LookGo(); err == nil {
		return path
	}

	return "go"
}

// Run executes automatic go vet checks for the current module or workspace.
func Run(ctx context.Context, workRoot string, cfg Config) Report {
	return run(ctx, workRoot, cfg, execToolchain{})
}

func run(ctx context.Context, workRoot string, cfg Config, tc toolchain) Report {
	if !cfg.Enabled {
		return Report{}
	}

	if _, err := tc.LookGo(); err != nil {
		return Report{Skipped: true}
	}

	root, err := discoverVetRoot(ctx, workRoot, tc)

	if err != nil {
		return Report{
			Errors: []ErrorResult{{
				File:    workRoot,
				Message: err.Error(),
			}},
		}
	}

	if strings.TrimSpace(root) == "" {
		return Report{}
	}

	report := Report{Root: root}

	targets, err := discoverVetTargets(ctx, root, tc)

	if err != nil {
		report.Errors = append(report.Errors, ErrorResult{
			File:    root,
			Message: err.Error(),
		})

		return report
	}

	for _, target := range targets {
		if vetError := runGoVet(ctx, target, tc); vetError != nil {
			report.Errors = append(report.Errors, *vetError)
		}
	}

	return report
}

// ErrorCount returns the number of vet errors in the report.
func (r Report) ErrorCount() int {
	return len(r.Errors)
}

func discoverVetRoot(ctx context.Context, workRoot string, tc toolchain) (string, error) {
	values, err := goEnv(ctx, workRoot, tc, "GOWORK", "GOMOD")

	if err != nil {
		return "", err
	}

	if root, ok := existingGoRoot(values[0], "go.work"); ok {
		return root, nil
	}

	if root, ok := existingGoRoot(values[1], "go.mod"); ok {
		return root, nil
	}

	return "", nil
}

func goEnv(ctx context.Context, workRoot string, tc toolchain, keys ...string) ([]string, error) {
	out, err := tc.EnvOutput(ctx, workRoot, keys...)

	if err != nil {
		var exitErr *exec.ExitError

		label := strings.Join(keys, " ")

		if errors.As(err, &exitErr) {
			return nil, fmt.Errorf("resolve go %s: %s: %w", label, strings.TrimSpace(string(exitErr.Stderr)), err)
		}

		return nil, fmt.Errorf("resolve go %s: %w", label, err)
	}

	return parseGoEnvValues(out, len(keys)), nil
}

func parseGoEnvValues(out []byte, count int) []string {
	lines := strings.Split(string(out), "\n")

	if len(lines) > 0 && lines[len(lines)-1] == "" {
		lines = lines[:len(lines)-1]
	}

	values := make([]string, count)

	for i := 0; i < count && i < len(lines); i++ {
		values[i] = strings.TrimSuffix(lines[i], "\r")
	}

	return values
}

func existingGoRoot(path string, filename string) (string, bool) {
	if path == "" || path == "off" {
		return "", false
	}

	if filepath.Clean(path) == filepath.Clean(os.DevNull) {
		return "", false
	}

	info, err := os.Stat(path)

	if err != nil || info.IsDir() || filepath.Base(path) != filename {
		return "", false
	}

	return filepath.Dir(path), true
}

func discoverVetTargets(ctx context.Context, root string, tc toolchain) ([]string, error) {
	if strings.TrimSpace(root) == "" {
		return nil, nil
	}

	out, err := tc.ListModulesOutput(ctx, root)

	if err != nil {
		var exitErr *exec.ExitError

		if errors.As(err, &exitErr) {
			return nil, fmt.Errorf("resolve go vet targets: %s: %w", strings.TrimSpace(string(exitErr.Stderr)), err)
		}

		return nil, fmt.Errorf("resolve go vet targets: %w", err)
	}

	lines := strings.Split(string(out), "\n")
	targets := make([]string, 0, len(lines))
	seen := make(map[string]struct{}, len(lines))

	for _, line := range lines {
		target := strings.TrimSpace(strings.TrimSuffix(line, "\r"))

		if target == "" {
			continue
		}

		if _, ok := seen[target]; ok {
			continue
		}

		seen[target] = struct{}{}
		targets = append(targets, target)
	}

	return targets, nil
}

func runGoVet(ctx context.Context, root string, tc toolchain) *ErrorResult {
	if strings.TrimSpace(root) == "" {
		return nil
	}

	out, err := tc.VetOutput(ctx, root)

	if err == nil {
		return nil
	}

	message := "automatic go vet ./... failed"
	trimmed := strings.TrimSpace(string(bytes.TrimSpace(out)))

	if trimmed != "" {
		message = fmt.Sprintf("%s:\n%s", message, trimmed)
	} else {
		message = fmt.Sprintf("%s: %v", message, err)
	}

	return &ErrorResult{
		File:    root,
		Message: message,
	}
}
