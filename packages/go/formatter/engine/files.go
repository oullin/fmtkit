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

func shouldSkipDir(path, root, name string, cfg config.Config) bool {
	if path != root && strings.HasPrefix(name, ".") {
		return true
	}

	return slices.Contains(cfg.Exclude, name)
}

func isExcludedFile(path string, cfg config.Config) bool {
	base := filepath.Base(path)

	nameExcluded := slices.ContainsFunc(cfg.NotName, func(pattern string) bool {
		matched, _ := filepath.Match(pattern, base)

		return matched
	})

	if nameExcluded {
		return true
	}

	slashed := filepath.ToSlash(path)

	return slices.ContainsFunc(cfg.NotPath, func(pattern string) bool {
		return strings.Contains(slashed, pattern)
	})
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
