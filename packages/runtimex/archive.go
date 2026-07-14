package runtimex

import (
	"archive/tar"
	"bytes"
	"compress/gzip"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strings"
)

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
