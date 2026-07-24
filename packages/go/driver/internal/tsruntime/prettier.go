package tsruntime

import (
	"context"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
)

// prettierConfigNames are the standalone Prettier configuration filenames, in
// the order Prettier itself resolves them. package.json's "prettier" key is
// checked separately, after these.
var prettierConfigNames = []string{
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

// detectPrettierConfig returns the path of the Prettier configuration cwd
// carries, or "" when it has none. A standalone config file wins over
// package.json's "prettier" key, matching Prettier's own precedence.
func detectPrettierConfig(cwd string) string {
	for _, name := range prettierConfigNames {
		if path := existingFile(filepath.Join(cwd, name)); path != "" {
			return path
		}
	}

	packageJSON := filepath.Join(cwd, "package.json")

	if existingFile(packageJSON) != "" && packageJSONHasPrettierKey(packageJSON) {
		return packageJSON
	}

	return ""
}

// packageJSONHasPrettierKey reports whether package.json declares a top-level
// "prettier" key set to anything other than null. Unreadable or invalid JSON
// counts as absent.
func packageJSONHasPrettierKey(path string) bool {
	data, err := os.ReadFile(path)

	if err != nil {
		return false
	}

	var fields map[string]json.RawMessage

	if err := json.Unmarshal(data, &fields); err != nil {
		return false
	}

	value, ok := fields["prettier"]

	return ok && string(value) != "null"
}

// prettierDerivedConfig returns the path of an oxfmt config derived from cwd's
// Prettier configuration, or "" when there is no Prettier config or the
// migration fails. Failures print a one-line warning to stderr and leave the
// caller to fall back to the bundled config; a translated config is cached by
// the source config's content hash so migration runs at most once per config.
func (s Support) prettierDerivedConfig(ctx context.Context, cwd string, env overrides, stderr io.Writer) string {
	source := detectPrettierConfig(cwd)

	if source == "" {
		return ""
	}

	data, err := os.ReadFile(source)

	if err != nil {
		_, _ = fmt.Fprintf(stderr, "[oxfmt] could not read Prettier config %s: %v; using bundled config\n", source, err)

		return ""
	}

	sum := sha256.Sum256(data)
	cachePath := filepath.Join(s.Dir, "prettier-derived", hex.EncodeToString(sum[:])+".json")

	if existingFile(cachePath) != "" {
		return cachePath
	}

	derived, err := s.migratePrettierConfig(ctx, source, env)

	if err != nil {
		_, _ = fmt.Fprintf(stderr, "[oxfmt] could not derive oxfmt config from %s: %v; using bundled config\n", source, err)

		return ""
	}

	if err := writeCacheFile(cachePath, derived); err != nil {
		_, _ = fmt.Fprintf(stderr, "[oxfmt] could not cache derived oxfmt config: %v; using bundled config\n", err)

		return ""
	}

	return cachePath
}

// migratePrettierConfig copies the Prettier config into a private temp dir,
// runs oxfmt --migrate=prettier there, and returns the resulting .oxfmtrc.json
// bytes. The temp dir starts empty so the migrator never trips over a
// pre-existing oxfmt config.
func (s Support) migratePrettierConfig(ctx context.Context, source string, env overrides) ([]byte, error) {
	dir, err := os.MkdirTemp("", "fmtkit-prettier-migrate-")

	if err != nil {
		return nil, fmt.Errorf("create migration dir: %w", err)
	}

	defer func() {
		_ = os.RemoveAll(dir)
	}()

	if err := copyFileContents(source, filepath.Join(dir, filepath.Base(source))); err != nil {
		return nil, err
	}

	bin, args := s.migrateCommand(env)
	args = append(args, "--migrate=prettier")

	cmd := exec.CommandContext(ctx, bin, args...)
	cmd.Dir = dir
	cmd.Stdout = io.Discard
	cmd.Stderr = io.Discard

	if err := cmd.Run(); err != nil {
		return nil, fmt.Errorf("oxfmt --migrate=prettier: %w", err)
	}

	derived, err := os.ReadFile(filepath.Join(dir, ".oxfmtrc.json"))

	if err != nil {
		return nil, fmt.Errorf("read migrated config: %w", err)
	}

	return derived, nil
}

// migrateCommand resolves the oxfmt invocation for a migration, mirroring
// RunPipeline: an OXFMT_BIN override runs directly, otherwise the sidecar runs
// in its oxfmt pass-through mode.
func (s Support) migrateCommand(env overrides) (string, []string) {
	if env.oxfmtBin != "" {
		return env.oxfmtBin, nil
	}

	return s.Sidecar(), []string{"oxfmt"}
}

func copyFileContents(source, dst string) error {
	data, err := os.ReadFile(source)

	if err != nil {
		return fmt.Errorf("read %s: %w", source, err)
	}

	if err := os.WriteFile(dst, data, 0o644); err != nil {
		return fmt.Errorf("write %s: %w", dst, err)
	}

	return nil
}

// writeCacheFile writes data to path, creating parents and renaming a sibling
// temp file into place so a hit never sees a half-written config.
func writeCacheFile(path string, data []byte) error {
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		return fmt.Errorf("create cache dir: %w", err)
	}

	tmp, err := os.CreateTemp(filepath.Dir(path), filepath.Base(path)+".tmp-")

	if err != nil {
		return fmt.Errorf("create cache temp: %w", err)
	}

	if _, err := tmp.Write(data); err != nil {
		_ = tmp.Close()
		_ = os.Remove(tmp.Name())

		return fmt.Errorf("write cache temp: %w", err)
	}

	if err := tmp.Close(); err != nil {
		_ = os.Remove(tmp.Name())

		return fmt.Errorf("close cache temp: %w", err)
	}

	if err := os.Rename(tmp.Name(), path); err != nil {
		_ = os.Remove(tmp.Name())

		return fmt.Errorf("activate cache file: %w", err)
	}

	return nil
}
