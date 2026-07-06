package cli

import (
	"bytes"
	"os"
	"path/filepath"
	"strings"
	"testing"

	driverreport "github.com/oullin/fmtkit/packages/driver/report"
	formatterengine "github.com/oullin/fmtkit/packages/formatter/engine"
	"github.com/oullin/fmtkit/packages/vet"
)

func TestExitCode(t *testing.T) {
	cases := []struct {
		name   string
		mode   Mode
		result driverreport.Combined
		want   int
	}{
		{
			name:   "vet errors fail either mode",
			mode:   FormatMode,
			result: driverreport.Combined{Vet: vet.Report{Errors: []vet.ErrorResult{{Message: "boom"}}}},
			want:   1,
		},
		{
			name:   "check passes on pass result",
			mode:   CheckMode,
			result: driverreport.Combined{Formatter: formatterengine.Report{Result: "pass"}},
			want:   0,
		},
		{
			name:   "check fails on non-pass result",
			mode:   CheckMode,
			result: driverreport.Combined{Formatter: formatterengine.Report{Result: "fail"}},
			want:   1,
		},
		{
			name: "format fails on formatter errors",
			mode: FormatMode,
			result: driverreport.Combined{Formatter: formatterengine.Report{
				Result: "fail",
				Errors: []formatterengine.ErrorResult{{Message: "walk failed"}},
			}},
			want: 1,
		},
		{
			name:   "format succeeds after applying fixes",
			mode:   FormatMode,
			result: driverreport.Combined{Formatter: formatterengine.Report{Result: "fixed"}},
			want:   0,
		},
	}

	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			if got := exitCode(tc.mode, tc.result); got != tc.want {
				t.Fatalf("exitCode(%s) = %d, want %d", tc.mode, got, tc.want)
			}
		})
	}
}

const cleanSource = `package sample

func run() {
	println("ok")
}
`

// spacingViolationSource is missing the blank line the spacing rule
// requires between the defer statement and the return.
const spacingViolationSource = `package sample

func run() {
	defer println("done")
	return
}
`

// runInTempModulelessDir writes source to a Go file in a fresh temp dir,
// chdirs there (so vet finds no module and skips), and runs the CLI.
func runInTempModulelessDir(t *testing.T, source string, mode Mode, extraArgs ...string) (code int, stdout, stderr string, file string) {
	t.Helper()

	dir := t.TempDir()
	file = filepath.Join(dir, "sample.go")

	if err := os.WriteFile(file, []byte(source), 0o644); err != nil {
		t.Fatalf("write sample: %v", err)
	}

	t.Chdir(dir)

	var out, errOut bytes.Buffer

	code = NewRunner(&out, &errOut).Run(mode, append(extraArgs, file))

	return code, out.String(), errOut.String(), file
}

func TestRunnerRunCleanFileJSON(t *testing.T) {
	code, stdout, stderr, _ := runInTempModulelessDir(t, cleanSource, CheckMode, "--format", "json")

	if code != 0 {
		t.Fatalf("exit = %d, stderr: %s", code, stderr)
	}

	if !strings.Contains(stdout, `"result":"pass"`) {
		t.Fatalf("unexpected json output: %s", stdout)
	}

	if !strings.Contains(stdout, `"status":"skipped"`) {
		t.Fatalf("expected vet skipped in module-less dir: %s", stdout)
	}
}

func TestRunnerRunCheckModeReportsViolation(t *testing.T) {
	code, stdout, _, file := runInTempModulelessDir(t, spacingViolationSource, CheckMode)

	if code != 1 {
		t.Fatalf("exit = %d, stdout: %s", code, stdout)
	}

	content, err := os.ReadFile(file)

	if err != nil {
		t.Fatalf("read sample: %v", err)
	}

	if string(content) != spacingViolationSource {
		t.Fatal("check mode must not rewrite the file")
	}
}

func TestRunnerRunFormatModeRewritesFile(t *testing.T) {
	code, stdout, stderr, file := runInTempModulelessDir(t, spacingViolationSource, FormatMode)

	if code != 0 {
		t.Fatalf("exit = %d, stdout: %s, stderr: %s", code, stdout, stderr)
	}

	content, err := os.ReadFile(file)

	if err != nil {
		t.Fatalf("read sample: %v", err)
	}

	if string(content) == spacingViolationSource {
		t.Fatal("format mode should rewrite the file")
	}

	if !strings.Contains(string(content), "defer println(\"done\")\n\n\treturn") {
		t.Fatalf("expected blank line inserted after defer, got:\n%s", content)
	}
}

func TestRunnerRunRejectsUnsupportedFormat(t *testing.T) {
	code, _, stderr, _ := runInTempModulelessDir(t, cleanSource, CheckMode, "--format", "yaml")

	if code != 1 {
		t.Fatalf("exit = %d", code)
	}

	if !strings.Contains(stderr, "unsupported output format") {
		t.Fatalf("unexpected stderr: %s", stderr)
	}
}

func TestRunnerRunRejectsRelativeHostPath(t *testing.T) {
	dir := t.TempDir()

	t.Chdir(dir)

	var out, errOut bytes.Buffer

	code := NewRunner(&out, &errOut).Run(CheckMode, []string{"--host-path", "relative/path"})

	if code != 1 {
		t.Fatalf("exit = %d", code)
	}

	if !strings.Contains(errOut.String(), "--host-path must be an absolute path") {
		t.Fatalf("unexpected stderr: %s", errOut.String())
	}
}

func TestRunnerRunRejectsUnknownFlag(t *testing.T) {
	dir := t.TempDir()

	t.Chdir(dir)

	var out, errOut bytes.Buffer

	if code := NewRunner(&out, &errOut).Run(CheckMode, []string{"--bogus"}); code != 1 {
		t.Fatalf("exit = %d", code)
	}
}
