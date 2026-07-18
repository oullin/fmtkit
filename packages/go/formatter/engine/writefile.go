package engine

import (
	"os"
	"path/filepath"
)

// writeFileAtomic writes via a sibling temp file plus rename so a crash
// mid-write can never leave a truncated source file behind: the original
// stays intact until the rename atomically replaces it. The temp file is
// created with os.CreateTemp, which O_EXCL-creates it under an
// unpredictable, randomized name in the target directory (CWE-377) so no
// other process can pre-create or guess the path. It is chmod'd to the
// original's permissions (stat first, falling back to 0o644 for new files)
// and removed on any failure path, so the error surfaces without leaving
// debris next to the source.
func writeFileAtomic(path string, data []byte) error {
	mode := os.FileMode(0o644)

	if info, err := os.Stat(path); err == nil {
		mode = info.Mode().Perm()
	}

	tmp, err := os.CreateTemp(filepath.Dir(path), filepath.Base(path)+".*.tmp")

	if err != nil {
		return err
	}

	tmpName := tmp.Name()

	if err := tmp.Chmod(mode); err != nil {
		_ = tmp.Close()
		_ = os.Remove(tmpName)

		return err
	}

	if _, err := tmp.Write(data); err != nil {
		_ = tmp.Close()
		_ = os.Remove(tmpName)

		return err
	}

	if err := tmp.Close(); err != nil {
		_ = os.Remove(tmpName)

		return err
	}

	if err := os.Rename(tmpName, path); err != nil {
		_ = os.Remove(tmpName)

		return err
	}

	return nil
}
