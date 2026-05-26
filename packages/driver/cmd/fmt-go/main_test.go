package main

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/oullin/go-fmt/packages/driver/internal/cli"
	"github.com/oullin/go-fmt/packages/driver/testutil"
)

func TestRunCheckFailsOnStyleChanges(t *testing.T) {
	dir := t.TempDir()
	testutil.WriteGoFile(t, filepath.Join(dir, "sample.go"), `package sample

func run() {
	if true {
		println("ok")
	}
	println("next")
}
`)

	exitCode, stdout, stderr := runCLI(t, dir, "check", dir)

	if exitCode != 1 {
		t.Fatalf("expected exit code 1, got %d", exitCode)
	}

	if !strings.Contains(stdout, "Result: fail") {
		t.Fatalf("unexpected stdout:\n%s", stdout)
	}

	if !strings.Contains(stdout, "Formatter") || !strings.Contains(stdout, "Vet") {
		t.Fatalf("expected sectioned output, got:\n%s", stdout)
	}

	if stderr != "" {
		t.Fatalf("unexpected stderr:\n%s", stderr)
	}
}

func TestRunFormatWritesChanges(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "sample.go")
	testutil.WriteGoFile(t, path, `package sample

func run() {
	defer println("done")
	return
}
`)

	exitCode, stdout, stderr := runCLI(t, dir, "format", dir)

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d", exitCode)
	}

	if !strings.Contains(stdout, "Result: fixed") {
		t.Fatalf("unexpected stdout:\n%s", stdout)
	}

	if stderr != "" {
		t.Fatalf("unexpected stderr:\n%s", stderr)
	}

	content, err := os.ReadFile(path)

	if err != nil {
		t.Fatalf("read file: %v", err)
	}

	if !strings.Contains(string(content), "defer println(\"done\")\n\n\treturn") {
		t.Fatalf("expected formatted file, got:\n%s", content)
	}
}

func TestRunFormatSkipsSingleLineFuncLiteralSpacingViolations(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "sample.go")
	testutil.WriteGoFile(t, path, `package sample

type config struct {
	SecureCookie bool
}

func run() config {
	return config{
		SecureCookie: func() bool { value := true; return value }(),
	}
}
`)

	exitCode, stdout, stderr := runCLI(t, dir, "format", dir)

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d", exitCode)
	}

	if !strings.Contains(stdout, "Result: pass. 0 changed, 0 violation(s), 0 error(s).") {
		t.Fatalf("unexpected stdout:\n%s", stdout)
	}

	if stderr != "" {
		t.Fatalf("unexpected stderr:\n%s", stderr)
	}

	content, err := os.ReadFile(path)

	if err != nil {
		t.Fatalf("read file: %v", err)
	}

	if !strings.Contains(string(content), "SecureCookie: func() bool { value := true; return value }(),") {
		t.Fatalf("expected file to stay unchanged, got:\n%s", content)
	}
}

func TestRunAgentOutput(t *testing.T) {
	dir := t.TempDir()
	testutil.WriteGoFile(t, filepath.Join(dir, "sample.go"), `package sample

func run() {
	if true {
		println("ok")
	}
	println("next")
}
`)

	exitCode, stdout, stderr := runCLI(t, dir, "check", "--format", "agent", dir)

	if exitCode != 1 {
		t.Fatalf("expected exit code 1, got %d", exitCode)
	}

	if !strings.Contains(stdout, "\"formatter\"") || !strings.Contains(stdout, "\"vet\"") {
		t.Fatalf("unexpected agent output:\n%s", stdout)
	}

	if stderr != "" {
		t.Fatalf("unexpected stderr:\n%s", stderr)
	}
}

func TestRunCheckWithHostPath(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "sample.go")
	testutil.WriteGoFile(t, path, `package sample

func run() {
	if true {
		println("ok")
	}
	println("next")
}
`)
	t.Setenv(cli.HostRootEnv, dir)

	exitCode, stdout, stderr := runCLI(t, dir, "check", "--host-path", path)

	if exitCode != 1 {
		t.Fatalf("expected exit code 1, got %d", exitCode)
	}

	if !strings.Contains(stdout, "Result: fail") {
		t.Fatalf("unexpected stdout:\n%s", stdout)
	}

	if stderr != "" {
		t.Fatalf("unexpected stderr:\n%s", stderr)
	}
}

func TestRunFormatWithHostPath(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "sample.go")
	testutil.WriteGoFile(t, path, `package sample

func run() {
	defer println("done")
	return
}
`)
	t.Setenv(cli.HostRootEnv, dir)

	exitCode, stdout, stderr := runCLI(t, dir, "format", "--host-path", path)

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d", exitCode)
	}

	if !strings.Contains(stdout, "Result: fixed") {
		t.Fatalf("unexpected stdout:\n%s", stdout)
	}

	if stderr != "" {
		t.Fatalf("unexpected stderr:\n%s", stderr)
	}

	content, err := os.ReadFile(path)

	if err != nil {
		t.Fatalf("read file: %v", err)
	}

	if !strings.Contains(string(content), "defer println(\"done\")\n\n\treturn") {
		t.Fatalf("expected formatted file, got:\n%s", content)
	}
}

func TestRunWithHostPathRequiresEnv(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "sample.go")
	testutil.WriteGoFile(t, path, "package sample\n")

	exitCode, _, stderr := runCLI(t, dir, "check", "--host-path", path)

	if exitCode != 1 {
		t.Fatalf("expected exit code 1, got %d", exitCode)
	}

	if !strings.Contains(stderr, cli.HostRootEnv) {
		t.Fatalf("unexpected stderr:\n%s", stderr)
	}
}

func TestRunWithHostPathRejectsPositionalPaths(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "sample.go")
	testutil.WriteGoFile(t, path, "package sample\n")
	t.Setenv(cli.HostRootEnv, dir)

	exitCode, _, stderr := runCLI(t, dir, "check", "--host-path", path, dir)

	if exitCode != 1 {
		t.Fatalf("expected exit code 1, got %d", exitCode)
	}

	if !strings.Contains(stderr, "cannot be used with positional paths") {
		t.Fatalf("unexpected stderr:\n%s", stderr)
	}
}

func TestPrintUsage(t *testing.T) {
	tests := []struct {
		name string
		args []string
	}{
		{"no args", nil},
		{"help subcommand", []string{"help"}},
		{"help flag", []string{"--help"}},
		{"short help flag", []string{"-h"}},
		{"unknown subcommand", []string{"unknown"}},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			exitCode, stdout, stderr := runCLI(t, t.TempDir(), tt.args...)

			// For unknown subcommand, exit code is 1. For help, it is 0. For no args, it is 1.
			expectedExitCode := 0

			if tt.name == "no args" || tt.name == "unknown subcommand" {
				expectedExitCode = 1
			}

			if exitCode != expectedExitCode {
				t.Errorf("expected exit code %d, got %d", expectedExitCode, exitCode)
			}

			if stdout != "" {
				t.Errorf("expected empty stdout, got %q", stdout)
			}

			if !strings.Contains(stderr, "go-fmt check [--host-path /absolute/host/path] [paths...]") {
				t.Errorf("expected stderr to contain usage, got %q", stderr)
			}

			if strings.Contains(stderr, "{%v}") {
				t.Errorf("stderr contains literal {%%v}: %q", stderr)
			}

			if tt.name == "unknown subcommand" && !strings.Contains(stderr, "unknown subcommand - {\"unknown\"}") {
				t.Errorf("expected stderr to contain unknown subcommand error, got %q", stderr)
			}
		})
	}
}

func TestVersion(t *testing.T) {
	exitCode, stdout, stderr := runCLI(t, t.TempDir(), "version")

	if exitCode != 0 {
		t.Errorf("expected exit code 0, got %d", exitCode)
	}

	if !strings.Contains(stdout, "go-fmt dev") {
		t.Errorf("expected stdout to contain version, got %q", stdout)
	}

	if stderr != "" {
		t.Errorf("expected empty stderr, got %q", stderr)
	}
}

func TestRunCheckRunsGoVetInModule(t *testing.T) {
	dir := writeTempModule(t, "example.com/sample")
	testutil.WriteGoFile(t, filepath.Join(dir, "sample.go"), `package sample

import "fmt"

func run() {
	fmt.Printf("%d", "not-a-number")
}
`)

	exitCode, stdout, stderr := runCLI(t, dir, "check", dir)

	if exitCode != 1 {
		t.Fatalf("expected exit code 1, got %d", exitCode)
	}

	if !strings.Contains(stdout, "automatic go vet ./... failed") {
		t.Fatalf("expected stdout to include go vet failure, got:\n%s", stdout)
	}

	if !strings.Contains(stdout, "Printf format %d has arg \"not-a-number\" of wrong type string") {
		t.Fatalf("expected stdout to include vet details, got:\n%s", stdout)
	}

	if stderr != "" {
		t.Fatalf("unexpected stderr:\n%s", stderr)
	}
}

func TestRunFormatRunsGoVetAfterWritingChanges(t *testing.T) {
	dir := writeTempModule(t, "example.com/sample")
	path := filepath.Join(dir, "sample.go")
	testutil.WriteGoFile(t, path, `package sample

func run() {
	defer println("done")
	return
}
`)

	exitCode, stdout, stderr := runCLI(t, dir, "format", dir)

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d", exitCode)
	}

	if !strings.Contains(stdout, "Result: fixed") {
		t.Fatalf("unexpected stdout:\n%s", stdout)
	}

	if strings.Contains(stdout, "automatic go vet ./... failed") {
		t.Fatalf("did not expect vet failure output, got:\n%s", stdout)
	}

	if stderr != "" {
		t.Fatalf("unexpected stderr:\n%s", stderr)
	}

	content, err := os.ReadFile(path)

	if err != nil {
		t.Fatalf("read file: %v", err)
	}

	if !strings.Contains(string(content), "defer println(\"done\")\n\n\treturn") {
		t.Fatalf("expected formatted file, got:\n%s", content)
	}
}

func TestRunFormatReportsGoVetFailureAfterWritingChanges(t *testing.T) {
	dir := writeTempModule(t, "example.com/sample")
	path := filepath.Join(dir, "sample.go")
	testutil.WriteGoFile(t, path, `package sample

import "fmt"

func run() {
	defer fmt.Printf("%d", "not-a-number")
	return
}
`)

	exitCode, stdout, stderr := runCLI(t, dir, "format", dir)

	if exitCode != 1 {
		t.Fatalf("expected exit code 1, got %d", exitCode)
	}

	if !strings.Contains(stdout, "automatic go vet ./... failed") {
		t.Fatalf("expected stdout to include go vet failure, got:\n%s", stdout)
	}

	content, err := os.ReadFile(path)

	if err != nil {
		t.Fatalf("read file: %v", err)
	}

	if !strings.Contains(string(content), "defer fmt.Printf(\"%d\", \"not-a-number\")\n\n\treturn") {
		t.Fatalf("expected formatted file to be written before vet failure, got:\n%s", content)
	}

	if stderr != "" {
		t.Fatalf("unexpected stderr:\n%s", stderr)
	}
}

func TestRunOutsideModuleSkipsGoVet(t *testing.T) {
	dir := t.TempDir()
	testutil.WriteGoFile(t, filepath.Join(dir, "sample.go"), `package sample

func run() {
	defer println("done")
	return
}
`)

	exitCode, stdout, stderr := runCLI(t, dir, "check", dir)

	if exitCode != 1 {
		t.Fatalf("expected exit code 1, got %d", exitCode)
	}

	if strings.Contains(stdout, "automatic go vet ./... failed") {
		t.Fatalf("expected go vet to be skipped outside a module, got:\n%s", stdout)
	}

	if stderr != "" {
		t.Fatalf("unexpected stderr:\n%s", stderr)
	}
}

func TestRunWithHostPathRunsGoVetInModule(t *testing.T) {
	dir := writeTempModule(t, "example.com/sample")
	path := filepath.Join(dir, "sample.go")
	testutil.WriteGoFile(t, path, `package sample

import "fmt"

func run() {
	fmt.Printf("%d", "not-a-number")
}
`)
	t.Setenv(cli.HostRootEnv, dir)

	exitCode, stdout, stderr := runCLI(t, dir, "check", "--host-path", path)

	if exitCode != 1 {
		t.Fatalf("expected exit code 1, got %d", exitCode)
	}

	if !strings.Contains(stdout, "automatic go vet ./... failed") {
		t.Fatalf("expected stdout to include go vet failure, got:\n%s", stdout)
	}

	if stderr != "" {
		t.Fatalf("unexpected stderr:\n%s", stderr)
	}
}

func TestRunCheckWithVetDisabledSkipsGoVet(t *testing.T) {
	dir := writeTempModule(t, "example.com/sample")
	configPath := filepath.Join(dir, "config.yml")
	testutil.WriteFile(t, configPath, "vet:\n  enabled: false\n")
	testutil.WriteGoFile(t, filepath.Join(dir, "sample.go"), `package sample

import "fmt"

func run() {
	fmt.Printf("%d", "not-a-number")
}
`)

	exitCode, stdout, stderr := runCLI(t, dir, "check", "--config", configPath, dir)

	if exitCode != 0 {
		t.Fatalf("expected exit code 0, got %d", exitCode)
	}

	if strings.Contains(stdout, "automatic go vet ./... failed") {
		t.Fatalf("expected go vet to be disabled, got:\n%s", stdout)
	}

	if stderr != "" {
		t.Fatalf("unexpected stderr:\n%s", stderr)
	}
}

func runCLI(t *testing.T, workdir string, args ...string) (int, string, string) {
	t.Helper()

	oldwd, err := os.Getwd()

	if err != nil {
		t.Fatalf("getwd: %v", err)
	}

	if err := os.Chdir(workdir); err != nil {
		t.Fatalf("chdir: %v", err)
	}

	defer func() {
		_ = os.Chdir(oldwd)
	}()

	var stdout strings.Builder

	var stderr strings.Builder
	exitCode := run(args, &stdout, &stderr)

	return exitCode, stdout.String(), stderr.String()
}

func writeTempModule(t *testing.T, modulePath string) string {
	t.Helper()

	dir := t.TempDir()
	testutil.WriteGoMod(t, dir, modulePath)

	return dir
}
