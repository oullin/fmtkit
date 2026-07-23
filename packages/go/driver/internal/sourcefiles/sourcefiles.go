package sourcefiles

import (
	"bytes"
	"context"
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

	// SelectionChanged covers only what has actually diverged from HEAD:
	// modified-but-tracked (staged or not) plus untracked. This is what `format`
	// runs against, so an everyday format stays proportional to the diff rather
	// than the repo.
	SelectionChanged
)

// gitCommands returns the git invocations whose combined output lists the
// files s covers under scope. Every command prints NUL-separated paths
// relative to the directory git runs in.
func (s Selection) gitCommands(scope string) [][]string {
	if s == SelectionChanged {
		return [][]string{
			// Untracked files, plus tracked ones whose working-tree copy differs
			// from the index.
			{"ls-files", "--others", "--modified", "--exclude-standard", "-z", "--", scope},

			// Staged changes are invisible to ls-files' worktree-vs-index view —
			// a pre-commit hook would otherwise see nothing to format — so they
			// come from an index-vs-HEAD diff. --relative keeps paths cwd-relative
			// like ls-files; --diff-filter=d drops staged deletions, which leave
			// no file to format.
			{"diff", "--cached", "--name-only", "--relative", "--diff-filter=d", "-z", "--", scope},
		}
	}

	return [][]string{{"ls-files", "--cached", "--others", "--exclude-standard", "-z", "--", scope}}
}

// Collect lists the files the formatter owns under the given scopes: the TS
// and Vue families plus the HTML and Markdown documents whose embedded scripts
// get formatted.
func Collect(ctx context.Context, opts Options) ([]string, []string, error) {
	return collect(ctx, opts.Cwd, opts.Scopes, opts.Selection, func(path string) bool {
		return isTargetFile(path, opts.IncludeDeclarations)
	})
}

// CollectLintable lists only the files oxlint can lint under the given scopes:
// the TS and Vue families. It is a subset of Collect — HTML and Markdown are
// formattable but not lintable.
func CollectLintable(ctx context.Context, opts Options) ([]string, []string, error) {
	return collect(ctx, opts.Cwd, opts.Scopes, opts.Selection, func(path string) bool {
		return isLintableFile(path, opts.IncludeDeclarations)
	})
}

// ChangedPaths lists every file that diverges from HEAD — modified, staged, or
// added — under the given scopes, whatever its extension. Callers do their own
// filtering — the Go formatter, for one, has its own notion of which files it
// owns.
func ChangedPaths(ctx context.Context, cwd string, scopes []string) ([]string, error) {
	files, _, err := collect(ctx, cwd, scopes, SelectionChanged, func(string) bool {
		return true
	})

	return files, err
}

func collect(ctx context.Context, cwd string, scopes []string, selection Selection, keep func(string) bool) ([]string, []string, error) {
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

		entries, err := gitFiles(ctx, cwd, absolute, selection)

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

func gitFiles(ctx context.Context, cwd, scope string, selection Selection) ([]string, error) {
	entries := []string{}

	for _, args := range selection.gitCommands(scope) {
		found, err := runGit(ctx, cwd, args)

		if err != nil {
			return nil, err
		}

		entries = append(entries, found...)
	}

	return entries, nil
}

func runGit(ctx context.Context, cwd string, args []string) ([]string, error) {
	cmd := exec.CommandContext(ctx, "git", args...)
	cmd.Dir = cwd

	var stderr bytes.Buffer

	cmd.Stderr = &stderr

	out, err := cmd.Output()

	if err != nil {
		reason := strings.TrimSpace(stderr.String())

		if reason == "" {
			return nil, fmt.Errorf("git %s failed: %w", args[0], err)
		}

		return nil, fmt.Errorf("git %s failed: %s: %w", args[0], reason, err)
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

// targetSuffixes are the extensions the sidecar knows how to format. The .ts
// entry also covers .d.ts; whether declarations are kept is decided separately.
var targetSuffixes = []string{".ts", ".vue", ".html", ".htm", ".md", ".markdown"}

func isTargetFile(path string, includeDeclarations bool) bool {
	matched := false

	for _, suffix := range targetSuffixes {
		if strings.HasSuffix(path, suffix) {
			matched = true

			break
		}
	}

	if !matched {
		return false
	}

	return includeDeclarations || !strings.HasSuffix(path, ".d.ts")
}

// isLintableFile reports whether oxlint can lint path: the TS family (minus
// declarations unless includeDeclarations) and Vue, but not HTML or Markdown.
func isLintableFile(path string, includeDeclarations bool) bool {
	if !strings.HasSuffix(path, ".ts") && !strings.HasSuffix(path, ".vue") {
		return false
	}

	return includeDeclarations || !strings.HasSuffix(path, ".d.ts")
}
