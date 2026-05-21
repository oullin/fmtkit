package cli

import (
	"flag"
	"io"
	"os"
	"strconv"
	"strings"
)

type parser struct {
	stderr io.Writer
}

func newParser(stderr io.Writer) parser {
	return parser{stderr: stderr}
}

func (p parser) Parse(mode Mode, args []string) (options, error) {
	fs := flag.NewFlagSet(mode.String(), flag.ContinueOnError)
	fs.SetOutput(p.stderr)

	configPath := fs.String("config", "", "Path to go-fmt YAML config")
	reportRoot := fs.String("cwd", "", "Path used for config discovery and report-relative file paths")
	outputFormat := fs.String("format", "text", "Output format: text, json, agent")
	hostPath := fs.String("host-path", "", "Absolute host path under HOST_PROJECT_PATH to check or format")
	jobs := fs.Int("jobs", envJobs(), "Max files processed in parallel (0 = NumCPU; also reads GO_FMT_JOBS)")

	if err := fs.Parse(args); err != nil {
		return options{}, err
	}

	return options{
		mode:         mode,
		configPath:   *configPath,
		reportRoot:   *reportRoot,
		outputFormat: *outputFormat,
		hostPath:     HostPath(*hostPath),
		positional:   fs.Args(),
		jobs:         *jobs,
	}, nil
}

// envJobs reads GO_FMT_JOBS as the default for the --jobs flag.
// Returns -1 when the env var is unset so the runner can distinguish
// "unset" from an explicit 0 (which means "use NumCPU").
// Invalid values fall back to -1 as well.
func envJobs() int {
	val, ok := os.LookupEnv("GO_FMT_JOBS")

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
