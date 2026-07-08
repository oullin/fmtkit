package sourcefiles

import (
	"bytes"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"slices"
	"strings"
)

type Options struct {
	Cwd                 string
	IncludeDeclarations bool
	Scopes              []string
}

func Collect(opts Options) ([]string, []string, error) {
	cwd := opts.Cwd

	if strings.TrimSpace(cwd) == "" {
		var err error

		cwd, err = os.Getwd()

		if err != nil {
			return nil, nil, err
		}
	}

	scopes := opts.Scopes

	if len(scopes) == 0 {
		scopes = []string{"."}
	}

	files := []string{}
	warnings := []string{}
	seen := map[string]struct{}{}

	for _, scope := range scopes {
		absolute := scope

		if !filepath.IsAbs(absolute) {
			absolute = filepath.Join(cwd, scope)
		}

		if _, err := os.Stat(absolute); err != nil {
			if os.IsNotExist(err) {
				warnings = append(warnings, fmt.Sprintf("path not found, skipping: %s", absolute))

				continue
			}

			return nil, warnings, err
		}

		entries, err := gitFiles(cwd, absolute)

		if err != nil {
			if !canUseFilesystemFallback(err) {
				return nil, warnings, err
			}

			entries, err = filesystemFiles(absolute)

			if err != nil {
				return nil, warnings, err
			}
		}

		for _, entry := range entries {
			if !isTargetFile(entry, opts.IncludeDeclarations) {
				continue
			}

			path := entry

			if !filepath.IsAbs(path) {
				path = filepath.Join(cwd, path)
			}

			path = filepath.Clean(path)

			if isRuntimePath(path) {
				continue
			}

			if _, ok := seen[path]; ok {
				continue
			}

			files = append(files, path)
			seen[path] = struct{}{}
		}
	}

	slices.Sort(files)

	return files, warnings, nil
}

func gitFiles(cwd, scope string) ([]string, error) {
	cmd := exec.Command("git", "ls-files", "--cached", "--others", "--exclude-standard", "-z", "--", scope)
	cmd.Dir = cwd

	var stderr bytes.Buffer

	cmd.Stderr = &stderr

	out, err := cmd.Output()

	if err != nil {
		reason := strings.TrimSpace(stderr.String())

		if reason == "" {
			reason = err.Error()
		}

		return nil, fmt.Errorf("git ls-files failed: %s", reason)
	}

	parts := bytes.Split(out, []byte{0})
	entries := make([]string, 0, len(parts))

	for _, part := range parts {
		if len(part) == 0 {
			continue
		}

		entries = append(entries, string(part))
	}

	return entries, nil
}

func filesystemFiles(scope string) ([]string, error) {
	info, err := os.Stat(scope)

	if err != nil {
		return nil, err
	}

	if !info.IsDir() {
		return []string{scope}, nil
	}

	entries := []string{}

	err = filepath.WalkDir(scope, func(path string, entry os.DirEntry, err error) error {
		if err != nil {
			return err
		}

		if entry.IsDir() {
			if shouldSkipFilesystemDir(path, entry) {
				return filepath.SkipDir
			}

			return nil
		}

		entries = append(entries, path)

		return nil
	})

	if err != nil {
		return nil, err
	}

	return entries, nil
}

func shouldSkipFilesystemDir(path string, entry os.DirEntry) bool {
	base := entry.Name()

	return strings.HasPrefix(base, ".") || base == "node_modules" || base == "vendor" || isRuntimePath(path)
}

func canUseFilesystemFallback(err error) bool {
	message := err.Error()

	return strings.Contains(message, "not a git repository") || strings.Contains(message, "executable file not found")
}

func isRuntimePath(path string) bool {
	runtimeDir := strings.TrimSpace(os.Getenv("GO_FMT_RUNTIME_DIR"))

	if runtimeDir == "" {
		return false
	}

	absRuntime, err := filepath.Abs(runtimeDir)

	if err != nil {
		return false
	}

	absPath, err := filepath.Abs(path)

	if err != nil {
		return false
	}

	rel, err := filepath.Rel(absRuntime, absPath)

	if err != nil {
		return false
	}

	return rel == "." || (rel != ".." && !strings.HasPrefix(rel, ".."+string(filepath.Separator)))
}

func isTargetFile(path string, includeDeclarations bool) bool {
	if !strings.HasSuffix(path, ".ts") && !strings.HasSuffix(path, ".vue") {
		return false
	}

	return includeDeclarations || !strings.HasSuffix(path, ".d.ts")
}
