//go:build darwin || dragonfly || freebsd || linux || netbsd || openbsd

package runtimex

import (
	"fmt"
	"os"
	"syscall"
)

// lockRuntime uses an advisory lock on a private regular file. The lock file
// may remain after a crash, but the kernel releases the advisory lock when the
// owning process dies, including after SIGKILL.
func lockRuntime(root string) (func(), error) {
	path := root + ".lock"
	fd, err := syscall.Open(path, syscall.O_RDWR|syscall.O_CREAT|syscall.O_NOFOLLOW, 0o600)

	if err != nil {
		return nil, fmt.Errorf("open runtime lock: %w", err)
	}

	var stat syscall.Stat_t

	if err := syscall.Fstat(fd, &stat); err != nil {
		_ = syscall.Close(fd)

		return nil, fmt.Errorf("inspect runtime lock: %w", err)
	}

	mode := os.FileMode(stat.Mode)

	if !mode.IsRegular() || mode.Perm()&0o077 != 0 || int(stat.Uid) != os.Getuid() {
		_ = syscall.Close(fd)

		return nil, fmt.Errorf("runtime lock is unsafe")
	}

	if err := syscall.Flock(fd, syscall.LOCK_EX); err != nil {
		_ = syscall.Close(fd)

		return nil, fmt.Errorf("lock runtime cache: %w", err)
	}

	return func() {
		_ = syscall.Flock(fd, syscall.LOCK_UN)
		_ = syscall.Close(fd)
	}, nil
}
