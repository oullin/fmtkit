// Package planner selects and classifies files for the contained formatter.
package planner

import (
	"bytes"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"slices"
	"strings"

	"github.com/oullin/fmtkit/packages/runtimex/pathx"
)

type Options struct {
	Cwd    string
	Scopes []string
}

type Plan struct {
	Go []string
	TS []string
}

// Build selects changed files when no scopes are supplied in a Git checkout.
// Explicit scopes and non-Git directories are scanned in full.
func Build(opts Options) (Plan, error) {
	cwd := opts.Cwd

	if strings.TrimSpace(cwd) == "" {
		var err error

		cwd, err = os.Getwd()

		if err != nil {
			return Plan{}, fmt.Errorf("resolve current directory: %w", err)
		}
	}

	cwd, err := filepath.Abs(cwd)

	if err != nil {
		return Plan{}, fmt.Errorf("resolve current directory: %w", err)
	}

	if resolved, resolveErr := filepath.EvalSymlinks(cwd); resolveErr == nil {
		cwd = resolved
	}

	var candidates []string

	if len(opts.Scopes) == 0 {
		root, ok, discoveryErr := gitRoot(cwd)

		if discoveryErr != nil {
			return Plan{}, discoveryErr
		}

		if ok {
			candidates, err = changedFiles(root, cwd)
		} else {
			candidates, err = scanScopes(cwd, []string{"."})
		}
	} else {
		candidates, err = scanScopes(cwd, opts.Scopes)
	}

	if err != nil {
		return Plan{}, err
	}

	return classify(cwd, candidates), nil
}

func gitRoot(cwd string) (string, bool, error) {
	cmd := exec.Command("git", "rev-parse", "--show-toplevel")
	cmd.Dir = cwd

	var stderr bytes.Buffer

	cmd.Stderr = &stderr
	out, err := cmd.Output()

	if err != nil {
		if apparentGitCheckout(cwd) {
			reason := strings.TrimSpace(stderr.String())

			if reason == "" {
				reason = err.Error()
			}

			return "", false, fmt.Errorf("git rev-parse failed in an apparent checkout: %s", reason)
		}

		return "", false, nil
	}

	root := strings.TrimSpace(string(out))

	if root == "" {
		return "", false, fmt.Errorf("git rev-parse returned an empty repository root")
	}

	return filepath.Clean(root), true, nil
}

func apparentGitCheckout(cwd string) bool {
	for current := filepath.Clean(cwd); ; current = filepath.Dir(current) {
		if _, err := os.Lstat(filepath.Join(current, ".git")); err == nil || !os.IsNotExist(err) {
			return true
		}

		parent := filepath.Dir(current)

		if parent == current {
			return false
		}
	}
}

func changedFiles(root, cwd string) ([]string, error) {
	cmd := exec.Command("git", "status", "--porcelain=v1", "-z", "--untracked-files=all", "--ignored=no")
	cmd.Dir = root

	var stderr bytes.Buffer

	cmd.Stderr = &stderr
	out, err := cmd.Output()

	if err != nil {
		reason := strings.TrimSpace(stderr.String())

		if reason == "" {
			reason = err.Error()
		}

		return nil, fmt.Errorf("git status failed: %s", reason)
	}

	parts := bytes.Split(out, []byte{0})
	files := make([]string, 0, len(parts))

	for index := 0; index < len(parts); index++ {
		entry := parts[index]

		if len(entry) < 4 {
			continue
		}

		status := string(entry[:2])
		path := string(entry[3:])

		if status[0] == 'R' || status[0] == 'C' || status[1] == 'R' || status[1] == 'C' {
			index++ // porcelain -z follows a rename/copy destination with its source.
		}

		absolute := filepath.Clean(filepath.Join(root, filepath.FromSlash(path)))

		if !contains(cwd, absolute) {
			continue
		}

		info, statErr := os.Lstat(absolute)

		if statErr != nil || !info.Mode().IsRegular() {
			continue
		}

		files = append(files, absolute)
	}

	return files, nil
}

func scanScopes(cwd string, scopes []string) ([]string, error) {
	files := []string{}
	runtimeDirs := containedRuntimeDirs()

	for _, scope := range scopes {
		absolute := scope

		if !filepath.IsAbs(absolute) {
			absolute = filepath.Join(cwd, scope)
		}

		absolute = filepath.Clean(absolute)
		info, err := os.Lstat(absolute)

		if err != nil {
			return nil, fmt.Errorf("scan %s: %w", scope, err)
		}

		if info.Mode()&os.ModeSymlink != 0 {
			continue
		}

		if !info.IsDir() {
			if info.Mode().IsRegular() {
				files = append(files, absolute)
			}

			continue
		}

		err = filepath.WalkDir(absolute, func(path string, entry os.DirEntry, walkErr error) error {
			if walkErr != nil {
				return walkErr
			}

			if entry.IsDir() {
				if shouldSkipDir(path, absolute, entry.Name(), runtimeDirs) {
					return filepath.SkipDir
				}

				return nil
			}

			if entry.Type()&os.ModeSymlink != 0 {
				return nil
			}

			files = append(files, path)

			return nil
		})

		if err != nil {
			return nil, fmt.Errorf("scan %s: %w", scope, err)
		}
	}

	return files, nil
}

func containedRuntimeDirs() []string {
	dirs := []string{}

	if configured := pathx.Resolve(); configured != "" {
		dirs = append(dirs, resolveExistingPath(configured))
	}

	if cacheRoot, err := os.UserCacheDir(); err == nil {
		defaultCache := resolveExistingPath(filepath.Join(cacheRoot, "go-fmt", "contained"))

		if len(dirs) == 0 || dirs[0] != defaultCache {
			dirs = append(dirs, defaultCache)
		}
	}

	return dirs
}

func resolveExistingPath(path string) string {
	if resolved, err := filepath.EvalSymlinks(path); err == nil {
		return resolved
	}

	return filepath.Clean(path)
}

func shouldSkipDir(path, root, name string, runtimeDirs []string) bool {
	if containsAnyRuntimeDir(runtimeDirs, path) {
		return true
	}

	if path == root {
		return false
	}

	return strings.HasPrefix(name, ".") || name == "node_modules" || name == "vendor"
}

func containsAnyRuntimeDir(runtimeDirs []string, path string) bool {
	for _, runtimeDir := range runtimeDirs {
		if pathx.Contains(runtimeDir, path) {
			return true
		}
	}

	return false
}

func classify(cwd string, candidates []string) Plan {
	plan := Plan{}
	seen := map[string]struct{}{}
	runtimeDirs := containedRuntimeDirs()

	for _, candidate := range candidates {
		absolute := filepath.Clean(candidate)

		if containsAnyRuntimeDir(runtimeDirs, absolute) {
			continue
		}

		info, err := os.Lstat(absolute)

		if err != nil || !info.Mode().IsRegular() {
			continue
		}

		if _, ok := seen[absolute]; ok {
			continue
		}

		seen[absolute] = struct{}{}
		target := displayPath(cwd, absolute)

		switch {
		case isEligibleGo(absolute):
			plan.Go = append(plan.Go, target)
		case strings.HasSuffix(absolute, ".ts"), strings.HasSuffix(absolute, ".vue"):
			plan.TS = append(plan.TS, target)
		}
	}

	slices.Sort(plan.Go)

	slices.Sort(plan.TS)

	return plan
}

func isEligibleGo(path string) bool {
	if filepath.Ext(path) != ".go" || strings.HasSuffix(filepath.Base(path), ".gen.go") {
		return false
	}

	source, err := os.ReadFile(path)

	return err == nil && !bytes.HasPrefix(source, []byte("// Code generated"))
}

func displayPath(cwd, absolute string) string {
	relative, err := filepath.Rel(cwd, absolute)

	if err == nil && relative != ".." && !strings.HasPrefix(relative, ".."+string(filepath.Separator)) {
		first := relative

		if separator := strings.IndexRune(relative, filepath.Separator); separator >= 0 {
			first = relative[:separator]
		}

		if strings.HasPrefix(first, "-") {
			return "." + string(filepath.Separator) + relative
		}

		return relative
	}

	return absolute
}

func contains(root, path string) bool {
	relative, err := filepath.Rel(root, path)

	return err == nil && relative != ".." && !strings.HasPrefix(relative, ".."+string(filepath.Separator))
}
