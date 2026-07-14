package runtimex

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"syscall"
)

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
