// Package gitfiles discovers the files a working tree covers by driving git.
// It knows how to map a Selection onto the git invocations that list it, parse
// their NUL-separated output, and resolve those paths against a tree root. It
// carries no opinion about which files are worth formatting — that taxonomy
// lives in filetypes — nor about Prettier's ignore list.
package gitfiles

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

// Tree is a working-tree root that git commands run against.
type Tree struct {
	Dir string
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

// NewTree returns a Tree rooted at dir. An empty dir resolves to the current
// working directory, matching the behaviour of the callers that previously
// defaulted the root themselves.
func NewTree(dir string) (Tree, error) {
	if strings.TrimSpace(dir) == "" {
		cwd, err := os.Getwd()

		if err != nil {
			return Tree{}, err
		}

		dir = cwd
	}

	return Tree{Dir: dir}, nil
}

// Files lists the paths git reports for sel under scope, a single path git is
// passed as its pathspec. Entries come back exactly as git prints them —
// NUL-separated, relative to the tree — with no filtering or ordering applied.
func (t Tree) Files(ctx context.Context, scope string, sel Selection) ([]string, error) {
	entries := []string{}

	for _, args := range sel.gitCommands(scope) {
		found, err := t.runGit(ctx, args)

		if err != nil {
			return nil, err
		}

		entries = append(entries, found...)
	}

	return entries, nil
}

// ChangedPaths lists every file that diverges from HEAD — modified, staged, or
// added — under the given scopes, whatever its extension, as absolute cleaned
// paths, deduplicated and sorted. Callers do their own filtering — the Go
// formatter, for one, has its own notion of which files it owns.
//
// ChangedPaths deliberately skips .prettierignore filtering: it feeds the Go
// formatter, whose file set has nothing to do with Prettier's JS/TS ignore
// list.
func (t Tree) ChangedPaths(ctx context.Context, scopes []string) ([]string, error) {
	return t.collect(ctx, scopes, SelectionChanged)
}

// IntersectChanged narrows owned — the file list an engine reports it owns —
// down to the ones the working tree has touched under scopes.
//
// It intersects rather than asking git for the owned extensions directly: the
// engine's walk is what applies its own exclusions (vendor, not_path/not_name,
// generated files), and git knows nothing about those. Taking the engine's list
// and keeping only what git reports as changed preserves both. Outside a git
// work tree there is no such thing as "changed", so the error surfaces rather
// than silently formatting everything.
func (t Tree) IntersectChanged(ctx context.Context, scopes, owned []string) ([]string, error) {
	touched, err := t.ChangedPaths(ctx, scopes)

	if err != nil {
		return nil, err
	}

	changed := make(map[string]struct{}, len(touched))

	for _, path := range touched {
		changed[path] = struct{}{}
	}

	files := make([]string, 0, len(owned))

	for _, path := range owned {
		if _, ok := changed[path]; ok {
			files = append(files, path)
		}
	}

	return files, nil
}

// collect walks scopes, listing the files sel covers under each, and returns
// them as absolute cleaned paths, deduplicated and sorted. Missing scopes are
// skipped silently; any other stat failure surfaces.
func (t Tree) collect(ctx context.Context, scopes []string, sel Selection) ([]string, error) {
	if len(scopes) == 0 {
		scopes = []string{"."}
	}

	files := []string{}
	seen := map[string]struct{}{}

	for _, scope := range scopes {
		absolute := scope

		if !filepath.IsAbs(absolute) {
			absolute = filepath.Join(t.Dir, scope)
		}

		if _, err := os.Stat(absolute); err != nil {
			if os.IsNotExist(err) {
				continue
			}

			return nil, err
		}

		entries, err := t.Files(ctx, absolute, sel)

		if err != nil {
			return nil, err
		}

		for _, entry := range entries {
			path := entry

			if !filepath.IsAbs(path) {
				path = filepath.Join(t.Dir, path)
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

	return files, nil
}

func (t Tree) runGit(ctx context.Context, args []string) ([]string, error) {
	cmd := exec.CommandContext(ctx, "git", args...)
	cmd.Dir = t.Dir

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
