package gotool

import (
	"flag"
	"fmt"
	"io"
	"os"
	"strconv"
	"strings"

	report "go.ollin.sh/fmtkit/driver/report"
)

// Invocation is the parsed form of a `check`/`format` command line: the flags
// (--config --cwd --format --jobs, plus FMTKIT_JOBS) resolved to typed values
// and the positional paths.
type Invocation struct {
	Mode       report.Mode
	ConfigPath string
	ReportRoot string
	Output     report.Format
	Paths      []string

	// Jobs overrides config.Concurrency when not -1. -1 means "unset" (no
	// override); 0 means "use NumCPU"; positive values pin the worker count.
	Jobs int
}

// ParseInvocation parses the shared check/format flag set for mode. Flag errors
// (already reported to stderr by the flag package) and an unknown --format
// value both surface as an error so the caller can exit non-zero.
func ParseInvocation(mode report.Mode, args []string, stderr io.Writer) (Invocation, error) {
	fs := flag.NewFlagSet(string(mode), flag.ContinueOnError)
	fs.SetOutput(stderr)

	configPath := fs.String("config", "", "Path to fmtkit YAML config")
	reportRoot := fs.String("cwd", "", "Path used for config discovery and report-relative file paths")
	outputFormat := fs.String("format", "text", "Output format: text, json, agent")
	jobs := fs.Int("jobs", envJobs(), "Max files processed in parallel (0 = NumCPU; also reads FMTKIT_JOBS)")

	if err := fs.Parse(args); err != nil {
		return Invocation{}, err
	}

	format, err := report.ParseFormat(*outputFormat)

	if err != nil {
		_, _ = fmt.Fprintf(stderr, "%v\n", err)

		return Invocation{}, err
	}

	return Invocation{
		Mode:       mode,
		ConfigPath: *configPath,
		ReportRoot: *reportRoot,
		Output:     format,
		Paths:      fs.Args(),
		Jobs:       *jobs,
	}, nil
}

// envJobs reads FMTKIT_JOBS as the default for the --jobs flag.
// Returns -1 when the env var is unset so the runner can distinguish
// "unset" from an explicit 0 (which means "use NumCPU").
// Invalid values fall back to -1 as well.
func envJobs() int {
	val, ok := os.LookupEnv("FMTKIT_JOBS")

	if !ok {
		return -1
	}

	raw := strings.TrimSpace(val)

	if raw == "" {
		return -1
	}

	n, err := strconv.Atoi(raw)

	if err != nil || n < 0 {
		return -1
	}

	return n
}
