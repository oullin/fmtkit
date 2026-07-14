package pathx

import (
	"os"
	"path/filepath"
	"strings"
)

// Resolve returns an absolute cleaned runtime root, or an empty string when
// GO_FMT_RUNTIME_DIR is unset or cannot be resolved.
func Resolve() string {
	value := os.Getenv("GO_FMT_RUNTIME_DIR")

	if strings.TrimSpace(value) == "" {
		return ""
	}

	path, err := filepath.Abs(value)

	if err != nil {
		return ""
	}

	return filepath.Clean(path)
}

func Contains(root, path string) bool {
	if root == "" {
		return false
	}

	absPath, err := filepath.Abs(path)

	if err != nil {
		return false
	}

	rel, err := filepath.Rel(root, absPath)

	if err != nil {
		return false
	}

	return rel == "." || (rel != ".." && !strings.HasPrefix(rel, ".."+string(filepath.Separator)))
}
