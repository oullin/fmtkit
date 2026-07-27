package testutil

import (
	"os/exec"
	"testing"
)

// InitRepo creates a temporary git repository with the identity committing
// requires, and returns its directory.
func InitRepo(t *testing.T) string {
	t.Helper()

	dir := t.TempDir()
	Run(t, dir, "git", "init", "-q")
	Run(t, dir, "git", "config", "user.email", "tests@example.com")
	Run(t, dir, "git", "config", "user.name", "Test Runner")

	return dir
}

// GitAdd stages paths in the repository at dir.
func GitAdd(t *testing.T, dir string, paths ...string) {
	t.Helper()

	Run(t, dir, "git", append([]string{"add"}, paths...)...)
}

// GitCommit commits whatever is staged in the repository at dir.
func GitCommit(t *testing.T, dir string) {
	t.Helper()

	Run(t, dir, "git", "commit", "-q", "-m", "fixture")
}

// Run executes name with args in dir, failing the test with the combined
// output when it exits non-zero.
func Run(t *testing.T, dir string, name string, args ...string) {
	t.Helper()

	cmd := exec.Command(name, args...)
	cmd.Dir = dir

	if out, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("%s %v: %v\n%s", name, args, err, out)
	}
}
