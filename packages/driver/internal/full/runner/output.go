package runner

import (
	"fmt"
	"io"
)

func (r Runner) section(label string)              { writef(r.stderr, "\n==> %s\n", label) }
func (r Runner) detail(label string, value string) { writef(r.stderr, "    %-12s %s\n", label, value) }
func (r Runner) failure(label string)              { writef(r.stderr, "\n!! %s\n", label) }

func writef(writer io.Writer, format string, args ...any) {
	_, _ = fmt.Fprintf(writer, format, args...)
}

func (r Runner) printUsage() {
	writef(r.stderr, "usage: fmt-all <format|format-all|go|ts|check|version|help> [args...]\n")
	writef(r.stderr, "  format [paths...]                        run TS/Vue support + lint, then Go formatting\n")
	writef(r.stderr, "  format-all                               run the full formatter pipeline against .\n")
	writef(r.stderr, "  go [check|format|sources|version|help] [args...] run the Go formatter CLI\n")
	writef(r.stderr, "  ts [paths...]                            run TS/Vue formatting support and oxfmt\n")
	writef(r.stderr, "  check|version|help [args...]             run the matching Go formatter CLI command\n")
}

func (r Runner) printGoUsage() {
	writef(r.stderr, "go-fmt check [--host-path /absolute/host/path] [paths...]\n\ngo-fmt format [--host-path /absolute/host/path] [paths...]\n\ngo-fmt sources [--include-declarations] [paths...]\n")
}
