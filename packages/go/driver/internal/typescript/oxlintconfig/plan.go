// Package oxlintconfig composes fmtkit's bundled Oxlint policy with the
// project configurations that apply to each linted file.
package oxlintconfig

import (
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strings"

	"github.com/tailscale/hujson"
)

// Request describes one Oxlint configuration-planning operation.
type Request struct {
	RootDir       string
	BundledConfig string
	GlobalOverlay string
	Files         []string
}

// Batch is a disjoint set of files that share one effective Oxlint config.
type Batch struct {
	ConfigPath       string
	Files            []string
	ProjectOxlint    bool
	ProjectConfigDir string
}

type configChain struct {
	base     string
	overlays []string
	files    []string
}

type discoveredConfig struct {
	path  string
	found bool
	err   error
}

const explicitConfigName = "FMTKIT_OXLINTRC"

var discoveredConfigNames = []string{".oxlintrc", ".oxlintrc.json", ".oxlintrc.jsonc", "oxlint.config.ts", "oxlint.config.mts"}

// WithBatches prepares deterministic Oxlint batches, runs use, and removes all
// generated entry configs before returning.
func WithBatches(req Request, use func([]Batch) error) (err error) {
	if use == nil {
		return errors.New("use Oxlint config batches: callback is nil")
	}

	root, err := filepath.Abs(req.RootDir)

	if err != nil {
		return fmt.Errorf("resolve lint root %q: %w", req.RootDir, err)
	}

	base, err := regularFile(req.BundledConfig, "bundled oxlint config")

	if err != nil {
		return err
	}

	global, err := optionalGlobalOverlay(root, req.GlobalOverlay)

	if err != nil {
		return err
	}

	discovery := make(map[string]discoveredConfig)
	rootConfig, rootFound, err := configInDirectory(root, discovery)

	if err != nil {
		return err
	}

	chains := make(map[string]*configChain)

	for _, file := range req.Files {
		absoluteFile, err := containedFile(root, file)

		if err != nil {
			return err
		}

		nearest, nearestFound, err := nearestConfig(root, filepath.Dir(absoluteFile), discovery)

		if err != nil {
			return err
		}

		overlays := make([]string, 0, 3)

		if rootFound {
			overlays = append(overlays, rootConfig)
		}

		if nearestFound && nearest != rootConfig {
			overlays = append(overlays, nearest)
		}

		if global != "" {
			overlays = append(overlays, global)
		}

		key := strings.Join(overlays, "\x00")
		chain := chains[key]

		if chain == nil {
			chain = &configChain{base: base, overlays: overlays}
			chains[key] = chain
		}

		chain.files = append(chain.files, absoluteFile)
	}

	keys := make([]string, 0, len(chains))

	for key := range chains {
		keys = append(keys, key)
	}

	sort.Strings(keys)

	batches := make([]Batch, 0, len(keys))
	generated := make([]string, 0, len(keys))

	defer func() {
		err = errors.Join(err, removeGenerated(generated))
	}()

	for _, key := range keys {
		chain := chains[key]

		sort.Strings(chain.files)

		configPath := base

		projectOxlint := slicesContainsConfigModule(chain.overlays)
		projectConfigDir := root

		if len(chain.overlays) > 0 {
			if projectOxlint {
				configPath, err = materialiseModule(root, *chain)
			} else {
				prepared, temporary, prepareErr := dedupeJSONPlugins(*chain)
				generated = append(generated, temporary...)

				if prepareErr != nil {
					return prepareErr
				}

				configPath, err = materialise(prepared)
			}

			if err != nil {
				return err
			}

			generated = append(generated, configPath)
			projectConfigDir = filepath.Dir(chain.overlays[len(chain.overlays)-1])
		}

		batches = append(batches, Batch{
			ConfigPath:       configPath,
			Files:            append([]string(nil), chain.files...),
			ProjectOxlint:    projectOxlint,
			ProjectConfigDir: projectConfigDir,
		})
	}

	return use(batches)
}

func optionalGlobalOverlay(root, path string) (string, error) {
	if path == "" {
		return "", nil
	}

	if !filepath.IsAbs(path) {
		path = filepath.Join(root, path)
	}

	return regularFile(path, explicitConfigName)
}

func regularFile(path, label string) (string, error) {
	absolute, err := filepath.Abs(path)

	if err != nil {
		return "", fmt.Errorf("resolve %s %q: %w", label, path, err)
	}

	info, err := os.Stat(absolute)

	if err != nil {
		return "", fmt.Errorf("read %s %q: %w", label, absolute, err)
	}

	if !info.Mode().IsRegular() {
		return "", fmt.Errorf("read %s %q: not a regular file", label, absolute)
	}

	file, err := os.Open(absolute)

	if err != nil {
		return "", fmt.Errorf("read %s %q: %w", label, absolute, err)
	}

	if err := file.Close(); err != nil {
		return "", fmt.Errorf("close %s %q: %w", label, absolute, err)
	}

	return absolute, nil
}

func containedFile(root, path string) (string, error) {
	if !filepath.IsAbs(path) {
		path = filepath.Join(root, path)
	}

	absolute, err := filepath.Abs(path)

	if err != nil {
		return "", fmt.Errorf("resolve lint file %q: %w", path, err)
	}

	relative, err := filepath.Rel(root, absolute)

	if err != nil {
		return "", fmt.Errorf("locate lint file %q under %q: %w", absolute, root, err)
	}

	if relative == ".." || strings.HasPrefix(relative, ".."+string(filepath.Separator)) {
		return "", fmt.Errorf("lint file %q is outside root %q", absolute, root)
	}

	return absolute, nil
}

func nearestConfig(root, directory string, cache map[string]discoveredConfig) (string, bool, error) {
	for {
		config, found, err := configInDirectory(directory, cache)

		if err != nil || found {
			return config, found, err
		}

		if directory == root {
			return "", false, nil
		}

		parent := filepath.Dir(directory)

		if parent == directory {
			return "", false, fmt.Errorf("resolve oxlint config for %q: reached filesystem root before %q", directory, root)
		}

		directory = parent
	}
}

func configInDirectory(directory string, cache map[string]discoveredConfig) (string, bool, error) {
	if cached, ok := cache[directory]; ok {
		return cached.path, cached.found, cached.err
	}

	matches := make([]string, 0, 1)

	for _, name := range discoveredConfigNames {
		path := filepath.Join(directory, name)
		info, err := os.Stat(path)

		switch {
		case err == nil && info.Mode().IsRegular():
			matches = append(matches, path)
		case err == nil:
			result := discoveredConfig{err: fmt.Errorf("oxlint config %q is not a regular file", path)}
			cache[directory] = result

			return "", false, result.err
		case !os.IsNotExist(err):
			result := discoveredConfig{err: fmt.Errorf("inspect oxlint config %q: %w", path, err)}
			cache[directory] = result

			return "", false, result.err
		}
	}

	if len(matches) > 1 {
		result := discoveredConfig{err: fmt.Errorf("multiple oxlint configs in %q: %s", directory, strings.Join(matches, ", "))}
		cache[directory] = result

		return "", false, result.err
	}

	result := discoveredConfig{}

	if len(matches) == 1 {
		result.path = matches[0]
		result.found = true
	}

	cache[directory] = result

	return result.path, result.found, nil
}

func materialise(chain configChain) (string, error) {
	highest := chain.overlays[len(chain.overlays)-1]
	data, err := os.ReadFile(highest)

	if err != nil {
		return "", fmt.Errorf("read oxlint overlay %q: %w", highest, err)
	}

	documentSyntax, err := hujson.Parse(data)

	if err != nil {
		return "", fmt.Errorf("parse oxlint overlay %q: %w", highest, err)
	}

	if hasTrailingComma(documentSyntax) {
		return "", fmt.Errorf("parse oxlint overlay %q: trailing comma is not supported by Oxlint", highest)
	}

	documentSyntax.Standardize()
	standard := documentSyntax.Pack()

	var document map[string]json.RawMessage

	if err := json.Unmarshal(standard, &document); err != nil {
		return "", fmt.Errorf("parse oxlint overlay %q: %w", highest, err)
	}

	originalExtends := []string{}

	if raw, ok := document["extends"]; ok {
		if err := json.Unmarshal(raw, &originalExtends); err != nil {
			return "", fmt.Errorf("parse oxlint overlay %q extends: %w", highest, err)
		}
	}

	lowerLayers := make([]string, 0, len(chain.overlays))
	lowerLayers = append(lowerLayers, chain.base)
	lowerLayers = append(lowerLayers, chain.overlays[:len(chain.overlays)-1]...)

	lower := make([]string, 0, len(lowerLayers)+len(originalExtends))

	for _, path := range lowerLayers {
		if !extendsPath(originalExtends, filepath.Dir(highest), path) {
			lower = append(lower, path)
		}
	}

	lower = append(lower, originalExtends...)

	extends, err := json.Marshal(lower)

	if err != nil {
		return "", fmt.Errorf("encode oxlint overlay %q extends: %w", highest, err)
	}

	document["extends"] = extends

	composed, err := json.MarshalIndent(document, "", "\t")

	if err != nil {
		return "", fmt.Errorf("encode composed oxlint config for %q: %w", highest, err)
	}

	composed = append(composed, '\n')

	file, err := os.CreateTemp(filepath.Dir(highest), ".fmtkit-oxlint-*.json")

	if err != nil {
		return "", fmt.Errorf("create composed oxlint config beside %q: %w", highest, err)
	}

	path := file.Name()
	keep := false

	defer func() {
		if !keep {
			_ = os.Remove(path)
		}
	}()

	if err := file.Chmod(0o600); err != nil {
		_ = file.Close()

		return "", fmt.Errorf("secure composed oxlint config %q: %w", path, err)
	}

	if _, err := file.Write(composed); err != nil {
		_ = file.Close()

		return "", fmt.Errorf("write composed oxlint config %q: %w", path, err)
	}

	if err := file.Close(); err != nil {
		return "", fmt.Errorf("close composed oxlint config %q: %w", path, err)
	}

	keep = true

	return path, nil
}

func extendsPath(extends []string, configDirectory, candidate string) bool {
	for _, path := range extends {
		if !filepath.IsAbs(path) {
			path = filepath.Join(configDirectory, path)
		}

		if samePath(path, candidate) {
			return true
		}
	}

	return false
}

func samePath(left, right string) bool {
	left = filepath.Clean(left)
	right = filepath.Clean(right)

	resolvedLeft, leftErr := filepath.EvalSymlinks(left)
	resolvedRight, rightErr := filepath.EvalSymlinks(right)

	if leftErr == nil && rightErr == nil {
		return resolvedLeft == resolvedRight
	}

	return left == right
}

func hasTrailingComma(value hujson.Value) bool {
	switch value := value.Value.(type) {
	case *hujson.Object:
		if len(value.Members) > 0 && value.Members[len(value.Members)-1].Value.AfterExtra != nil {
			return true
		}

		for _, member := range value.Members {
			if hasTrailingComma(member.Value) {
				return true
			}
		}
	case *hujson.Array:
		if len(value.Elements) > 0 && value.Elements[len(value.Elements)-1].AfterExtra != nil {
			return true
		}

		for _, element := range value.Elements {
			if hasTrailingComma(element) {
				return true
			}
		}
	}

	return false
}

func removeGenerated(paths []string) error {
	var errs []error

	for _, path := range paths {
		if err := os.Remove(path); err != nil && !os.IsNotExist(err) {
			errs = append(errs, fmt.Errorf("remove composed oxlint config %q: %w", path, err))
		}
	}

	return errors.Join(errs...)
}
