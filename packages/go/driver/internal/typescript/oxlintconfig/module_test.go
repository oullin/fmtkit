package oxlintconfig

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestWithBatchesComposesImportedRootAndNestedJSON(t *testing.T) {
	root := t.TempDir()
	base := writeConfig(t, t.TempDir(), ".oxlintrc.json", `{"jsPlugins":[{"name":"demo","specifier":"./demo.mjs"}],"rules":{"eqeqeq":"error"}}`)
	writeConfig(t, root, "oxlint.config.ts", "export default { rules: { eqeqeq: 'off' } };\n")
	nested := filepath.Join(root, "package")
	writeConfig(t, nested, "local.json", `{"rules":{"no-console":"off"}}`)
	writeConfig(t, nested, ".oxlintrc.jsonc", `{"extends":["./local.json"],"rules":{"curly":"error"}}`)
	file := writeConfig(t, nested, "app.ts", "export const value = 1;\n")

	var generated string

	err := WithBatches(Request{RootDir: root, BundledConfig: base, Files: []string{file}}, func(batches []Batch) error {
		if len(batches) != 1 || !batches[0].ProjectOxlint {
			t.Fatalf("batches = %#v, want one imported-config batch", batches)
		}

		generated = batches[0].ConfigPath

		if filepath.Dir(generated) != root || filepath.Ext(generated) != ".mts" {
			t.Fatalf("generated config = %q, want a root .mts adapter", generated)
		}

		data, err := os.ReadFile(generated)

		if err != nil {
			t.Fatalf("read generated config: %v", err)
		}

		contents := string(data)

		for _, expected := range []string{"oxlint.config.ts", `"package"`, `"no-console"`, `"curly"`, "dedupePlugins"} {
			if !strings.Contains(contents, expected) {
				t.Errorf("generated config omits %q", expected)
			}
		}

		return nil
	})

	if err != nil {
		t.Fatalf("WithBatches: %v", err)
	}

	if _, err := os.Stat(generated); !os.IsNotExist(err) {
		t.Fatalf("imported config remains after callback: %v", err)
	}
}

func TestWithBatchesRejectsJSONAndImportedConfigInOneDirectory(t *testing.T) {
	root := t.TempDir()
	base := writeConfig(t, t.TempDir(), ".oxlintrc.json", `{}`)
	writeConfig(t, root, ".oxlintrc.json", `{}`)
	writeConfig(t, root, "oxlint.config.mts", "export default {};\n")
	file := writeConfig(t, root, "app.ts", "export const value = 1;\n")

	err := WithBatches(Request{RootDir: root, BundledConfig: base, Files: []string{file}}, func([]Batch) error { return nil })

	if err == nil || !strings.Contains(err.Error(), "multiple oxlint configs") {
		t.Fatalf("error = %v, want ambiguous-config failure", err)
	}
}

func TestWithBatchesDetectsImportedConfigJSONExtendsCycle(t *testing.T) {
	root := t.TempDir()
	base := writeConfig(t, t.TempDir(), ".oxlintrc.json", `{}`)
	writeConfig(t, root, "oxlint.config.ts", "export default {};\n")
	nested := filepath.Join(root, "nested")
	writeConfig(t, nested, "a.json", `{"extends":["./b.json"]}`)
	writeConfig(t, nested, "b.json", `{"extends":["./a.json"]}`)
	writeConfig(t, nested, ".oxlintrc.jsonc", `{"extends":["./a.json"]}`)
	file := writeConfig(t, nested, "app.ts", "export const value = 1;\n")

	err := WithBatches(Request{RootDir: root, BundledConfig: base, Files: []string{file}}, func([]Batch) error { return nil })

	if err == nil || !strings.Contains(err.Error(), "extends cycle") {
		t.Fatalf("error = %v, want JSON extends-cycle failure", err)
	}
}
