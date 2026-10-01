package oxlintconfig

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
)

type jsonPluginRewriter struct {
	seen      map[string]bool
	resolved  map[string]string
	visiting  map[string]bool
	temporary []string
}

// Oxlint registers JS plugin names globally across a JSON extends chain. Keep
// the bundled copy when a project config repeats an alias, so ordinary linting
// never depends on the project's plugin installation.
func dedupeJSONPlugins(chain configChain) (configChain, []string, error) {
	rewriter := jsonPluginRewriter{
		seen:      make(map[string]bool),
		resolved:  make(map[string]string),
		visiting:  make(map[string]bool),
		temporary: make([]string, 0),
	}

	base, err := rewriter.rewrite(chain.base)

	if err != nil {
		return chain, rewriter.temporary, err
	}

	chain.base = base

	for index, path := range chain.overlays {
		chain.overlays[index], err = rewriter.rewrite(path)

		if err != nil {
			return chain, rewriter.temporary, err
		}
	}

	return chain, rewriter.temporary, nil
}

func (r *jsonPluginRewriter) rewrite(path string) (string, error) {
	path, err := filepath.Abs(path)

	if err != nil {
		return "", fmt.Errorf("resolve Oxlint config %q: %w", path, err)
	}

	if r.visiting[path] {
		return "", fmt.Errorf("Oxlint config extends cycle at %q", path)
	}

	if resolved, ok := r.resolved[path]; ok {
		return resolved, nil
	}

	r.visiting[path] = true

	defer delete(r.visiting, path)

	document, err := readJSONConfig(path)

	if err != nil {
		return "", err
	}

	changed := false

	if raw, ok := document["extends"]; ok {
		var extended []string

		if err := json.Unmarshal(raw, &extended); err != nil {
			return "", fmt.Errorf("parse Oxlint config %q extends: %w", path, err)
		}

		for index, entry := range extended {
			if !filepath.IsAbs(entry) && !strings.HasPrefix(entry, ".") {
				continue
			}

			candidate := entry

			if !filepath.IsAbs(candidate) {
				candidate = filepath.Join(filepath.Dir(path), candidate)
			}

			resolved, err := r.rewrite(candidate)

			if err != nil {
				return "", err
			}

			if resolved != candidate {
				extended[index] = resolved
				changed = true
			}
		}

		if changed {
			document["extends"], err = json.Marshal(extended)

			if err != nil {
				return "", fmt.Errorf("encode Oxlint config %q extends: %w", path, err)
			}
		}
	}

	if raw, ok := document["jsPlugins"]; ok {
		var plugins []json.RawMessage

		if err := json.Unmarshal(raw, &plugins); err != nil {
			return "", fmt.Errorf("parse Oxlint config %q JS plugins: %w", path, err)
		}

		unique := make([]json.RawMessage, 0, len(plugins))

		for _, plugin := range plugins {
			name, err := jsonPluginName(plugin)

			if err != nil {
				return "", fmt.Errorf("parse Oxlint config %q JS plugin: %w", path, err)
			}

			if r.seen[name] {
				changed = true

				continue
			}

			r.seen[name] = true
			unique = append(unique, plugin)
		}

		if changed {
			document["jsPlugins"], err = json.Marshal(unique)

			if err != nil {
				return "", fmt.Errorf("encode Oxlint config %q JS plugins: %w", path, err)
			}
		}
	}

	if !changed {
		r.resolved[path] = path

		return path, nil
	}

	contents, err := json.MarshalIndent(document, "", "\t")

	if err != nil {
		return "", fmt.Errorf("encode Oxlint config %q: %w", path, err)
	}

	file, err := os.CreateTemp(filepath.Dir(path), ".fmtkit-oxlint-layer-*.json")

	if err != nil {
		return "", fmt.Errorf("create Oxlint plugin layer beside %q: %w", path, err)
	}

	temporary := file.Name()
	r.temporary = append(r.temporary, temporary)

	if _, err := file.Write(append(contents, '\n')); err != nil {
		_ = file.Close()

		return "", fmt.Errorf("write Oxlint plugin layer %q: %w", temporary, err)
	}

	if err := file.Close(); err != nil {
		return "", fmt.Errorf("close Oxlint plugin layer %q: %w", temporary, err)
	}

	r.resolved[path] = temporary

	return temporary, nil
}

func jsonPluginName(raw json.RawMessage) (string, error) {
	var specifier string

	if err := json.Unmarshal(raw, &specifier); err == nil {
		return specifier, nil
	}

	var plugin struct {
		Name string `json:"name"`
	}

	if err := json.Unmarshal(raw, &plugin); err != nil {
		return "", err
	}

	if plugin.Name == "" {
		return "", fmt.Errorf("JS plugin name is empty")
	}

	return plugin.Name, nil
}
