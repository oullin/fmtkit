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

// Selection is how much of the working tree a collection covers.
type Selection int

type Options struct {
	Cwd                 string
	IncludeDeclarations bool
	Scopes              []string

	// Selection defaults to SelectionAll.
	Selection Selection
}

const (
	// SelectionAll covers every non-ignored file: tracked plus untracked.
	// This is what `format-all` runs against.
	SelectionAll Selection = iota

	// SelectionChanged covers only what the working tree has actually touched:
	// modified-but-tracked plus untracked. This is what `format` runs against,
	// so an everyday format stays proportional to the diff rather than the repo.
	SelectionChanged
)

// gitArgs returns the `git ls-files` selection flags for s.
func (s Selection) gitArgs() []string {
	if s == SelectionChanged {
		// --modified is relative to the index, so a staged-and-unmodified file is
		// deliberately not "changed" here.
		return []string{"--others", "--modified"}
	}

	return []string{"--cached", "--others"}
}

// Collect lists the TS/Vue files under the given scopes.
func Collect(opts Options) ([]string, []string, error) {
	return collect(opts.Cwd, opts.Scopes, opts.Selection, func(path string) bool {
		return isTargetFile(path, opts.IncludeDeclarations)
	})
}

// ChangedPaths lists every file the working tree has modified or added under
// the given scopes, whatever its extension. Callers do their own filtering —
// the Go formatter, for one, has its own notion of which files it owns.
func ChangedPaths(cwd string, scopes []string) ([]string, error) {
	files, _, err := collect(cwd, scopes, SelectionChanged, func(string) bool {
		return true
	})

	return files, err
}

func collect(cwd string, scopes []string, selection Selection, keep func(string) bool) ([]string, []string, error) {
	if strings.TrimSpace(cwd) == "" {
		var err error

		cwd, err = os.Getwd()

		if err != nil {
			return nil, nil, err
		}
	}

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

		entries, err := gitFiles(cwd, absolute, selection)

		if err != nil {
			return nil, warnings, err
		}

		for _, entry := range entries {
			if !keep(entry) {
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

func gitFiles(cwd, scope string, selection Selection) ([]string, error) {
	args := []string{"ls-files"}
	args = append(args, selection.gitArgs()...)
	args = append(args, "--exclude-standard", "-z", "--", scope)

	cmd := exec.Command("git", args...)
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
