package runtimex

import (
	"fmt"
	"os"
	"path/filepath"
)

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
