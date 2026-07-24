package report

import (
	"errors"
	"io"
	"path/filepath"

	formatterengine "go.ollin.sh/fmtkit/formatter/engine"
	"go.ollin.sh/fmtkit/vet"
)

// Mode is whether the CLI is checking or rewriting files. It drives the verbs
// in the text render ("Checked"/"would apply" vs "Formatted"/"applied") and the
// exit-code policy (see Combined.ExitCode).
type Mode string

// Format is the output representation the CLI renders.
type Format string

// Combined contains the formatter and vet reports rendered by the CLI.
type Combined struct {
	Formatter formatterengine.Report `json:"formatter"`
	Vet       vet.Report             `json:"vet"`
}

// Renderer writes a Combined report. Root is the base that file paths are made
// relative to; Mode selects the check/format verbs in the text render.
type Renderer struct {
	Root string
	Mode Mode
}

type jsonErrorMessage struct {
	File    string `json:"file"`
	Message string `json:"message"`
}

const (
	// ModeCheck reports what would change without touching files.
	ModeCheck Mode = "check"

	// ModeFormat rewrites files in place.
	ModeFormat Mode = "format"
)

const (
	// FormatText is the human-readable, sectioned report.
	FormatText Format = "text"

	// FormatJSON is the compact single-line JSON report.
	FormatJSON Format = "json"

	// FormatAgent is the indented, agent-oriented JSON report.
	FormatAgent Format = "agent"
)

// ParseFormat resolves a --format flag value to a Format. Unknown values are
// rejected with the same error the CLI has always returned for them.
func ParseFormat(s string) (Format, error) {
	switch Format(s) {
	case FormatText, FormatJSON, FormatAgent:
		return Format(s), nil
	default:
		return "", errors.New("unsupported output format")
	}
}

// ExitCode maps a combined report onto a process exit code for the given mode.
// Vet errors always fail. In check mode any non-pass formatter result fails; in
// format mode only formatter errors (not fixable violations) fail.
func (c Combined) ExitCode(m Mode) int {
	if c.Vet.ErrorCount() > 0 {
		return 1
	}

	if m == ModeCheck {
		if c.Formatter.Result == formatterengine.ResultPass {
			return 0
		}

		return 1
	}

	if c.Formatter.ErrorCount() > 0 {
		return 1
	}

	return 0
}

// Render writes the report in the requested output format.
func (r Renderer) Render(w io.Writer, format Format, report Combined) error {
	switch format {
	case FormatText:
		return r.renderText(w, report)
	case FormatJSON:
		return r.renderJSON(w, report)
	case FormatAgent:
		return r.renderAgent(w, report)
	default:
		return errors.New("unsupported output format")
	}
}

func relativePath(root, path string) string {
	rel, err := filepath.Rel(root, path)

	if err != nil {
		return path
	}

	return rel
}

func combinedResult(report Combined) string {
	if report.Vet.ErrorCount() > 0 || report.Formatter.ErrorCount() > 0 {
		return "fail"
	}

	return string(report.Formatter.Result)
}

func vetStatus(report vet.Report) string {
	switch {
	case report.Skipped || report.Root == "":
		return "skipped"
	case report.ErrorCount() > 0:
		return "fail"
	default:
		return "pass"
	}
}
