package runtimex

import (
	"fmt"
	"os"
	"path/filepath"

	"github.com/oullin/fmtkit/packages/runtimex/integrityx"
)

var runtimeShimExclusions = map[string]struct{}{"bin/fmt-sources": {}}

func validatePrivateRuntime(root string, manifest integrityx.Manifest) error {
	if err := validatePrivateRuntimePath(root); err != nil {
		return err
	}

	return integrityx.ValidateTree(root, manifest, runtimeShimExclusions)
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
