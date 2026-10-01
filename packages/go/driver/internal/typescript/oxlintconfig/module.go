package oxlintconfig

import (
	"encoding/json"
	"fmt"
	"net/url"
	"os"
	"path/filepath"
	"strings"

	"github.com/tailscale/hujson"
)

type moduleLayer struct {
	path     string
	contents json.RawMessage
	module   bool
}

func isConfigModule(path string) bool {
	return strings.HasSuffix(path, ".ts") || strings.HasSuffix(path, ".mts")
}

func slicesContainsConfigModule(paths []string) bool {
	for _, path := range paths {
		if isConfigModule(path) {
			return true
		}
	}

	return false
}

// materialiseModule composes a JS config from original source modules and
// parsed JSON configs. Oxlint's JS config extends accepts objects, not paths.
func materialiseModule(root string, chain configChain) (string, error) {
	layers := make([]moduleLayer, 0, len(chain.overlays)+1)
	paths := append([]string{chain.base}, chain.overlays...)
	added := make(map[string]bool)

	for _, path := range paths {
		if err := appendModuleLayers(path, &layers, make(map[string]bool), added); err != nil {
			return "", err
		}
	}

	var source strings.Builder

	source.WriteString("import { createRequire } from 'node:module';\n")
	source.WriteString("import { dirname, isAbsolute, resolve } from 'node:path';\n")
	source.WriteString("import { pathToFileURL } from 'node:url';\n")
	source.WriteString(moduleAdapter)

	for index, layer := range layers {
		if layer.module {
			uri := (&url.URL{Scheme: "file", Path: filepath.ToSlash(layer.path)}).String()
			_, _ = fmt.Fprintf(&source, "import layer%d from %q;\n", index, uri)
		} else {
			_, _ = fmt.Fprintf(&source, "const layer%d = %s;\n", index, layer.contents)
		}
	}

	source.WriteString("const layers = [\n")

	for index, layer := range layers {
		_, _ = fmt.Fprintf(&source, "  adapt(layer%d, %q, %q),\n", index, layer.path, configPrefix(root, layer.path))
	}

	source.WriteString("];\nconst seenPlugins = new Set();\n")
	source.WriteString("for (let index = layers.length - 1; index >= 0; index--) layers[index] = dedupePlugins(layers[index], seenPlugins);\n")
	source.WriteString("export default { extends: layers };\n")

	file, err := os.CreateTemp(root, ".fmtkit-oxlint-*.mts")

	if err != nil {
		return "", fmt.Errorf("create imported Oxlint config in %q: %w", root, err)
	}

	path := file.Name()

	if _, err := file.WriteString(source.String()); err != nil {
		_ = file.Close()
		_ = os.Remove(path)

		return "", fmt.Errorf("write imported Oxlint config %q: %w", path, err)
	}

	if err := file.Close(); err != nil {
		_ = os.Remove(path)

		return "", fmt.Errorf("close imported Oxlint config %q: %w", path, err)
	}

	return path, nil
}

func appendModuleLayers(path string, layers *[]moduleLayer, visiting, added map[string]bool) error {
	path, err := filepath.Abs(path)

	if err != nil {
		return fmt.Errorf("resolve Oxlint config %q: %w", path, err)
	}

	if visiting[path] {
		return fmt.Errorf("Oxlint config extends cycle at %q", path)
	}

	if added[path] {
		return nil
	}

	if isConfigModule(path) {
		*layers = append(*layers, moduleLayer{path: path, module: true})
		added[path] = true

		return nil
	}

	visiting[path] = true

	defer delete(visiting, path)

	document, err := readJSONConfig(path)

	if err != nil {
		return err
	}

	var extended []string

	if raw, ok := document["extends"]; ok {
		if err := json.Unmarshal(raw, &extended); err != nil {
			return fmt.Errorf("parse Oxlint config %q extends: %w", path, err)
		}

		delete(document, "extends")
	}

	for _, relative := range extended {
		if !filepath.IsAbs(relative) && !strings.HasPrefix(relative, ".") {
			return fmt.Errorf("Oxlint JSON config %q extends package %q; use a TypeScript config for package imports", path, relative)
		}

		candidate := relative

		if !filepath.IsAbs(candidate) {
			candidate = filepath.Join(filepath.Dir(path), candidate)
		}

		if err := appendModuleLayers(candidate, layers, visiting, added); err != nil {
			return err
		}
	}

	contents, err := json.Marshal(document)

	if err != nil {
		return fmt.Errorf("encode Oxlint config %q: %w", path, err)
	}

	*layers = append(*layers, moduleLayer{path: path, contents: contents})
	added[path] = true

	return nil
}

func readJSONConfig(path string) (map[string]json.RawMessage, error) {
	data, err := os.ReadFile(path)

	if err != nil {
		return nil, fmt.Errorf("read Oxlint config %q: %w", path, err)
	}

	value, err := hujson.Parse(data)

	if err != nil {
		return nil, fmt.Errorf("parse Oxlint config %q: %w", path, err)
	}

	if hasTrailingComma(value) {
		return nil, fmt.Errorf("parse Oxlint config %q: trailing comma is not supported by Oxlint", path)
	}

	value.Standardize()

	var document map[string]json.RawMessage

	if err := json.Unmarshal(value.Pack(), &document); err != nil {
		return nil, fmt.Errorf("parse Oxlint config %q: %w", path, err)
	}

	return document, nil
}

func configPrefix(root, path string) string {
	relative, err := filepath.Rel(root, filepath.Dir(path))

	if err != nil || relative == "." || relative == ".." || strings.HasPrefix(relative, ".."+string(filepath.Separator)) {
		return ""
	}

	return filepath.ToSlash(relative)
}

const moduleAdapter = `
function dedupePlugins(config, seen) {
  const jsPlugins = config.jsPlugins?.filter((entry) => {
    const name = typeof entry === 'string' ? entry : entry.name;
    if (seen.has(name)) return false;
    seen.add(name);
    return true;
  });
  const extensions = config.extends?.slice();
  if (extensions) {
    for (let index = extensions.length - 1; index >= 0; index--) {
      extensions[index] = dedupePlugins(extensions[index], seen);
    }
  }
  return {
    ...config,
    ...(jsPlugins && { jsPlugins }),
    ...(extensions && { extends: extensions }),
  };
}

function adapt(config, source, prefix) {
  const directory = dirname(source);
  const requireFromSource = createRequire(pathToFileURL(source));
  const glob = (pattern) => {
    const negated = pattern.startsWith('!');
    const value = negated ? pattern.slice(1) : pattern;
    const rebased = prefix && !isAbsolute(value) ? prefix + '/' + value.replace(/^\.\//, '') : value;
    return negated ? '!' + rebased : rebased;
  };
  const plugin = (entry) => {
    const specifier = typeof entry === 'string' ? entry : entry.specifier;
    if (isAbsolute(specifier) || specifier.startsWith('file:')) return entry;
    const absolute = specifier.startsWith('./') || specifier.startsWith('../')
      ? resolve(directory, specifier)
      : requireFromSource.resolve(specifier);
    const resolved = pathToFileURL(absolute).href;
    return typeof entry === 'string' ? resolved : { ...entry, specifier: resolved };
  };
  return {
    ...config,
    ...(config.ignorePatterns && { ignorePatterns: config.ignorePatterns.map(glob) }),
    ...(config.jsPlugins && { jsPlugins: config.jsPlugins.map(plugin) }),
    ...(config.overrides && { overrides: config.overrides.map((override) => ({
      ...override,
      ...(override.files && { files: override.files.map(glob) }),
      ...(override.excludeFiles && { excludeFiles: override.excludeFiles.map(glob) }),
      ...(override.jsPlugins && { jsPlugins: override.jsPlugins.map(plugin) }),
    })) }),
  };
}
`
