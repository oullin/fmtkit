package cli

import (
	"fmt"
	"io"
	"os"
	"strings"

	driverconfig "go.ollin.sh/fmtkit/driver/config"
	driverreport "go.ollin.sh/fmtkit/driver/report"
	"go.ollin.sh/fmtkit/formatter"
	formatterconfig "go.ollin.sh/fmtkit/formatter/config"
	formatterengine "go.ollin.sh/fmtkit/formatter/engine"
	"go.ollin.sh/fmtkit/vet"
)

type Runner struct {
	stdout io.Writer
	stderr io.Writer
	parser parser
}

func NewRunner(stdout, stderr io.Writer) Runner {
	return Runner{
		stdout: stdout,
		stderr: stderr,
		parser: newParser(stderr),
	}
}

func (r Runner) Run(mode Mode, args []string) int {
	opts, err := r.parser.Parse(mode, args)

	if err != nil {
		return 1
	}

	workRoot, err := os.Getwd()

	if err != nil {
		r.writeError("resolve cwd: %v\n", err)

		return 1
	}

	reportRoot := workRoot

	if strings.TrimSpace(opts.reportRoot) != "" {
		reportRoot = opts.reportRoot
	}

	cfg, err := driverconfig.Load(reportRoot, opts.configPath)

	if err != nil {
		r.writeError("%v\n", err)

		return 1
	}

	runPaths := opts.positional

	formatterCfg := cfg.FormatterConfig()

	if opts.jobs != -1 {
		formatterCfg.Concurrency = opts.jobs
	}

	formatterReport, err := r.runFormatter(mode, runPaths, formatterCfg)

	if err != nil {
		r.writeError("%v\n", err)

		return 1
	}

	result := driverreport.Combined{
		Formatter: formatterReport,
		Vet:       vet.Run(workRoot, cfg.VetConfig()),
	}

	if err := driverreport.Render(r.stdout, opts.outputFormat, reportRoot, mode.String(), result); err != nil {
		r.writeError("render report: %v\n", err)

		return 1
	}

	return exitCode(mode, result)
}

func (r Runner) runFormatter(mode Mode, paths []string, cfg formatterconfig.Config) (formatterengine.Report, error) {
	switch mode {
	case CheckMode:
		return formatter.Check(paths, cfg)
	case FormatMode:
		return formatter.Format(paths, cfg)
	default:
		return formatterengine.Report{}, fmt.Errorf("unsupported mode %q", mode)
	}
}

func exitCode(mode Mode, result driverreport.Combined) int {
	if result.Vet.ErrorCount() > 0 {
		return 1
	}

	if mode == CheckMode {
		if result.Formatter.Result == "pass" {
			return 0
		}

		return 1
	}

	if result.Formatter.ErrorCount() > 0 {
		return 1
	}

	return 0
}

func (r Runner) writeError(format string, args ...any) {
	_, _ = fmt.Fprintf(r.stderr, format, args...)
}
