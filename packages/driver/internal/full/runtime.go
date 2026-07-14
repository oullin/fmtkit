package full

import (
	"archive/tar"
	"bytes"
	"compress/gzip"
	"crypto/sha256"
	"embed"
	"encoding/hex"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"runtime"
	"strings"
)

const runtimeVersion = "v1"

//go:embed assets/*
var runtimeAssets embed.FS

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

	root := strings.TrimSpace(os.Getenv("GO_FMT_RUNTIME_DIR"))

	if root == "" {
		cacheRoot, err := os.UserCacheDir()

		if err != nil {
			return toolRuntime{}, fmt.Errorf("resolve user cache dir: %w", err)
		}

		root = filepath.Join(cacheRoot, "go-fmt", "runtime", runtimeVersion, runtime.GOOS+"-"+runtime.GOARCH)
	}

	binDir := filepath.Join(root, "bin")

	if err := os.MkdirAll(binDir, 0o755); err != nil {
		return toolRuntime{}, fmt.Errorf("create runtime bin dir: %w", err)
	}

	if err := extractBundledRuntime(root); err != nil {
		return toolRuntime{}, err
	}

	if err := writeSourceShim(filepath.Join(binDir, "fmt-sources"), self); err != nil {
		return toolRuntime{}, err
	}

	return toolRuntime{
		binDir: binDir,
		root:   root,
		self:   self,
	}, nil
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

func extractBundledRuntime(root string) error {
	content, ok, err := bundledRuntimeArchive()

	if err != nil {
		return err
	}

	if !ok {
		return nil
	}

	sum := sha256.Sum256(content)
	fingerprint := hex.EncodeToString(sum[:])
	marker := filepath.Join(root, ".runtime.sha256")

	if current, err := os.ReadFile(marker); err == nil && strings.TrimSpace(string(current)) == fingerprint {
		return nil
	}

	if err := untarGzip(root, content); err != nil {
		return err
	}

	if err := os.WriteFile(marker, []byte(fingerprint+"\n"), 0o644); err != nil {
		return fmt.Errorf("write runtime marker: %w", err)
	}

	return nil
}

func bundledRuntimeArchive() ([]byte, bool, error) {
	name := "assets/runtime-" + runtime.GOOS + "-" + runtime.GOARCH + ".tar.gz"
	content, err := runtimeAssets.ReadFile(name)

	if err == nil {
		return content, true, nil
	}

	if os.IsNotExist(err) {
		return nil, false, nil
	}

	return nil, false, fmt.Errorf("read bundled runtime %s: %w", name, err)
}

func untarGzip(root string, content []byte) error {
	gzipReader, err := gzip.NewReader(bytes.NewReader(content))

	if err != nil {
		return fmt.Errorf("open bundled runtime: %w", err)
	}

	defer gzipReader.Close()

	tarReader := tar.NewReader(gzipReader)

	for {
		header, err := tarReader.Next()

		if err == io.EOF {
			return nil
		}

		if err != nil {
			return fmt.Errorf("read bundled runtime: %w", err)
		}

		if filepath.Clean(header.Name) == "." {
			continue
		}

		if isAppleDoublePath(header.Name) {
			continue
		}

		target, err := runtimeTarget(root, header.Name)

		if err != nil {
			return err
		}

		switch header.Typeflag {
		case tar.TypeDir:
			if err := os.MkdirAll(target, header.FileInfo().Mode()); err != nil {
				return fmt.Errorf("create runtime dir %s: %w", target, err)
			}
		case tar.TypeReg:
			if err := os.MkdirAll(filepath.Dir(target), 0o755); err != nil {
				return fmt.Errorf("create runtime parent %s: %w", filepath.Dir(target), err)
			}

			file, err := os.OpenFile(target, os.O_CREATE|os.O_WRONLY|os.O_TRUNC, header.FileInfo().Mode())

			if err != nil {
				return fmt.Errorf("create runtime file %s: %w", target, err)
			}

			if _, err := io.Copy(file, tarReader); err != nil {
				_ = file.Close()

				return fmt.Errorf("write runtime file %s: %w", target, err)
			}

			if err := file.Close(); err != nil {
				return fmt.Errorf("close runtime file %s: %w", target, err)
			}
		case tar.TypeSymlink:
			if _, err := runtimeTarget(root, filepath.Join(filepath.Dir(header.Name), header.Linkname)); err != nil {
				return fmt.Errorf("invalid runtime symlink %s -> %s: %w", header.Name, header.Linkname, err)
			}

			if err := os.RemoveAll(target); err != nil {
				return fmt.Errorf("replace runtime symlink %s: %w", target, err)
			}

			if err := os.Symlink(header.Linkname, target); err != nil {
				return fmt.Errorf("create runtime symlink %s: %w", target, err)
			}
		case tar.TypeLink:
			source, err := runtimeTarget(root, header.Linkname)

			if err != nil {
				return fmt.Errorf("invalid runtime hard link %s -> %s: %w", header.Name, header.Linkname, err)
			}

			if err := os.RemoveAll(target); err != nil {
				return fmt.Errorf("replace runtime hard link %s: %w", target, err)
			}

			if err := os.Link(source, target); err != nil {
				return fmt.Errorf("create runtime hard link %s: %w", target, err)
			}
		default:
			return fmt.Errorf("unsupported runtime archive entry %s", header.Name)
		}
	}
}

func runtimeTarget(root string, name string) (string, error) {
	clean := filepath.Clean(name)

	if clean == "." || filepath.IsAbs(clean) || strings.HasPrefix(clean, ".."+string(filepath.Separator)) || clean == ".." {
		return "", fmt.Errorf("invalid runtime archive path %q", name)
	}

	target := filepath.Join(root, clean)
	rel, err := filepath.Rel(root, target)

	if err != nil {
		return "", fmt.Errorf("validate runtime archive path %q: %w", name, err)
	}

	if rel == ".." || strings.HasPrefix(rel, ".."+string(filepath.Separator)) {
		return "", fmt.Errorf("runtime archive path escapes runtime dir: %q", name)
	}

	return target, nil
}

func isAppleDoublePath(name string) bool {
	for _, part := range strings.Split(filepath.ToSlash(name), "/") {
		if strings.HasPrefix(part, "._") {
			return true
		}
	}

	return false
}

func writeSourceShim(path string, self string) error {
	content := "#!/usr/bin/env sh\nexec " + shellQuote(self) + " go sources \"$@\"\n"

	current, err := os.ReadFile(path)

	if err == nil && string(current) == content {
		return os.Chmod(path, 0o755)
	}

	if err := os.WriteFile(path, []byte(content), 0o755); err != nil {
		return fmt.Errorf("write fmt-sources shim: %w", err)
	}

	if err := os.Chmod(path, 0o755); err != nil {
		return fmt.Errorf("make fmt-sources shim executable: %w", err)
	}

	return nil
}

func shellQuote(value string) string {
	if value == "" {
		return "''"
	}

	return "'" + strings.ReplaceAll(value, "'", "'\\''") + "'"
}
