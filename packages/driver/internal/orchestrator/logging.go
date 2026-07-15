// Package orchestrator drives the full fmtkit formatting pipeline (TS/Vue
// formatting, TS/Vue lint, Go formatting) with the sectioned, colorized
// progress output the infra/bin/fmtkit entrypoint established. Unlike the
// bash entrypoint, tool output streams live (indented under each section)
// and is followed by the condensed summary lines.
package orchestrator

import (
	"fmt"
	"io"
	"os"
	"strings"

	"github.com/mattn/go-isatty"
)

type logger struct {
	w     io.Writer
	quiet bool

	bold  string
	dim   string
	cyan  string
	green string
	red   string
	reset string
}

// stream returns a writer that renders tool output live, dimmed and indented
// under the current section. Callers must Close it to flush a trailing
// partial line.

type indentWriter struct {
	logger  *logger
	partial strings.Builder
}

func newLogger(w io.Writer, quiet bool) *logger {
	l := &logger{w: w, quiet: quiet}

	if colorEnabled(w) {
		l.bold = "\033[1m"
		l.dim = "\033[2m"
		l.cyan = "\033[36m"
		l.green = "\033[32m"
		l.red = "\033[31m"
		l.reset = "\033[0m"
	}

	return l
}

func colorEnabled(w io.Writer) bool {
	if os.Getenv("FORCE_COLOR") != "" {
		return true
	}

	if os.Getenv("NO_COLOR") != "" {
		return false
	}

	file, ok := w.(*os.File)

	return ok && isatty.IsTerminal(file.Fd())
}

func (l *logger) section(msg string) {
	_, _ = fmt.Fprintf(l.w, "\n%s==>%s %s%s%s\n", l.cyan, l.reset, l.bold, msg, l.reset)
}

func (l *logger) detail(label, value string) {
	_, _ = fmt.Fprintf(l.w, "    %s%-12s%s %s\n", l.dim, label, l.reset, value)
}

func (l *logger) successDetail(label, value string) {
	_, _ = fmt.Fprintf(l.w, "    %s%-12s%s %s%s%s\n", l.green, label, l.reset, l.green, value, l.reset)
}

func (l *logger) failure(msg string) {
	_, _ = fmt.Fprintf(l.w, "\n%s!!%s %s%s%s\n", l.red, l.reset, l.bold, msg, l.reset)
}

func (l *logger) stream() io.WriteCloser {
	return &indentWriter{logger: l}
}

func (w *indentWriter) Write(p []byte) (int, error) {
	for _, b := range p {
		if b != '\n' {
			w.partial.WriteByte(b)

			continue
		}

		w.flushLine()
	}

	return len(p), nil
}

func (w *indentWriter) Close() error {
	if w.partial.Len() > 0 {
		w.flushLine()
	}

	return nil
}

func (w *indentWriter) flushLine() {
	l := w.logger

	_, _ = fmt.Fprintf(l.w, "    %s%s%s\n", l.dim, w.partial.String(), l.reset)

	w.partial.Reset()
}
