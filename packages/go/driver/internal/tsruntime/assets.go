// Package tsruntime manages the self-contained TS toolchain shipped inside
// release binaries: a bun-compiled sidecar plus the oxc-parser, oxfmt, and
// oxlint napi bindings. On first use the embedded assets are extracted to a
// per-version cache directory and spawned as child processes from there.
//
// The type split mirrors the three responsibilities: Assets owns the extracted
// directory (extraction, caching, lookup); Invoker spawns the toolchain; and
// PrettierMigration derives an oxfmt config from a project's Prettier setup. All
// argv and environment construction goes through the sidecarproto package.
package tsruntime

import (
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"fmt"
	"io"
	"io/fs"
	"os"
	"path/filepath"
	"sort"

	"go.ollin.sh/fmtkit/driver/internal/embedded"
	"go.ollin.sh/fmtkit/driver/internal/sidecarproto"
)

// Assets locates the extracted TS toolchain on disk.
type Assets struct {
	Dir string
}

// sentinelName marks a completed extraction; it is tsruntime's own bookkeeping,
// not part of the sidecar wire protocol.
const sentinelName = ".fmtkit-complete"

// Sidecar returns the path of the multiplexed toolchain executable.
func (a Assets) Sidecar() string {
	return filepath.Join(a.Dir, sidecarproto.SidecarName)
}

// OxfmtConfig returns the bundled oxfmt configuration path, or "" when the
// support directory carries none.
func (a Assets) OxfmtConfig() string {
	return existingFile(filepath.Join(a.Dir, sidecarproto.OxfmtRCName))
}

// OxlintConfig returns the bundled oxlint configuration path, or "" when the
// support directory carries none.
func (a Assets) OxlintConfig() string {
	return existingFile(filepath.Join(a.Dir, sidecarproto.OxlintRCName))
}

func existingFile(path string) string {
	if info, err := os.Stat(path); err == nil && info.Mode().IsRegular() {
		return path
	}

	return ""
}

// Resolve locates the TS toolchain, extracting the embedded assets into the
// user cache on first use. version tells extractions of different releases
// apart; dev builds derive a digest from the assets instead.
func Resolve(version string) (Assets, error) {
	if dir := os.Getenv(sidecarproto.SupportDirEnv); dir != "" {
		assets := Assets{Dir: dir}

		if existingFile(assets.Sidecar()) == "" {
			return Assets{}, fmt.Errorf("%s (%s) does not contain %s", sidecarproto.SupportDirEnv, dir, sidecarproto.SidecarName)
		}

		return assets, nil
	}

	embeddedAssets, ok := embedded.SidecarAssets()

	if !ok {
		return Assets{}, errors.New(
			"this fmtkit build carries no TS toolchain (built without the fmtkit_sidecar tag); " +
				"point " + sidecarproto.SupportDirEnv + " at a staged toolchain directory " +
				"(see packages/ts/infra/stage-ts-assets.sh), or use a release binary",
		)
	}

	if version == "" || version == "dev" {
		digest, err := assetsDigest(embeddedAssets)

		if err != nil {
			return Assets{}, err
		}

		version = "dev-" + digest
	}

	cacheRoot, err := os.UserCacheDir()

	if err != nil {
		return Assets{}, fmt.Errorf("resolve user cache dir: %w", err)
	}

	dir := filepath.Join(cacheRoot, "fmtkit", version)

	if err := extractOnce(dir, embeddedAssets); err != nil {
		return Assets{}, err
	}

	return Assets{Dir: dir}, nil
}

// extractOnce materializes the toolchain into dir unless a completed
// extraction is already there. Concurrent first runs race benignly: each
// extracts into its own temp sibling and the first rename wins.
func extractOnce(dir string, assets fs.FS) error {
	if existingFile(filepath.Join(dir, sentinelName)) != "" {
		return nil
	}

	parent := filepath.Dir(dir)

	if err := os.MkdirAll(parent, 0o755); err != nil {
		return fmt.Errorf("create cache dir: %w", err)
	}

	tmp, err := os.MkdirTemp(parent, filepath.Base(dir)+".tmp-")

	if err != nil {
		return fmt.Errorf("create extraction dir: %w", err)
	}

	defer func() {
		_ = os.RemoveAll(tmp)
	}()

	if err := extract(tmp, assets); err != nil {
		return err
	}

	if err := os.Rename(tmp, dir); err != nil {
		if existingFile(filepath.Join(dir, sentinelName)) != "" {
			return nil
		}

		return fmt.Errorf("activate toolchain dir: %w", err)
	}

	return nil
}

func extract(dst string, assets fs.FS) error {
	entries, err := fs.ReadDir(assets, ".")

	if err != nil {
		return fmt.Errorf("read embedded toolchain: %w", err)
	}

	for _, entry := range entries {
		if entry.IsDir() {
			continue
		}

		mode := os.FileMode(0o644)

		if entry.Name() == sidecarproto.SidecarName {
			mode = 0o755
		}

		if err := writeFileFrom(assets, entry.Name(), filepath.Join(dst, entry.Name()), mode); err != nil {
			return err
		}
	}

	if err := os.WriteFile(filepath.Join(dst, sentinelName), nil, 0o644); err != nil {
		return fmt.Errorf("write %s: %w", sentinelName, err)
	}

	return nil
}

func writeFileFrom(assets fs.FS, name, dst string, mode os.FileMode) error {
	src, err := assets.Open(name)

	if err != nil {
		return fmt.Errorf("open embedded %s: %w", name, err)
	}

	defer func() {
		_ = src.Close()
	}()

	out, err := os.OpenFile(dst, os.O_WRONLY|os.O_CREATE|os.O_TRUNC, mode)

	if err != nil {
		return fmt.Errorf("write %s: %w", dst, err)
	}

	if _, err := io.Copy(out, src); err != nil {
		_ = out.Close()

		return fmt.Errorf("write %s: %w", dst, err)
	}

	return out.Close()
}

func assetsDigest(assets fs.FS) (string, error) {
	hash := sha256.New()

	var names []string

	entries, err := fs.ReadDir(assets, ".")

	if err != nil {
		return "", fmt.Errorf("read embedded toolchain: %w", err)
	}

	for _, entry := range entries {
		if !entry.IsDir() {
			names = append(names, entry.Name())
		}
	}

	sort.Strings(names)

	for _, name := range names {
		hash.Write([]byte(name))

		if err := hashFile(hash, assets, name); err != nil {
			return "", err
		}
	}

	return hex.EncodeToString(hash.Sum(nil))[:12], nil
}

// hashFile streams one asset through hash. The sidecar alone is tens of
// megabytes, so reading assets whole would spike memory for a digest that is
// only used to name a cache directory.
func hashFile(hash io.Writer, assets fs.FS, name string) error {
	file, err := assets.Open(name)

	if err != nil {
		return fmt.Errorf("open embedded %s: %w", name, err)
	}

	defer func() {
		_ = file.Close()
	}()

	if _, err := io.Copy(hash, file); err != nil {
		return fmt.Errorf("read embedded %s: %w", name, err)
	}

	return nil
}
