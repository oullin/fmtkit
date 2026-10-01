package oxlintconfig

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func TestWithBatchesKeepsBundledPluginWhenJSONOverlayRepeatsAlias(t *testing.T) {
	root := t.TempDir()
	base := writeConfig(t, t.TempDir(), ".oxlintrc.json", `{"jsPlugins":[{"name":"demo","specifier":"./bundled.mjs"}]}`)
	writeConfig(t, root, ".oxlintrc.jsonc", `{"jsPlugins":[{"name":"demo","specifier":"./missing.mjs"}],"rules":{"demo/check":"off"}}`)
	file := writeConfig(t, root, "app.ts", "export const value = 1;\n")

	err := WithBatches(Request{RootDir: root, BundledConfig: base, Files: []string{file}}, func(batches []Batch) error {
		config := readGeneratedConfig(t, batches[0].ConfigPath)

		if len(config.Extends) != 1 || config.Extends[0] != base {
			t.Fatalf("extends = %q, want the bundled policy", config.Extends)
		}

		contents, err := os.ReadFile(batches[0].ConfigPath)

		if err != nil {
			t.Fatal(err)
		}

		var overlay struct {
			JSPlugins []json.RawMessage `json:"jsPlugins"`
			Rules     map[string]string `json:"rules"`
		}

		if err := json.Unmarshal(contents, &overlay); err != nil {
			t.Fatal(err)
		}

		if len(overlay.JSPlugins) != 0 || overlay.Rules["demo/check"] != "off" {
			t.Fatalf("prepared overlay = %+v, want duplicate plugin removed and rule override retained", overlay)
		}

		return nil
	})

	if err != nil {
		t.Fatal(err)
	}

	leftovers, err := filepath.Glob(filepath.Join(root, ".fmtkit-oxlint-*"))

	if err != nil {
		t.Fatal(err)
	}

	if len(leftovers) != 0 {
		t.Fatalf("composed configs were not removed: %q", leftovers)
	}
}
