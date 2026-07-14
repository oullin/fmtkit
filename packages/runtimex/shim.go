package runtimex

import (
	cryptorand "crypto/rand"
	"encoding/hex"
	"fmt"
	"os"
	"path/filepath"
	"strings"
)

type sourceShimPublisher func(temporary, path string) error

func writeSourceShim(path, self string) error {
	return writeSourceShimWithPublisher(path, self, publishSourceShimAtomically)
}

func publishSourceShimAtomically(temporary, path string) error {
	return os.Rename(temporary, path)
}

func writeSourceShimWithPublisher(path, self string, publish sourceShimPublisher) error {
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

	if err := publish(temporary, path); err != nil {
		return fmt.Errorf("publish fmt-sources shim: %w", err)
	}

	published = true

	if err := syncSourceShimDirectory(filepath.Dir(path)); err != nil {
		return err
	}

	return nil
}

func syncSourceShimDirectory(path string) error {
	directory, err := os.Open(path)

	if err != nil {
		return fmt.Errorf("open fmt-sources shim directory for sync: %w", err)
	}

	if err := directory.Sync(); err != nil {
		_ = directory.Close()

		return fmt.Errorf("sync fmt-sources shim directory: %w", err)
	}

	if err := directory.Close(); err != nil {
		return fmt.Errorf("close fmt-sources shim directory: %w", err)
	}

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
