package cli

import (
	"io"
	"os"
	"reflect"
	"testing"
)

func TestParseDefaults(t *testing.T) {
	unsetJobsEnv(t)

	opts, err := newParser(io.Discard).Parse(CheckMode, nil)

	if err != nil {
		t.Fatalf("parse: %v", err)
	}

	want := options{
		mode:         CheckMode,
		outputFormat: "text",
		jobs:         -1,
	}

	if !reflect.DeepEqual(opts, want) {
		t.Fatalf("unexpected defaults: %#v", opts)
	}
}

func TestParseAllFlags(t *testing.T) {
	unsetJobsEnv(t)

	args := []string{
		"--config", "custom.yml",
		"--cwd", "/repo",
		"--format", "json",
		"--host-path", "/host/project",
		"--jobs", "4",
		"main.go", "pkg",
	}

	opts, err := newParser(io.Discard).Parse(FormatMode, args)

	if err != nil {
		t.Fatalf("parse: %v", err)
	}

	want := options{
		mode:         FormatMode,
		configPath:   "custom.yml",
		reportRoot:   "/repo",
		outputFormat: "json",
		hostPath:     HostPath("/host/project"),
		positional:   []string{"main.go", "pkg"},
		jobs:         4,
	}

	if !reflect.DeepEqual(opts, want) {
		t.Fatalf("unexpected options: %#v", opts)
	}
}

func TestParseRejectsUnknownFlag(t *testing.T) {
	if _, err := newParser(io.Discard).Parse(CheckMode, []string{"--bogus"}); err == nil {
		t.Fatal("expected unknown flag error")
	}
}

func TestParseRejectsNonNumericJobs(t *testing.T) {
	if _, err := newParser(io.Discard).Parse(CheckMode, []string{"--jobs", "abc"}); err == nil {
		t.Fatal("expected invalid --jobs error")
	}
}

func TestEnvJobs(t *testing.T) {
	cases := []struct {
		name  string
		value string
		set   bool
		want  int
	}{
		{name: "unset", set: false, want: -1},
		{name: "empty", value: "", set: true, want: -1},
		{name: "whitespace", value: "   ", set: true, want: -1},
		{name: "non-numeric", value: "abc", set: true, want: -1},
		{name: "negative", value: "-2", set: true, want: -1},
		{name: "zero", value: "0", set: true, want: 0},
		{name: "positive", value: "8", set: true, want: 8},
		{name: "padded", value: " 3 ", set: true, want: 3},
	}

	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			if tc.set {
				t.Setenv("GO_FMT_JOBS", tc.value)
			} else {
				unsetJobsEnv(t)
			}

			if got := envJobs(); got != tc.want {
				t.Fatalf("envJobs() = %d, want %d", got, tc.want)
			}
		})
	}
}

// unsetJobsEnv removes GO_FMT_JOBS for the test while registering the
// t.Setenv cleanup that restores the original value afterwards.
func unsetJobsEnv(t *testing.T) {
	t.Helper()

	t.Setenv("GO_FMT_JOBS", "")

	if err := os.Unsetenv("GO_FMT_JOBS"); err != nil {
		t.Fatalf("unset GO_FMT_JOBS: %v", err)
	}
}
