package tsruntime

import (
	"bytes"
	"context"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"go.ollin.sh/fmtkit/driver/internal/sidecarproto"
)

// writeMigrateStub creates a fake oxfmt that, on --migrate=prettier, writes an
// .oxfmtrc.json into its working directory carrying a marker so tests can prove
// migration ran. It counts invocations in a sibling file so cache hits are
// observable.
func writeMigrateStub(t *testing.T, path string) {
	t.Helper()

	counter := filepath.Join(filepath.Dir(path), "migrate-count")

	script := "#!/bin/sh\n" +
		"printf 'x' >> '" + counter + "'\n" +
		"echo '{\"derived\":true}' > \"$PWD/.oxfmtrc.json\"\n"

	if err := os.WriteFile(path, []byte(script), 0o755); err != nil {
		t.Fatalf("write migrate stub %s: %v", path, err)
	}
}

func migrateInvocations(t *testing.T, stubDir string) int {
	t.Helper()

	data, err := os.ReadFile(filepath.Join(stubDir, "migrate-count"))

	if os.IsNotExist(err) {
		return 0
	}

	if err != nil {
		t.Fatalf("read migrate count: %v", err)
	}

	return len(data)
}

func TestDetectPrettierConfigFilenames(t *testing.T) {
	names := []string{
		".prettierrc",
		".prettierrc.json",
		".prettierrc.yml",
		".prettierrc.yaml",
		".prettierrc.json5",
		".prettierrc.js",
		".prettierrc.cjs",
		".prettierrc.mjs",
		".prettierrc.toml",
		"prettier.config.js",
		"prettier.config.cjs",
		"prettier.config.mjs",
	}

	for _, name := range names {
		t.Run(name, func(t *testing.T) {
			dir := t.TempDir()

			path := filepath.Join(dir, name)

			if err := os.WriteFile(path, []byte("{}"), 0o644); err != nil {
				t.Fatalf("write %s: %v", name, err)
			}

			if got := detectPrettierConfig(dir); got != path {
				t.Fatalf("detectPrettierConfig = %q, want %q", got, path)
			}
		})
	}
}

func TestDetectPrettierConfigNone(t *testing.T) {
	if got := detectPrettierConfig(t.TempDir()); got != "" {
		t.Fatalf("detectPrettierConfig = %q, want empty", got)
	}
}

func TestDetectPrettierConfigPackageJSON(t *testing.T) {
	cases := []struct {
		name     string
		contents string
		want     bool
	}{
		{"key present", `{"prettier":{"semi":false}}`, true},
		{"key set to a string path", `{"prettier":"./cfg.json"}`, true},
		{"key null does not count", `{"prettier":null}`, false},
		{"key absent", `{"name":"pkg"}`, false},
		{"invalid json", `{`, false},
	}

	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			dir := t.TempDir()

			path := filepath.Join(dir, "package.json")

			if err := os.WriteFile(path, []byte(tc.contents), 0o644); err != nil {
				t.Fatalf("write package.json: %v", err)
			}

			got := detectPrettierConfig(dir)

			if tc.want && got != path {
				t.Fatalf("detectPrettierConfig = %q, want %q", got, path)
			}

			if !tc.want && got != "" {
				t.Fatalf("detectPrettierConfig = %q, want empty", got)
			}
		})
	}
}

func TestDetectPrettierConfigPrefersStandaloneOverPackageJSON(t *testing.T) {
	dir := t.TempDir()

	standalone := filepath.Join(dir, ".prettierrc.json")

	if err := os.WriteFile(standalone, []byte("{}"), 0o644); err != nil {
		t.Fatalf("write .prettierrc.json: %v", err)
	}

	if err := os.WriteFile(filepath.Join(dir, "package.json"), []byte(`{"prettier":{}}`), 0o644); err != nil {
		t.Fatalf("write package.json: %v", err)
	}

	if got := detectPrettierConfig(dir); got != standalone {
		t.Fatalf("detectPrettierConfig = %q, want %q", got, standalone)
	}
}

func TestOxfmtConfigForDerivesFromPrettier(t *testing.T) {
	support := supportWithStub(t)

	if err := os.WriteFile(filepath.Join(support.Dir, ".oxfmtrc.json"), []byte("{}"), 0o644); err != nil {
		t.Fatalf("write bundled config: %v", err)
	}

	oxfmt := filepath.Join(t.TempDir(), "oxfmt")
	writeMigrateStub(t, oxfmt)

	cwd := t.TempDir()

	if err := os.WriteFile(filepath.Join(cwd, ".prettierrc.json"), []byte(`{"semi":false}`), 0o644); err != nil {
		t.Fatalf("write prettier config: %v", err)
	}

	t.Setenv(sidecarproto.OxfmtBinEnv, oxfmt)

	env := sidecarproto.ReadOverrides()

	var stderr bytes.Buffer

	config := Invoker{Assets: support, Env: env}.oxfmtConfigFor(context.Background(), cwd, &stderr)

	if config == "" || existingFile(config) == "" {
		t.Fatalf("expected a derived config path, got %q (stderr: %s)", config, stderr.String())
	}

	data, err := os.ReadFile(config)

	if err != nil {
		t.Fatalf("read derived config: %v", err)
	}

	if !strings.Contains(string(data), "derived") {
		t.Fatalf("derived config = %q, want migrated marker", data)
	}

	if !strings.HasPrefix(config, filepath.Join(support.Dir, "prettier-derived")) {
		t.Fatalf("derived config %q not under the support cache", config)
	}
}

func TestOxfmtConfigForCachesDerivedConfig(t *testing.T) {
	support := supportWithStub(t)

	oxfmt := filepath.Join(t.TempDir(), "oxfmt")
	writeMigrateStub(t, oxfmt)

	cwd := t.TempDir()

	if err := os.WriteFile(filepath.Join(cwd, ".prettierrc.json"), []byte(`{"semi":false}`), 0o644); err != nil {
		t.Fatalf("write prettier config: %v", err)
	}

	t.Setenv(sidecarproto.OxfmtBinEnv, oxfmt)

	env := sidecarproto.ReadOverrides()

	var stderr bytes.Buffer

	first := Invoker{Assets: support, Env: env}.oxfmtConfigFor(context.Background(), cwd, &stderr)
	second := Invoker{Assets: support, Env: env}.oxfmtConfigFor(context.Background(), cwd, &stderr)

	if first != second {
		t.Fatalf("cache miss on second call: %q vs %q", first, second)
	}

	if got := migrateInvocations(t, filepath.Dir(oxfmt)); got != 1 {
		t.Fatalf("migration ran %d times, want 1 (cache hit expected)", got)
	}
}

func TestOxfmtConfigForRemigratesWhenConfigChanges(t *testing.T) {
	support := supportWithStub(t)

	oxfmt := filepath.Join(t.TempDir(), "oxfmt")
	writeMigrateStub(t, oxfmt)

	cwd := t.TempDir()

	prettier := filepath.Join(cwd, ".prettierrc.json")

	if err := os.WriteFile(prettier, []byte(`{"semi":false}`), 0o644); err != nil {
		t.Fatalf("write prettier config: %v", err)
	}

	t.Setenv(sidecarproto.OxfmtBinEnv, oxfmt)

	env := sidecarproto.ReadOverrides()

	var stderr bytes.Buffer

	Invoker{Assets: support, Env: env}.oxfmtConfigFor(context.Background(), cwd, &stderr)

	if err := os.WriteFile(prettier, []byte(`{"semi":true}`), 0o644); err != nil {
		t.Fatalf("rewrite prettier config: %v", err)
	}

	Invoker{Assets: support, Env: env}.oxfmtConfigFor(context.Background(), cwd, &stderr)

	if got := migrateInvocations(t, filepath.Dir(oxfmt)); got != 2 {
		t.Fatalf("migration ran %d times, want 2 (content hash should change)", got)
	}
}

func TestOxfmtConfigForPrecedence(t *testing.T) {
	support := supportWithStub(t)

	if err := os.WriteFile(filepath.Join(support.Dir, ".oxfmtrc.json"), []byte("{}"), 0o644); err != nil {
		t.Fatalf("write bundled config: %v", err)
	}

	oxfmt := filepath.Join(t.TempDir(), "oxfmt")
	writeMigrateStub(t, oxfmt)

	t.Setenv(sidecarproto.OxfmtBinEnv, oxfmt)

	t.Run("project .oxfmtrc beats prettier", func(t *testing.T) {
		cwd := t.TempDir()

		if err := os.WriteFile(filepath.Join(cwd, ".oxfmtrc.json"), []byte("{}"), 0o644); err != nil {
			t.Fatalf("write project config: %v", err)
		}

		if err := os.WriteFile(filepath.Join(cwd, ".prettierrc.json"), []byte("{}"), 0o644); err != nil {
			t.Fatalf("write prettier config: %v", err)
		}

		var stderr bytes.Buffer

		if got := (Invoker{Assets: support, Env: sidecarproto.ReadOverrides()}).oxfmtConfigFor(context.Background(), cwd, &stderr); got != "" {
			t.Fatalf("expected auto-discovery signal for project config, got %q", got)
		}
	})

	t.Run("prettier beats bundled", func(t *testing.T) {
		cwd := t.TempDir()

		if err := os.WriteFile(filepath.Join(cwd, ".prettierrc.json"), []byte("{}"), 0o644); err != nil {
			t.Fatalf("write prettier config: %v", err)
		}

		var stderr bytes.Buffer

		got := Invoker{Assets: support, Env: sidecarproto.ReadOverrides()}.oxfmtConfigFor(context.Background(), cwd, &stderr)

		if !strings.HasPrefix(got, filepath.Join(support.Dir, "prettier-derived")) {
			t.Fatalf("expected derived config, got %q", got)
		}
	})

	t.Run("bundled when neither present", func(t *testing.T) {
		cwd := t.TempDir()

		var stderr bytes.Buffer

		if got := (Invoker{Assets: support, Env: sidecarproto.ReadOverrides()}).oxfmtConfigFor(context.Background(), cwd, &stderr); got != support.OxfmtConfig() {
			t.Fatalf("expected bundled config %q, got %q", support.OxfmtConfig(), got)
		}
	})

	t.Run("env override beats all", func(t *testing.T) {
		cwd := t.TempDir()

		if err := os.WriteFile(filepath.Join(cwd, ".prettierrc.json"), []byte("{}"), 0o644); err != nil {
			t.Fatalf("write prettier config: %v", err)
		}

		override := filepath.Join(t.TempDir(), "custom.json")

		if err := os.WriteFile(override, []byte("{}"), 0o644); err != nil {
			t.Fatalf("write override config: %v", err)
		}

		env := sidecarproto.ReadOverrides()
		env.OxfmtConfig = override

		if got := (Invoker{Assets: support, Env: env}).oxfmtConfigFor(context.Background(), cwd, &bytes.Buffer{}); got != override {
			t.Fatalf("expected override %q, got %q", override, got)
		}
	})
}

func TestOxfmtConfigForFallsBackWhenMigrationFails(t *testing.T) {
	support := supportWithStub(t)

	if err := os.WriteFile(filepath.Join(support.Dir, ".oxfmtrc.json"), []byte("{}"), 0o644); err != nil {
		t.Fatalf("write bundled config: %v", err)
	}

	// A stub that exits nonzero and writes no config is the JS-config failure case.
	failing := filepath.Join(t.TempDir(), "oxfmt")

	if err := os.WriteFile(failing, []byte("#!/bin/sh\nexit 1\n"), 0o755); err != nil {
		t.Fatalf("write failing stub: %v", err)
	}

	cwd := t.TempDir()

	if err := os.WriteFile(filepath.Join(cwd, ".prettierrc.js"), []byte("module.exports = {};\n"), 0o644); err != nil {
		t.Fatalf("write prettier config: %v", err)
	}

	t.Setenv(sidecarproto.OxfmtBinEnv, failing)

	var stderr bytes.Buffer

	got := Invoker{Assets: support, Env: sidecarproto.ReadOverrides()}.oxfmtConfigFor(context.Background(), cwd, &stderr)

	if got != support.OxfmtConfig() {
		t.Fatalf("expected bundled fallback %q, got %q", support.OxfmtConfig(), got)
	}

	if !strings.Contains(stderr.String(), "using bundled config") {
		t.Fatalf("expected a warning on stderr, got %q", stderr.String())
	}
}
