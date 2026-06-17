package vet

import (
	"bytes"
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

// Default returns the default vet configuration.
func Default() Config {
	return Config{Enabled: true}
}

var lookGoPath = func() (string, error) {
	return exec.LookPath("go")
}

// goBinary resolves the go executable via lookGoPath so the toolchain location is
// sourced consistently (and honours the lookGoPath stub in tests), falling back to
// the bare "go" name when resolution fails.
func goBinary() string {
	if path, err := lookGoPath(); err == nil {
		return path
	}

	return "go"
}

var goEnvOutput = func(workRoot string, keys ...string) ([]byte, error) {
	args := append([]string{"env"}, keys...)
	cmd := exec.Command(goBinary(), args...)
	cmd.Dir = workRoot

	return cmd.Output()
}

var goListModulesOutput = func(root string) ([]byte, error) {
	cmd := exec.Command(goBinary(), "list", "-f", "{{.Dir}}", "-m")
	cmd.Dir = root

	return cmd.Output()
}

// Run executes automatic go vet checks for the current module or workspace.
func Run(workRoot string, cfg Config) Report {
	if !cfg.Enabled {
		return Report{}
	}

	if _, err := lookGoPath(); err != nil {
		return Report{Skipped: true}
	}

	root, err := discoverVetRoot(workRoot)

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

	targets, err := discoverVetTargets(root)

	if err != nil {
		report.Errors = append(report.Errors, ErrorResult{
			File:    root,
			Message: err.Error(),
		})

		return report
	}

	for _, target := range targets {
		if vetError := runGoVet(target); vetError != nil {
			report.Errors = append(report.Errors, *vetError)
		}
	}

	return report
}

// ErrorCount returns the number of vet errors in the report.
func (r Report) ErrorCount() int {
	return len(r.Errors)
}

func discoverVetRoot(workRoot string) (string, error) {
	values, err := goEnv(workRoot, "GOWORK", "GOMOD")

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

func goEnv(workRoot string, keys ...string) ([]string, error) {
	out, err := goEnvOutput(workRoot, keys...)

	if err != nil {
		var exitErr *exec.ExitError
		label := strings.Join(keys, " ")

		if errors.As(err, &exitErr) {
			return nil, fmt.Errorf("resolve go %s: %s", label, strings.TrimSpace(string(exitErr.Stderr)))
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

func discoverVetTargets(root string) ([]string, error) {
	if strings.TrimSpace(root) == "" {
		return nil, nil
	}

	out, err := goListModulesOutput(root)

	if err != nil {
		var exitErr *exec.ExitError

		if errors.As(err, &exitErr) {
			return nil, fmt.Errorf("resolve go vet targets: %s", strings.TrimSpace(string(exitErr.Stderr)))
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

func runGoVet(root string) *ErrorResult {
	if strings.TrimSpace(root) == "" {
		return nil
	}

	cmd := exec.Command(goBinary(), "vet", "./...")
	cmd.Dir = root

	out, err := cmd.CombinedOutput()

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
