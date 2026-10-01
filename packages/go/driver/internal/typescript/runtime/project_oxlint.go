package runtime

import (
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strconv"
	"strings"
)

// projectOxlintExecutable finds the Node-based Oxlint installation required
// when an imported TypeScript config is part of the effective policy.
func projectOxlintExecutable(root, configDir, override string) (string, error) {
	if override != "" {
		return override, nil
	}

	if _, err := exec.LookPath("node"); err != nil {
		return "", fmt.Errorf("import-based Oxlint config requires Node.js 24+ on PATH: %w", err)
	}

	nodeVersion, err := exec.Command("node", "--version").Output()

	if err != nil || !versionAtLeast(strings.TrimSpace(strings.TrimPrefix(string(nodeVersion), "v")), 24, 0) {
		return "", fmt.Errorf("import-based Oxlint config requires Node.js 24+ (found %q)", strings.TrimSpace(string(nodeVersion)))
	}

	root, err = filepath.Abs(root)

	if err != nil {
		return "", fmt.Errorf("resolve project Oxlint root: %w", err)
	}

	start, err := filepath.Abs(configDir)

	if err != nil {
		return "", fmt.Errorf("resolve project Oxlint config directory: %w", err)
	}

	if relative, err := filepath.Rel(root, start); err != nil || relative == ".." || strings.HasPrefix(relative, ".."+string(filepath.Separator)) {
		start = root
	}

	for directory := start; ; directory = filepath.Dir(directory) {
		candidate := filepath.Join(directory, "node_modules", ".bin", "oxlint")

		if info, err := os.Stat(candidate); err == nil && info.Mode().IsRegular() {
			if err := checkProjectOxlintVersion(directory); err != nil {
				return "", err
			}

			return candidate, nil
		}

		if directory == root {
			break
		}
	}

	return "", fmt.Errorf("import-based Oxlint config requires a project-installed Node-based oxlint package under %q (or OXLINT_BIN)", root)
}

func checkProjectOxlintVersion(directory string) error {
	path := filepath.Join(directory, "node_modules", "oxlint", "package.json")
	data, err := os.ReadFile(path)

	if err != nil {
		return fmt.Errorf("read project-installed Oxlint package %q: %w", path, err)
	}

	var manifest struct {
		Version string `json:"version"`
	}

	if err := json.Unmarshal(data, &manifest); err != nil {
		return fmt.Errorf("parse project-installed Oxlint package %q: %w", path, err)
	}

	if !versionAtLeast(manifest.Version, 1, 80) {
		return fmt.Errorf("import-based Oxlint config requires oxlint 1.80.0+ (found %q in %q)", manifest.Version, path)
	}

	return nil
}

func versionAtLeast(version string, minimumMajor, minimumMinor int) bool {
	majorText, remainder, ok := strings.Cut(version, ".")

	if !ok {
		return false
	}

	minorText, _, _ := strings.Cut(remainder, ".")
	major, majorErr := strconv.Atoi(majorText)
	minor, minorErr := strconv.Atoi(minorText)

	return majorErr == nil && minorErr == nil && (major > minimumMajor || major == minimumMajor && minor >= minimumMinor)
}
