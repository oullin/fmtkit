package runtime

import (
	"bytes"
	"context"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"go.ollin.sh/fmtkit/driver/internal/typescript/proto"
)

// writeScanStub creates a sidecar stub that records its argv and prints a
// canned measurement, standing in for the real complexity mode.
func writeScanStub(t *testing.T, payload string) (Assets, string) {
	t.Helper()

	dir := t.TempDir()
	logFile := filepath.Join(dir, "argv.log")

	script := "#!/bin/sh\n" +
		"for arg in \"$@\"; do printf '%s\\n' \"$arg\" >> \"" + logFile + "\"; done\n" +
		// The driver deletes the listing when the run ends, so the stub copies
		// it aside for the assertions.
		"if [ -n \"${5:-}\" ]; then cat \"$5\" > \"" + filepath.Join(dir, "listing") + "\"; fi\n" +
		"printf '%s' '" + payload + "'\n"

	if err := os.WriteFile(filepath.Join(dir, proto.SidecarName), []byte(script), 0o755); err != nil {
		t.Fatalf("write stub: %v", err)
	}

	return Assets{Dir: dir}, logFile
}

const scanPayload = `{"functions":[{"key":"app.ts#read","file":"app.ts","line":1,"cyclomatic":9,"cognitive":4}],"errors":[]}`

func TestRunComplexityDecodesTheSidecarMeasurement(t *testing.T) {
	repo := gitScratchRepo(t, map[string]string{
		"app.ts":    "export const a = 1;\n",
		"notes.md":  "# notes\n",
		"decl.d.ts": "declare const d: number;\nexport default d;\n",
	})

	support, logFile := writeScanStub(t, scanPayload)

	t.Setenv(proto.SourcesCwdEnv, repo)

	var stdout, stderr bytes.Buffer

	scan, err := NewInvoker(support).RunComplexity(context.Background(), Request{Stdout: &stdout, Stderr: &stderr})

	if err != nil {
		t.Fatalf("RunComplexity: %v\nstderr: %s", err, stderr.String())
	}

	if len(scan.Functions) != 1 || scan.Functions[0].Cyclomatic != 9 {
		t.Fatalf("functions = %#v", scan.Functions)
	}

	// Markdown is formattable but not scorable, and declarations are dropped.
	if len(scan.Files) != 1 || scan.Files[0] != "app.ts" {
		t.Fatalf("files = %#v", scan.Files)
	}

	argv, err := os.ReadFile(logFile)

	if err != nil {
		t.Fatalf("read argv log: %v", err)
	}

	lines := strings.Split(strings.TrimSpace(string(argv)), "\n")

	if len(lines) != 5 || lines[0] != proto.ModeComplexity || lines[1] != "--root" || lines[2] != repo || lines[3] != "--files-from" {
		t.Fatalf("argv = %q", lines)
	}

	listing, err := os.ReadFile(filepath.Join(filepath.Dir(logFile), "listing"))

	if err != nil {
		t.Fatalf("read listing: %v", err)
	}

	if string(listing) != filepath.Join(repo, "app.ts") {
		t.Fatalf("listing = %q", listing)
	}
}

func TestRunComplexityRemovesItsListing(t *testing.T) {
	repo := gitScratchRepo(t, map[string]string{"app.ts": "export const a = 1;\n"})
	support, logFile := writeScanStub(t, scanPayload)

	t.Setenv(proto.SourcesCwdEnv, repo)

	var stdout, stderr bytes.Buffer

	if _, err := NewInvoker(support).RunComplexity(context.Background(), Request{Stdout: &stdout, Stderr: &stderr}); err != nil {
		t.Fatalf("RunComplexity: %v", err)
	}

	argv, err := os.ReadFile(logFile)

	if err != nil {
		t.Fatalf("read argv log: %v", err)
	}

	listing := strings.Split(strings.TrimSpace(string(argv)), "\n")[4]

	if _, err := os.Stat(listing); !os.IsNotExist(err) {
		t.Fatalf("listing %s outlived the run", listing)
	}
}

func TestRunComplexitySkipsTheSidecarWithoutFiles(t *testing.T) {
	repo := gitScratchRepo(t, map[string]string{"notes.md": "# notes\n"})
	support, logFile := writeScanStub(t, scanPayload)

	t.Setenv(proto.SourcesCwdEnv, repo)

	var stdout, stderr bytes.Buffer

	scan, err := NewInvoker(support).RunComplexity(context.Background(), Request{Stdout: &stdout, Stderr: &stderr})

	if err != nil {
		t.Fatalf("RunComplexity: %v", err)
	}

	if len(scan.Functions) != 0 || len(scan.Files) != 0 {
		t.Fatalf("scan = %#v", scan)
	}

	if _, err := os.Stat(logFile); !os.IsNotExist(err) {
		t.Fatal("the sidecar was spawned for an empty file list")
	}
}

func TestRunComplexityReportsAnUndecodableMeasurement(t *testing.T) {
	repo := gitScratchRepo(t, map[string]string{"app.ts": "export const a = 1;\n"})
	support, _ := writeScanStub(t, "not json")

	t.Setenv(proto.SourcesCwdEnv, repo)

	var stdout, stderr bytes.Buffer

	_, err := NewInvoker(support).RunComplexity(context.Background(), Request{Stdout: &stdout, Stderr: &stderr})

	if err == nil || !strings.Contains(err.Error(), "decode complexity scan") {
		t.Fatalf("err = %v", err)
	}
}

func TestRunComplexityWarnsAboutMissingScopes(t *testing.T) {
	repo := gitScratchRepo(t, map[string]string{"app.ts": "export const a = 1;\n"})
	support, _ := writeScanStub(t, scanPayload)

	t.Setenv(proto.SourcesCwdEnv, repo)

	var stdout, stderr bytes.Buffer

	if _, err := NewInvoker(support).RunComplexity(context.Background(), Request{Scopes: []string{"missing"}, Stdout: &stdout, Stderr: &stderr}); err != nil {
		t.Fatalf("RunComplexity: %v", err)
	}

	if !strings.Contains(stderr.String(), "path not found, skipping") {
		t.Fatalf("stderr = %q", stderr.String())
	}
}
