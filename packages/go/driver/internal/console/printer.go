// Package console renders the pipeline's sectioned, ANSI-colored progress
// output: section headers, aligned detail lines, failure banners, and the
// indented live stream of a child tool's output. Color detection is resolved
// once by the caller (see DetectColor) and handed to NewPrinter, so the printer
// itself never reads the environment.
package console

import (
	"fmt"
	"io"
	"os"
	"strings"

	"github.com/mattn/go-isatty"
)

// ColorMode is whether a Printer emits ANSI escape sequences.
type ColorMode int

// Printer renders progress output to a writer. The palette fields are empty
// strings when color is off, so the same format strings render plain text.
type Printer struct {
	w io.Writer

	bold  string
	dim   string
	cyan  string
	green string
	red   string
	reset string
}

type indentWriter struct {
	printer *Printer
	partial strings.Builder
}

const (
	// ColorAuto defers the decision to DetectColor. NewPrinter treats it as
	// no-color, so callers resolve it through DetectColor before constructing a
	// Printer rather than passing it through.
	ColorAuto ColorMode = iota

	// ColorAlways forces ANSI color on.
	ColorAlways

	// ColorNever forces ANSI color off.
	ColorNever
)

// DetectColor resolves whether color should be used when writing to w. It
// honors FORCE_COLOR (always on) and NO_COLOR (always off) before falling back
// to whether w is a terminal. This is the single place the environment is read;
// callers resolve it once and pass the result to NewPrinter.
func DetectColor(w io.Writer) ColorMode {
	if os.Getenv("FORCE_COLOR") != "" {
		return ColorAlways
	}

	if os.Getenv("NO_COLOR") != "" {
		return ColorNever
	}

	if file, ok := w.(*os.File); ok && isatty.IsTerminal(file.Fd()) {
		return ColorAlways
	}

	return ColorNever
}

// NewPrinter builds a Printer writing to w. ANSI color is enabled only for
// ColorAlways; ColorAuto and ColorNever both render plain text, so callers pass
// the resolved result of DetectColor.
func NewPrinter(w io.Writer, mode ColorMode) *Printer {
	p := &Printer{w: w}

	if mode == ColorAlways {
		p.bold = "\033[1m"
		p.dim = "\033[2m"
		p.cyan = "\033[36m"
		p.green = "\033[32m"
		p.red = "\033[31m"
		p.reset = "\033[0m"
	}

	return p
}

// Section prints a bold, cyan-arrowed section header preceded by a blank line.
func (p *Printer) Section(msg string) {
	_, _ = fmt.Fprintf(p.w, "\n%s==>%s %s%s%s\n", p.cyan, p.reset, p.bold, msg, p.reset)
}

// Detail prints an aligned label/value line under the current section.
func (p *Printer) Detail(label, value string) {
	_, _ = fmt.Fprintf(p.w, "    %s%-12s%s %s\n", p.dim, label, p.reset, value)
}

// SuccessDetail prints an aligned label/value line in green.
func (p *Printer) SuccessDetail(label, value string) {
	_, _ = fmt.Fprintf(p.w, "    %s%-12s%s %s%s%s\n", p.green, label, p.reset, p.green, value, p.reset)
}

// Failure prints a red, banged failure banner preceded by a blank line.
func (p *Printer) Failure(msg string) {
	_, _ = fmt.Fprintf(p.w, "\n%s!!%s %s%s%s\n", p.red, p.reset, p.bold, msg, p.reset)
}

// Stream returns a writer that renders a child tool's output live, dimmed and
// indented under the current section. Callers must Close it to flush a trailing
// partial line.
func (p *Printer) Stream() io.WriteCloser {
	return &indentWriter{printer: p}
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
	p := w.printer

	_, _ = fmt.Fprintf(p.w, "    %s%s%s\n", p.dim, w.partial.String(), p.reset)

	w.partial.Reset()
}
