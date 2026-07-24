// Package sourcefiles composes the three source-discovery engines — git file
// discovery (gitfiles), the extension taxonomy (filetypes), and the
// .prettierignore matcher (prettierignore) — into the file lists the TS
// toolchain formats and lints.
package sourcefiles

import (
	"context"
	"fmt"
	"os"
	"path/filepath"
	"slices"

	"go.ollin.sh/fmtkit/driver/internal/filetypes"
	"go.ollin.sh/fmtkit/driver/internal/gitfiles"
	"go.ollin.sh/fmtkit/driver/internal/prettierignore"
)

// Selection re-exports gitfiles.Selection so existing callers keep compiling.
//
// Transitional: G5 adopts gitfiles.Selection directly.
type Selection = gitfiles.Selection

// Collector composes git discovery, the extension taxonomy, and the
// .prettierignore matcher into the formatter's and linter's file lists.
type Collector struct {
	Tree      gitfiles.Tree
	Selection gitfiles.Selection
	Filter    filetypes.Filter
}

// Options configures the transitional Collect/CollectLintable wrappers.
//
// Transitional: G5 adopts Collector directly.
type Options struct {
	Cwd                 string
	IncludeDeclarations bool
	Scopes              []string

	// Selection defaults to SelectionAll.
	Selection Selection
}

const (
	// SelectionAll covers every non-ignored file: tracked plus untracked.
	//
	// Transitional: G5 adopts gitfiles.SelectionAll directly.
	SelectionAll = gitfiles.SelectionAll

	// SelectionChanged covers only what has diverged from HEAD.
	//
	// Transitional: G5 adopts gitfiles.SelectionChanged directly.
	SelectionChanged = gitfiles.SelectionChanged
)

// Formattable lists the files the formatter owns under the given scopes: the TS
// and Vue families plus the HTML and Markdown documents whose embedded scripts
// get formatted. It returns warnings for scopes that do not exist.
func (c Collector) Formattable(ctx context.Context, scopes []string) ([]string, []string, error) {
	return c.collect(ctx, scopes, c.Filter.Formattable)
}

// Lintable lists only the files oxlint can lint under the given scopes: the TS
// and Vue families. It is a subset of Formattable — HTML and Markdown are
// formattable but not lintable.
func (c Collector) Lintable(ctx context.Context, scopes []string) ([]string, []string, error) {
	return c.collect(ctx, scopes, c.Filter.Lintable)
}

func (c Collector) collect(ctx context.Context, scopes []string, keep func(string) bool) ([]string, []string, error) {
	cwd := c.Tree.Dir

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

		entries, err := c.Tree.Files(ctx, absolute, c.Selection)

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

	kept, err := c.honorPrettierIgnore(cwd, files)

	if err != nil {
		return nil, warnings, err
	}

	files = kept

	slices.Sort(files)

	return files, warnings, nil
}

// honorPrettierIgnore drops any collected path the project's .prettierignore
// excludes. When there is no .prettierignore, the files pass through unchanged.
func (c Collector) honorPrettierIgnore(cwd string, files []string) ([]string, error) {
	matcher, err := prettierignore.Load(filepath.Join(cwd, ".prettierignore"))

	if err != nil {
		return nil, err
	}

	if matcher == nil {
		return files, nil
	}

	return matcher.FilterAbs(cwd, files)
}

// Collect lists the files the formatter owns under the given scopes.
//
// Transitional: G5 adopts Collector directly.
func Collect(ctx context.Context, opts Options) ([]string, []string, error) {
	collector, err := collectorFor(opts)

	if err != nil {
		return nil, nil, err
	}

	return collector.Formattable(ctx, opts.Scopes)
}

// CollectLintable lists only the files oxlint can lint under the given scopes.
//
// Transitional: G5 adopts Collector directly.
func CollectLintable(ctx context.Context, opts Options) ([]string, []string, error) {
	collector, err := collectorFor(opts)

	if err != nil {
		return nil, nil, err
	}

	return collector.Lintable(ctx, opts.Scopes)
}

// ChangedPaths lists every file that diverges from HEAD under the given scopes,
// whatever its extension, skipping .prettierignore filtering.
//
// Transitional: G5 adopts gitfiles.Tree directly.
func ChangedPaths(ctx context.Context, cwd string, scopes []string) ([]string, error) {
	tree, err := gitfiles.NewTree(cwd)

	if err != nil {
		return nil, err
	}

	return tree.ChangedPaths(ctx, scopes)
}

func collectorFor(opts Options) (Collector, error) {
	tree, err := gitfiles.NewTree(opts.Cwd)

	if err != nil {
		return Collector{}, err
	}

	return Collector{
		Tree:      tree,
		Selection: opts.Selection,
		Filter:    filetypes.Filter{IncludeDeclarations: opts.IncludeDeclarations},
	}, nil
}
