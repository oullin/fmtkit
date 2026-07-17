package engine

import (
	"bytes"
	"os"
	"path/filepath"
	"slices"
	"strings"

	"go.ollin.sh/fmtkit/formatter/config"
)

// CollectGoFiles expands input paths into a sorted list of Go source files.
func CollectGoFiles(paths []string, cfg config.Config) ([]string, error) {
	if len(paths) == 0 {
		paths = []string{"."}
	}

	var files []string

	seen := map[string]struct{}{}

	for _, root := range paths {
		absRoot, err := filepath.Abs(root)

		if err != nil {
			return nil, err
		}

		info, err := os.Stat(absRoot)

		if err != nil {
			return nil, err
		}

		if !info.IsDir() {
			if isGoSource(absRoot) && !isExcludedFile(absRoot, cfg) {
				if _, ok := seen[absRoot]; !ok {
					files = append(files, absRoot)
					seen[absRoot] = struct{}{}
				}
			}

			continue
		}

		err = filepath.WalkDir(absRoot, func(path string, entry os.DirEntry, walkErr error) error {
			if walkErr != nil {
				return walkErr
			}

			if entry.IsDir() {
				if shouldSkipDir(path, absRoot, entry.Name(), cfg) {
					return filepath.SkipDir
				}

				return nil
			}

			if isGoSource(path) && !isExcludedFile(path, cfg) {
				if _, ok := seen[path]; !ok {
					files = append(files, path)
					seen[path] = struct{}{}
				}
			}

			return nil
		})

		if err != nil {
			return nil, err
		}
	}

	slices.Sort(files)

	return files, nil
}

func filterFiles(files, selected []string) []string {
	if len(files) == 0 || len(selected) == 0 {
		return nil
	}

	allowed := make(map[string]struct{}, len(selected))

	for _, path := range selected {
		allowed[path] = struct{}{}
	}

	filtered := make([]string, 0, len(files))

	for _, path := range files {
		if _, ok := allowed[path]; ok {
			filtered = append(filtered, path)
		}
	}

	return filtered
}

func shouldSkipDir(path, root, name string, cfg config.Config) bool {
	if path != root && strings.HasPrefix(name, ".") {
		return true
	}

	for _, excluded := range cfg.Exclude {
		if name == excluded {
			return true
		}
	}

	return false
}

func isExcludedFile(path string, cfg config.Config) bool {
	base := filepath.Base(path)

	for _, pattern := range cfg.NotName {
		matched, _ := filepath.Match(pattern, base)

		if matched {
			return true
		}
	}

	slashed := filepath.ToSlash(path)

	for _, pattern := range cfg.NotPath {
		if strings.Contains(slashed, pattern) {
			return true
		}
	}

	return false
}

func isGoSource(path string) bool {
	if filepath.Ext(path) != ".go" {
		return false
	}

	base := filepath.Base(path)

	if strings.HasPrefix(base, "Dockerfile") {
		return false
	}

	if strings.HasSuffix(base, ".gen.go") {
		return false
	}

	src, err := os.ReadFile(path)

	if err == nil && bytes.HasPrefix(src, []byte("// Code generated")) {
		return false
	}

	return true
}
