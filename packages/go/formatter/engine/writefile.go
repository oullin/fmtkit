package engine

import (
	"os"
	"strconv"
)

// writeFileAtomic writes via a sibling temp file plus rename so a crash
// mid-write can never leave a truncated source file behind: the original
// stays intact until the rename atomically replaces it. The temp file
// inherits the original's permissions (falling back to 0o644 for new files)
// and is removed when the write or rename fails, so the error surfaces
// without leaving debris next to the source. This mirrors
// packages/ts/sidecar/src/pass-utils.ts writeFileAtomic.
func writeFileAtomic(path string, data []byte) error {
	mode := os.FileMode(0o644)

	if info, err := os.Stat(path); err == nil {
		mode = info.Mode().Perm()
	}

	tmp := path + "." + strconv.Itoa(os.Getpid()) + ".tmp"

	if err := os.WriteFile(tmp, data, mode); err != nil {
		_ = os.Remove(tmp)

		return err
	}

	if err := os.Rename(tmp, path); err != nil {
		_ = os.Remove(tmp)

		return err
	}

	return nil
}
