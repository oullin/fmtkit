package full

import (
	"archive/tar"
	"bytes"
	"compress/gzip"
	"os"
	"path/filepath"
	"sync"
	"testing"
	"time"

	"github.com/oullin/fmtkit/packages/runtimeintegrity"
)

func TestExtractRuntimeArchiveRejectsUnsafeEntries(t *testing.T) {
	for _, entry := range []tar.Header{
		{Name: "../escape", Typeflag: tar.TypeReg, Mode: 0o600, Size: 1},
		{Name: "/absolute", Typeflag: tar.TypeReg, Mode: 0o600, Size: 1},
		{Name: "bin/link", Typeflag: tar.TypeSymlink, Linkname: "/tmp/outside"},
		{Name: "bin/fifo", Typeflag: tar.TypeFifo},
	} {
		t.Run(entry.Name, func(t *testing.T) {
			if err := extractRuntimeArchive(realTempDir(t), runtimeArchive(t, entry)); err == nil {
				t.Fatal("expected unsafe archive entry to be rejected")
			}
		})
	}
}

func TestEnsureRuntimeCacheRepairsPoisonedCacheAndStaysPrivate(t *testing.T) {
	root := filepath.Join(realTempDir(t), "runtime")
	payload := testRuntimePayload(t)

	if err := ensureRuntimeCache(root, payload, true); err != nil {
		t.Fatalf("extract runtime: %v", err)
	}

	if err := os.WriteFile(filepath.Join(root, "bin", "node"), []byte("poisoned"), 0o600); err != nil {
		t.Fatalf("poison runtime cache: %v", err)
	}

	if err := ensureRuntimeCache(root, payload, true); err != nil {
		t.Fatalf("repair runtime cache: %v", err)
	}

	if content := readFile(t, filepath.Join(root, "bin", "node")); content != "node" {
		t.Fatalf("expected repaired node, got %q", content)
	}

	info, err := os.Stat(root)

	if err != nil {
		t.Fatalf("stat runtime root: %v", err)
	}

	if info.Mode().Perm() != 0o700 {
		t.Fatalf("expected private runtime root, got %04o", info.Mode().Perm())
	}
}

func TestEnsureRuntimeCacheRejectsSymlinkedRoot(t *testing.T) {
	base := realTempDir(t)
	target := filepath.Join(base, "target")

	if err := os.Mkdir(target, 0o700); err != nil {
		t.Fatalf("create target: %v", err)
	}

	root := filepath.Join(base, "runtime")

	if err := os.Symlink(target, root); err != nil {
		t.Fatalf("create root symlink: %v", err)
	}

	if err := ensureRuntimeCache(root, testRuntimePayload(t), true); err == nil {
		t.Fatal("expected symlinked runtime root to be rejected")
	}
}

func TestEnsureRuntimeCacheRejectsSymlinkedBackupAndLock(t *testing.T) {
	for _, suffix := range []string{".old", ".lock"} {
		t.Run(suffix, func(t *testing.T) {
			base := realTempDir(t)
			root := filepath.Join(base, "runtime")
			target := filepath.Join(base, "target")

			if err := os.Mkdir(target, 0o700); err != nil {
				t.Fatalf("create target: %v", err)
			}

			if err := os.Symlink(target, root+suffix); err != nil {
				t.Fatalf("create unsafe state: %v", err)
			}

			if err := ensureRuntimeCache(root, testRuntimePayload(t), true); err == nil {
				t.Fatal("expected unsafe runtime state to be rejected")
			}
		})
	}
}

func TestEnsureRuntimeCacheSerializesConcurrentExtraction(t *testing.T) {
	root := filepath.Join(realTempDir(t), "runtime")
	payload := testRuntimePayload(t)

	var group sync.WaitGroup
	errs := make(chan error, 8)

	for range 8 {
		group.Add(1)

		go func() {
			defer group.Done()

			errs <- ensureRuntimeCache(root, payload, true)
		}()
	}

	group.Wait()
	close(errs)

	for err := range errs {
		if err != nil {
			t.Fatalf("concurrent extraction: %v", err)
		}
	}
}

func TestEnsureRuntimeCacheReleasesStaleAdvisoryLock(t *testing.T) {
	root := filepath.Join(realTempDir(t), "runtime")
	payload := testRuntimePayload(t)

	if err := os.WriteFile(root+".lock", nil, 0o600); err != nil {
		t.Fatalf("create stale lock file: %v", err)
	}

	if err := ensureRuntimeCache(root, payload, true); err != nil {
		t.Fatalf("recover with stale lock file: %v", err)
	}
}

func TestEnsureRuntimeCacheWaitsForSlowConcurrentExtraction(t *testing.T) {
	root := filepath.Join(realTempDir(t), "runtime")
	payload := testRuntimePayload(t)
	unlock, err := lockRuntime(root)

	if err != nil {
		t.Fatalf("acquire runtime lock: %v", err)
	}

	errs := make(chan error, 1)
	go func() {
		errs <- ensureRuntimeCache(root, payload, true)
	}()

	select {
	case err := <-errs:
		t.Fatalf("runtime extraction did not wait for lock: %v", err)
	case <-time.After(5100 * time.Millisecond):
	}

	unlock()

	if err := <-errs; err != nil {
		t.Fatalf("runtime extraction after slow lock: %v", err)
	}
}

func TestEnsureRuntimeCacheRecoversInterruptedPublish(t *testing.T) {
	t.Run("retains current and removes old", func(t *testing.T) {
		root := filepath.Join(realTempDir(t), "runtime")
		payload := testRuntimePayload(t)

		if err := ensureRuntimeCache(root, payload, true); err != nil {
			t.Fatalf("extract runtime: %v", err)
		}

		if err := os.Rename(root, root+".old"); err != nil {
			t.Fatalf("move runtime to old: %v", err)
		}

		if err := ensureRuntimeCache(root, payload, true); err != nil {
			t.Fatalf("replace runtime: %v", err)
		}

		if err := os.Mkdir(root+".old", 0o700); err != nil {
			t.Fatalf("create stale backup: %v", err)
		}

		if err := ensureRuntimeCache(root, payload, true); err != nil {
			t.Fatalf("recover stale backup: %v", err)
		}

		if _, err := os.Lstat(root + ".old"); !os.IsNotExist(err) {
			t.Fatalf("expected stale backup removal, got %v", err)
		}
	})

	t.Run("restores valid old when current is absent", func(t *testing.T) {
		root := filepath.Join(realTempDir(t), "runtime")
		payload := testRuntimePayload(t)

		if err := ensureRuntimeCache(root, payload, true); err != nil {
			t.Fatalf("extract runtime: %v", err)
		}

		if err := os.Rename(root, root+".old"); err != nil {
			t.Fatalf("simulate interrupted publish: %v", err)
		}

		if err := ensureRuntimeCache(root, payload, true); err != nil {
			t.Fatalf("restore runtime backup: %v", err)
		}

		if content := readFile(t, filepath.Join(root, "bin", "node")); content != "node" {
			t.Fatalf("expected restored runtime, got %q", content)
		}
	})
}

func testRuntimePayload(t *testing.T) runtimePayload {
	t.Helper()

	stage := realTempDir(t)
	writeRuntimeFixture(t, stage)
	archive := filepath.Join(realTempDir(t), "runtime.tar.gz")

	if err := os.WriteFile(archive, runtimeArchive(t,
		tar.Header{Name: "bin", Typeflag: tar.TypeDir, Mode: 0o700},
		tar.Header{Name: "bin/node", Typeflag: tar.TypeReg, Mode: 0o700, Size: 4},
	), 0o600); err != nil {
		t.Fatalf("write archive: %v", err)
	}

	manifest, err := runtimeintegrity.Build(stage, archive, []string{"bin/node"})

	if err != nil {
		t.Fatalf("build manifest: %v", err)
	}

	content, err := os.ReadFile(archive)

	if err != nil {
		t.Fatalf("read archive: %v", err)
	}

	return runtimePayload{archive: content, manifest: manifest}
}

func writeRuntimeFixture(t *testing.T, root string) {
	t.Helper()

	if err := os.Mkdir(filepath.Join(root, "bin"), 0o700); err != nil {
		t.Fatalf("create fixture bin: %v", err)
	}

	if err := os.WriteFile(filepath.Join(root, "bin", "node"), []byte("node"), 0o700); err != nil {
		t.Fatalf("write fixture node: %v", err)
	}
}

func runtimeArchive(t *testing.T, headers ...tar.Header) []byte {
	t.Helper()

	var buffer bytes.Buffer
	gzipWriter := gzip.NewWriter(&buffer)
	tarWriter := tar.NewWriter(gzipWriter)

	for _, header := range headers {
		if err := tarWriter.WriteHeader(&header); err != nil {
			t.Fatalf("write tar header: %v", err)
		}

		if header.Typeflag == tar.TypeReg && header.Size > 0 {
			if _, err := tarWriter.Write([]byte("node")[:header.Size]); err != nil {
				t.Fatalf("write tar content: %v", err)
			}
		}
	}

	if err := tarWriter.Close(); err != nil {
		t.Fatalf("close tar: %v", err)
	}

	if err := gzipWriter.Close(); err != nil {
		t.Fatalf("close gzip: %v", err)
	}

	return buffer.Bytes()
}
