package gotool

import (
	"io"
	"os"
	"reflect"
	"testing"

	report "go.ollin.sh/fmtkit/driver/report"
)

func TestParseInvocationDefaults(t *testing.T) {
	unsetJobsEnv(t)

	inv, err := ParseInvocation(report.ModeCheck, nil, io.Discard)

	if err != nil {
		t.Fatalf("parse: %v", err)
	}

	want := Invocation{
		Mode:   report.ModeCheck,
		Output: report.FormatText,
		Jobs:   -1,
	}

	if !reflect.DeepEqual(inv, want) {
		t.Fatalf("unexpected defaults: %#v", inv)
	}
}

func TestParseInvocationAllFlags(t *testing.T) {
	unsetJobsEnv(t)

	args := []string{
		"--config", "custom.yml",
		"--cwd", "/repo",
		"--format", "json",
		"--jobs", "4",
		"main.go", "pkg",
	}

	inv, err := ParseInvocation(report.ModeFormat, args, io.Discard)

	if err != nil {
		t.Fatalf("parse: %v", err)
	}

	want := Invocation{
		Mode:       report.ModeFormat,
		ConfigPath: "custom.yml",
		ReportRoot: "/repo",
		Output:     report.FormatJSON,
		Paths:      []string{"main.go", "pkg"},
		Jobs:       4,
	}

	if !reflect.DeepEqual(inv, want) {
		t.Fatalf("unexpected invocation: %#v", inv)
	}
}

func TestParseInvocationRejectsUnknownFlag(t *testing.T) {
	if _, err := ParseInvocation(report.ModeCheck, []string{"--bogus"}, io.Discard); err == nil {
		t.Fatal("expected unknown flag error")
	}
}

func TestParseInvocationRejectsNonNumericJobs(t *testing.T) {
	if _, err := ParseInvocation(report.ModeCheck, []string{"--jobs", "abc"}, io.Discard); err == nil {
		t.Fatal("expected invalid --jobs error")
	}
}

func TestParseInvocationRejectsUnknownFormat(t *testing.T) {
	if _, err := ParseInvocation(report.ModeCheck, []string{"--format", "yaml"}, io.Discard); err == nil {
		t.Fatal("expected unsupported format error")
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
				t.Setenv("FMTKIT_JOBS", tc.value)
			} else {
				unsetJobsEnv(t)
			}

			if got := envJobs(); got != tc.want {
				t.Fatalf("envJobs() = %d, want %d", got, tc.want)
			}
		})
	}
}

// unsetJobsEnv removes FMTKIT_JOBS for the test while registering the
// t.Setenv cleanup that restores the original value afterwards.
func unsetJobsEnv(t *testing.T) {
	t.Helper()

	t.Setenv("FMTKIT_JOBS", "")

	if err := os.Unsetenv("FMTKIT_JOBS"); err != nil {
		t.Fatalf("unset FMTKIT_JOBS: %v", err)
	}
}
