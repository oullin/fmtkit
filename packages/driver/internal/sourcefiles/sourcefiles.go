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
			return nil, warnings, err
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

func isTargetFile(path string, includeDeclarations bool) bool {
	if !strings.HasSuffix(path, ".ts") && !strings.HasSuffix(path, ".vue") {
		return false
	}

	return includeDeclarations || !strings.HasSuffix(path, ".d.ts")
}
