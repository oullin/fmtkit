package app

import (
	"context"
	"fmt"
	"os"
	"slices"
	"strings"

	"go.ollin.sh/fmtkit/complexity"
	driverconfig "go.ollin.sh/fmtkit/driver/config"
	"go.ollin.sh/fmtkit/driver/internal/typescript/runtime"
	report "go.ollin.sh/fmtkit/driver/report"
)

// complexityOptions is the parsed `fmtkit complexity` command line.
type complexityOptions struct {
	// lanes names the languages to score. Empty means both, matching the
	// no-flag default of the format commands.
	lanes  []string
	quiet  bool
	config string
	output report.Format
	paths  []string
}

// wants reports whether a lane runs under these options.
func (o complexityOptions) wants(lane string) bool {
	return len(o.lanes) == 0 || slices.Contains(o.lanes, lane)
}

// parseComplexityArgs splits the complexity flags from the paths.
func parseComplexityArgs(args []string) (complexityOptions, error) {
	opts := complexityOptions{output: report.FormatText}

	for index := 0; index < len(args); index++ {
		argument := args[index]

		switch argument {
		case "--ts", "--go":
			opts.lanes = append(opts.lanes, strings.TrimPrefix(argument, "--"))
		case "--quiet", "-q":
			opts.quiet = true
		case "--config":
			index++
			opts.config = valueAt(args, index)
		case "--format":
			index++

			format, err := report.ParseFormat(valueAt(args, index))

			if err != nil {
				return complexityOptions{}, err
			}

			opts.output = format
		default:
			if strings.HasPrefix(argument, "-") {
				return complexityOptions{}, fmt.Errorf("unknown flag - {%q}", argument)
			}

			opts.paths = append(opts.paths, argument)
		}
	}

	return opts, nil
}

// valueAt reads a flag's value, returning "" when the flag ended the argv.
func valueAt(args []string, index int) string {
	if index >= len(args) {
		return ""
	}

	return args[index]
}

// runComplexity scores both lanes (or the selected one) and reports the
// functions over the configured limits. It never writes source.
func (d *deps) runComplexity(ctx context.Context, args []string) int {
	opts, err := parseComplexityArgs(args)

	if err != nil {
		_, _ = fmt.Fprintf(d.stderr, "%v\n\n", err)

		d.usage(d.stderr)

		return 2
	}

	root, err := os.Getwd()

	if err != nil {
		_, _ = fmt.Fprintf(d.stderr, "fmtkit: resolve cwd: %v\n", err)

		return 1
	}

	cfg, err := driverconfig.Load(root, opts.config)

	if err != nil {
		_, _ = fmt.Fprintf(d.stderr, "fmtkit: %v\n", err)

		return 1
	}

	scans, err := d.complexityScans(ctx, opts, cfg, root)

	if err != nil {
		return d.reportError(err)
	}

	outcome := complexity.Evaluate(root, cfg.ComplexityConfig(), scans)

	return d.renderComplexity(root, opts, outcome)
}

// complexityScans runs the selected lanes and returns their measurements.
func (d *deps) complexityScans(ctx context.Context, opts complexityOptions, cfg driverconfig.Config, root string) ([]complexity.Scan, error) {
	paths := opts.paths

	if len(paths) == 0 {
		paths = []string{"."}
	}

	var scans []complexity.Scan

	if opts.wants("ts") {
		scan, err := d.scanTypeScript(ctx, paths)

		if err != nil {
			return nil, err
		}

		scans = append(scans, scan)
	}

	if opts.wants("go") {
		scans = append(scans, complexity.ScanGo(root, paths, cfg.Formatter()))
	}

	return scans, nil
}

// scanTypeScript scores the TS lane through the sidecar.
func (d *deps) scanTypeScript(ctx context.Context, paths []string) (complexity.Scan, error) {
	assets, err := runtime.Resolve(d.version)

	if err != nil {
		return complexity.Scan{}, err
	}

	return runtime.NewInvoker(assets).RunComplexity(ctx, runtime.Request{Scopes: paths, Stdout: d.stdout, Stderr: d.stderr})
}

// renderComplexity writes the report and reduces it to an exit code. Quiet
// keeps a passing run silent; a failing one always prints its findings.
func (d *deps) renderComplexity(root string, opts complexityOptions, outcome complexity.Report) int {
	combined := report.Combined{Complexity: &outcome}
	code := combined.ExitCode(report.ModeComplexity)

	if opts.quiet && code == 0 {
		return 0
	}

	renderer := report.Renderer{Root: root, Mode: report.ModeComplexity}

	if err := renderer.Render(d.stdout, opts.output, combined); err != nil {
		_, _ = fmt.Fprintf(d.stderr, "render report: %v\n", err)

		return 1
	}

	return code
}
