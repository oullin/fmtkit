package full

import (
	"archive/tar"
	"bytes"
	"compress/gzip"
	cryptorand "crypto/rand"
	"embed"
	"encoding/hex"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"syscall"

	"github.com/oullin/fmtkit/packages/runtimeintegrity"
)

const runtimeVersion = "v2"

var runtimeShimExclusions = map[string]struct{}{"bin/fmt-sources": {}}

//go:embed assets/*
var runtimeAssets embed.FS

type runtimePayload struct {
	archive  []byte
	manifest runtimeintegrity.Manifest
}

type toolRuntime struct {
	binDir string
	root   string
	self   string
}

func ensureToolRuntime() (toolRuntime, error) {
	self, err := os.Executable()

	if err != nil {
		return toolRuntime{}, fmt.Errorf("resolve executable: %w", err)
	}

	payload, ok, err := bundledRuntimePayload()

	if err != nil {
		return toolRuntime{}, err
	}

	root, err := runtimeRoot(payload, ok)

	if err != nil {
		return toolRuntime{}, err
	}

	if err := ensureRuntimeCache(root, payload, ok); err != nil {
		return toolRuntime{}, err
	}

	binDir := filepath.Join(root, "bin")

	if err := ensureSourceShim(root, self); err != nil {
		return toolRuntime{}, err
	}

	return toolRuntime{binDir: binDir, root: root, self: self}, nil
}

func runtimeRoot(payload runtimePayload, hasPayload bool) (string, error) {
	base := strings.TrimSpace(os.Getenv("GO_FMT_RUNTIME_DIR"))

	if base != "" {
		if !filepath.IsAbs(base) {
			return "", fmt.Errorf("GO_FMT_RUNTIME_DIR must be an absolute path")
		}
	} else {
		cacheRoot, err := os.UserCacheDir()

		if err != nil {
			return "", fmt.Errorf("resolve user cache dir: %w", err)
		}

		base = filepath.Join(cacheRoot, "go-fmt", "contained")
	}

	base = filepath.Clean(base)

	if err := ensureTrustedBase(base); err != nil {
		return "", err
	}

	identity := "unbundled"

	if hasPayload {
		identity = payload.manifest.ArchiveSHA256
	}

	return filepath.Join(base, "runtime", runtimeVersion, runtime.GOOS+"-"+runtime.GOARCH, identity), nil
}

func (r toolRuntime) formatTSBin() string {
	if value := strings.TrimSpace(os.Getenv("FORMAT_TS_BIN")); value != "" {
		return value
	}

	return filepath.Join(r.binDir, "fmt-ts")
}

func (r toolRuntime) lintTSBin() string {
	if value := strings.TrimSpace(os.Getenv("FORMAT_LINT_BIN")); value != "" {
		return value
	}

	return filepath.Join(r.binDir, "fmt-lint")
}

func (r toolRuntime) env() []string {
	env := os.Environ()
	env = append(env,
		"FMTKIT_SOURCES_BIN="+filepath.Join(r.binDir, "fmt-sources"),
		"GO_FMT_SOURCES_BIN="+filepath.Join(r.binDir, "fmt-sources"),
	)

	return env
}

func (r toolRuntime) applyGoEnv() func() {
	updates := map[string]string{}
	goRoot := filepath.Join(r.root, "go")
	goBin := filepath.Join(goRoot, "bin", "go")

	if _, err := os.Stat(goBin); err == nil {
		updates = map[string]string{
			"PATH":       r.binDir + string(os.PathListSeparator) + os.Getenv("PATH"),
			"GOROOT":     goRoot,
			"GOCACHE":    filepath.Join(r.root, "cache", "go-build"),
			"GOPATH":     filepath.Join(r.root, "cache", "gopath"),
			"GOMODCACHE": filepath.Join(r.root, "cache", "gopath", "pkg", "mod"),
		}
	}

	previous := map[string]*string{}

	for key, value := range updates {
		if current, ok := os.LookupEnv(key); ok {
			copy := current
			previous[key] = &copy
		} else {
			previous[key] = nil
		}

		_ = os.Setenv(key, value)
	}

	return func() {
		for key, value := range previous {
			if value == nil {
				_ = os.Unsetenv(key)

				continue
			}

			_ = os.Setenv(key, *value)
		}
	}
}

func bundledRuntimePayload() (runtimePayload, bool, error) {
	archiveName := "assets/runtime-" + runtime.GOOS + "-" + runtime.GOARCH + ".tar.gz"
	archive, err := runtimeAssets.ReadFile(archiveName)

	if os.IsNotExist(err) {
		return runtimePayload{}, false, nil
	}

	if err != nil {
		return runtimePayload{}, false, fmt.Errorf("read bundled runtime %s: %w", archiveName, err)
	}

	manifestName := archiveName + ".manifest.json"
	manifestContent, err := runtimeAssets.ReadFile(manifestName)

	if err != nil {
		return runtimePayload{}, false, fmt.Errorf("read runtime manifest %s: %w", manifestName, err)
	}

	manifest, err := runtimeintegrity.Parse(manifestContent)

	if err != nil {
		return runtimePayload{}, false, err
	}

	if err := runtimeintegrity.ValidateArchive(archive, manifest); err != nil {
		return runtimePayload{}, false, err
	}

	return runtimePayload{archive: archive, manifest: manifest}, true, nil
}

func ensureRuntimeCache(root string, payload runtimePayload, hasPayload bool) error {
	if err := ensureSecureParent(filepath.Dir(root)); err != nil {
		return err
	}

	unlock, err := lockRuntime(root)

	if err != nil {
		return err
	}

	defer unlock()

	ready, err := reconcileRuntime(root, payload, hasPayload)

	if err != nil {
		return err
	}

	if ready {
		if !hasPayload {
			return ensureEmptyRuntimeRoot(root)
		}

		return nil
	}

	if !hasPayload {
		return ensureEmptyRuntimeRoot(root)
	}

	stage, err := os.MkdirTemp(filepath.Dir(root), "."+filepath.Base(root)+".tmp-")

	if err != nil {
		return fmt.Errorf("create runtime staging directory: %w", err)
	}

	defer func() {
		_ = os.RemoveAll(stage)
	}()

	if err := os.Chmod(stage, 0o700); err != nil {
		return fmt.Errorf("secure runtime staging directory: %w", err)
	}

	if err := extractRuntimeArchive(stage, payload.archive); err != nil {
		return err
	}

	if err := validatePrivateRuntime(stage, payload.manifest); err != nil {
		return fmt.Errorf("validate extracted runtime: %w", err)
	}

	if err := publishRuntime(stage, root); err != nil {
		return err
	}

	return nil
}

func validRuntimeCache(root string, payload runtimePayload) bool {
	if err := validatePrivateRuntime(root, payload.manifest); err != nil {
		return false
	}

	return true
}

// reconcileRuntime completes an interrupted atomic publish before deciding
// whether extraction is necessary. A valid current cache always wins; a valid
// backup is restored only when the current cache is absent or invalid.
func reconcileRuntime(root string, payload runtimePayload, hasPayload bool) (bool, error) {
	currentExists, currentValid, err := runtimeState(root, payload, hasPayload)

	if err != nil {
		return false, fmt.Errorf("inspect runtime cache: %w", err)
	}

	backup := root + ".old"
	backupExists, backupValid, err := runtimeState(backup, payload, hasPayload)

	if err != nil {
		return false, fmt.Errorf("inspect runtime backup: %w", err)
	}

	if currentValid {
		if backupExists {
			if err := removePrivateRuntime(backup); err != nil {
				return false, fmt.Errorf("remove stale runtime backup: %w", err)
			}
		}

		return true, nil
	}

	if backupValid {
		if currentExists {
			if err := removePrivateRuntime(root); err != nil {
				return false, fmt.Errorf("remove invalid runtime cache: %w", err)
			}
		}

		if err := os.Rename(backup, root); err != nil {
			return false, fmt.Errorf("restore runtime backup: %w", err)
		}

		return true, nil
	}

	if backupExists {
		if err := removePrivateRuntime(backup); err != nil {
			return false, fmt.Errorf("remove invalid runtime backup: %w", err)
		}
	}

	if currentExists {
		if err := removePrivateRuntime(root); err != nil {
			return false, fmt.Errorf("remove invalid runtime cache: %w", err)
		}
	}

	return false, nil
}

func runtimeState(path string, payload runtimePayload, hasPayload bool) (bool, bool, error) {
	if _, err := os.Lstat(path); os.IsNotExist(err) {
		return false, false, nil
	} else if err != nil {
		return false, false, err
	}

	if err := validatePrivateRuntimePath(path); err != nil {
		return true, false, err
	}

	if !hasPayload {
		return true, true, nil
	}

	return true, validRuntimeCache(path, payload), nil
}

func ensureEmptyRuntimeRoot(root string) error {
	info, err := os.Lstat(root)

	if err == nil {
		if info.Mode()&os.ModeSymlink != 0 || !info.IsDir() {
			return fmt.Errorf("runtime root is not a directory")
		}

		if info.Mode().Perm()&0o077 != 0 {
			return fmt.Errorf("runtime root permissions are not owner-only")
		}

		return ensureSecureDirectory(filepath.Join(root, "bin"))
	}

	if !os.IsNotExist(err) {
		return fmt.Errorf("inspect runtime root: %w", err)
	}

	if err := os.Mkdir(root, 0o700); err != nil {
		return fmt.Errorf("create runtime root: %w", err)
	}

	return ensureSecureDirectory(filepath.Join(root, "bin"))
}

func validatePrivateRuntime(root string, manifest runtimeintegrity.Manifest) error {
	if err := validatePrivateRuntimePath(root); err != nil {
		return err
	}

	return runtimeintegrity.ValidateTree(root, manifest, runtimeShimExclusions)
}

func validatePrivateRuntimePath(root string) error {
	info, err := os.Lstat(root)

	if err != nil {
		return err
	}

	if info.Mode()&os.ModeSymlink != 0 || !info.IsDir() {
		return fmt.Errorf("runtime root is not a directory")
	}

	if info.Mode().Perm()&0o077 != 0 {
		return fmt.Errorf("runtime root permissions are not owner-only")
	}

	if !ownedByCurrentUser(info) {
		return fmt.Errorf("runtime root is not owned by the current user")
	}

	err = filepath.WalkDir(root, func(path string, entry os.DirEntry, walkErr error) error {
		if walkErr != nil {
			return walkErr
		}

		if entry.Type()&os.ModeSymlink != 0 {
			return fmt.Errorf("runtime cache contains symlink %s", path)
		}

		info, err := entry.Info()

		if err != nil {
			return err
		}

		if info.Mode()&os.ModeSymlink != 0 {
			return fmt.Errorf("runtime cache contains symlink %s", path)
		}

		if info.Mode().Perm()&0o077 != 0 {
			return fmt.Errorf("runtime cache is not owner-only: %s", path)
		}

		if !ownedByCurrentUser(info) {
			return fmt.Errorf("runtime cache is not owned by the current user: %s", path)
		}

		return nil
	})

	if err != nil {
		return err
	}

	return nil
}

func extractRuntimeArchive(root string, content []byte) error {
	rootFS, err := os.OpenRoot(root)

	if err != nil {
		return fmt.Errorf("open runtime staging root: %w", err)
	}

	defer func() {
		_ = rootFS.Close()
	}()

	gzipReader, err := gzip.NewReader(bytes.NewReader(content))

	if err != nil {
		return fmt.Errorf("open bundled runtime: %w", err)
	}

	defer func() {
		_ = gzipReader.Close()
	}()

	tarReader := tar.NewReader(gzipReader)

	for {
		header, err := tarReader.Next()

		if err == io.EOF {
			return nil
		}

		if err != nil {
			return fmt.Errorf("read bundled runtime: %w", err)
		}

		if isAppleDoublePath(header.Name) {
			continue
		}

		rel, err := runtimeRelativePath(header.Name)

		if err != nil {
			return err
		}

		switch header.Typeflag {
		case tar.TypeDir:
			if err := rootFS.MkdirAll(rel, 0o700); err != nil {
				return fmt.Errorf("create runtime directory %s: %w", rel, err)
			}
		case tar.TypeReg:
			if err := rootFS.MkdirAll(filepath.ToSlash(filepath.Dir(rel)), 0o700); err != nil {
				return fmt.Errorf("create runtime parent %s: %w", rel, err)
			}

			if err := writeRuntimeRootFile(rootFS, rel, tarReader, header.FileInfo().Mode()); err != nil {
				return err
			}
		default:
			return fmt.Errorf("unsupported runtime archive entry %s", header.Name)
		}
	}
}

func runtimeRelativePath(name string) (string, error) {
	clean := filepath.Clean(name)

	if clean == "." || filepath.IsAbs(clean) || strings.HasPrefix(clean, ".."+string(filepath.Separator)) || clean == ".." {
		return "", fmt.Errorf("invalid runtime archive path %q", name)
	}

	return filepath.ToSlash(clean), nil
}

func writeRuntimeRootFile(root *os.Root, path string, source io.Reader, mode os.FileMode) error {
	if info, err := root.Lstat(path); err == nil {
		if info.Mode()&os.ModeSymlink != 0 {
			return fmt.Errorf("runtime path is a symlink: %s", path)
		}

		return fmt.Errorf("runtime archive contains duplicate path %s", path)
	} else if !os.IsNotExist(err) {
		return err
	}

	permissions := os.FileMode(0o600)

	if mode.Perm()&0o111 != 0 {
		permissions = 0o700
	}

	file, err := root.OpenFile(path, os.O_CREATE|os.O_EXCL|os.O_WRONLY, permissions)

	if err != nil {
		return fmt.Errorf("create runtime file %s: %w", path, err)
	}

	if _, err := io.Copy(file, source); err != nil {
		_ = file.Close()

		return fmt.Errorf("write runtime file %s: %w", path, err)
	}

	if err := file.Close(); err != nil {
		return fmt.Errorf("close runtime file %s: %w", path, err)
	}

	return nil
}

func ensureSecureParent(path string) error {
	volume := filepath.VolumeName(path)
	remaining := strings.TrimPrefix(path, volume)
	current := volume + string(filepath.Separator)

	for _, part := range strings.Split(remaining, string(filepath.Separator)) {
		if part == "" {
			continue
		}

		current = filepath.Join(current, part)
		info, err := os.Lstat(current)

		if os.IsNotExist(err) {
			if err := os.Mkdir(current, 0o700); err != nil {
				return fmt.Errorf("create runtime parent %s: %w", current, err)
			}

			continue
		}

		if err != nil {
			return fmt.Errorf("inspect runtime parent %s: %w", current, err)
		}

		if info.Mode()&os.ModeSymlink != 0 || !info.IsDir() {
			return fmt.Errorf("runtime parent is not a directory: %s", current)
		}
	}

	return nil
}

func ensureTrustedBase(path string) error {
	if err := ensureSecureParent(filepath.Dir(path)); err != nil {
		return err
	}

	info, err := os.Lstat(path)

	if os.IsNotExist(err) {
		if err := os.Mkdir(path, 0o700); err != nil {
			return fmt.Errorf("create GO_FMT_RUNTIME_DIR base: %w", err)
		}

		return nil
	}

	if err != nil {
		return fmt.Errorf("inspect GO_FMT_RUNTIME_DIR base: %w", err)
	}

	if info.Mode()&os.ModeSymlink != 0 || !info.IsDir() {
		return fmt.Errorf("GO_FMT_RUNTIME_DIR base is not a directory")
	}

	if info.Mode().Perm()&0o077 != 0 {
		return fmt.Errorf("GO_FMT_RUNTIME_DIR base permissions are not owner-only")
	}

	if !ownedByCurrentUser(info) {
		return fmt.Errorf("GO_FMT_RUNTIME_DIR base is not owned by the current user")
	}

	return nil
}

func ownedByCurrentUser(info os.FileInfo) bool {
	stat, ok := info.Sys().(*syscall.Stat_t)

	return ok && int(stat.Uid) == os.Getuid()
}

func ensureSecureDirectory(path string) error {
	if err := ensureSecureParent(filepath.Dir(path)); err != nil {
		return err
	}

	info, err := os.Lstat(path)

	if os.IsNotExist(err) {
		return os.Mkdir(path, 0o700)
	}

	if err != nil {
		return err
	}

	if info.Mode()&os.ModeSymlink != 0 || !info.IsDir() {
		return fmt.Errorf("runtime path is not a directory: %s", path)
	}

	return os.Chmod(path, 0o700)
}

func publishRuntime(stage, root string) error {
	backup := root + ".old"

	if _, err := os.Lstat(backup); err == nil {
		return fmt.Errorf("runtime backup path already exists")
	} else if !os.IsNotExist(err) {
		return err
	}

	if _, err := os.Lstat(root); err == nil {
		if err := validatePrivateRuntimePath(root); err != nil {
			return fmt.Errorf("validate current runtime before publish: %w", err)
		}

		if err := os.Rename(root, backup); err != nil {
			return fmt.Errorf("move invalid runtime aside: %w", err)
		}
	}

	if err := os.Rename(stage, root); err != nil {
		return fmt.Errorf("publish runtime cache: %w", err)
	}

	if err := removePrivateRuntime(backup); err != nil && !os.IsNotExist(err) {
		return fmt.Errorf("remove stale runtime cache: %w", err)
	}

	return nil
}

func removePrivateRuntime(path string) error {
	if err := validatePrivateRuntimePath(path); err != nil {
		return err
	}

	return os.RemoveAll(path)
}

func isAppleDoublePath(name string) bool {
	for _, part := range strings.Split(filepath.ToSlash(name), "/") {
		if strings.HasPrefix(part, "._") {
			return true
		}
	}

	return false
}

func writeSourceShim(path, self string) error {
	if err := ensureSecureDirectory(filepath.Dir(path)); err != nil {
		return err
	}

	if info, err := os.Lstat(path); err == nil {
		if info.Mode()&os.ModeSymlink != 0 {
			return fmt.Errorf("fmt-sources shim is a symlink")
		}

		if !ownedByCurrentUser(info) {
			return fmt.Errorf("fmt-sources shim is not owned by the current user")
		}
	} else if !os.IsNotExist(err) {
		return fmt.Errorf("inspect fmt-sources shim: %w", err)
	}

	content := "#!/usr/bin/env sh\nexec " + shellQuote(self) + " go sources \"$@\"\n"
	temporary, file, err := createSourceShimTemp(filepath.Dir(path), filepath.Base(path))

	if err != nil {
		return err
	}

	published := false

	defer func() {
		if !published {
			_ = os.Remove(temporary)
		}
	}()

	if _, err := file.WriteString(content); err != nil {
		_ = file.Close()

		return fmt.Errorf("write temporary fmt-sources shim: %w", err)
	}

	if err := file.Sync(); err != nil {
		_ = file.Close()

		return fmt.Errorf("sync temporary fmt-sources shim: %w", err)
	}

	if err := file.Close(); err != nil {
		return fmt.Errorf("close temporary fmt-sources shim: %w", err)
	}

	if err := os.Rename(temporary, path); err != nil {
		return fmt.Errorf("publish fmt-sources shim: %w", err)
	}

	published = true

	return nil
}

func createSourceShimTemp(dir, base string) (string, *os.File, error) {
	for range 16 {
		var token [16]byte

		if _, err := cryptorand.Read(token[:]); err != nil {
			return "", nil, fmt.Errorf("generate temporary fmt-sources shim name: %w", err)
		}

		path := filepath.Join(dir, "."+base+".tmp-"+hex.EncodeToString(token[:]))
		file, err := os.OpenFile(path, os.O_CREATE|os.O_EXCL|os.O_WRONLY, 0o700)

		if os.IsExist(err) {
			continue
		}

		if err != nil {
			return "", nil, fmt.Errorf("create temporary fmt-sources shim: %w", err)
		}

		return path, file, nil
	}

	return "", nil, fmt.Errorf("create temporary fmt-sources shim: exhausted unique names")
}

// ensureSourceShim serializes replacement of the mutable shim with runtime
// extraction. Without the lock, concurrent launches can both remove the file
// then race to create it with O_EXCL.
func ensureSourceShim(root, self string) error {
	unlock, err := lockRuntime(root)

	if err != nil {
		return err
	}

	defer unlock()

	return writeSourceShim(filepath.Join(root, "bin", "fmt-sources"), self)
}

func shellQuote(value string) string {
	if value == "" {
		return "''"
	}

	return "'" + strings.ReplaceAll(value, "'", "'\\''") + "'"
}
