package report

import (
	"errors"
	"io"
	"path/filepath"

	formatterengine "go.ollin.sh/fmtkit/formatter/engine"
	"go.ollin.sh/fmtkit/vet"
)

// Combined contains the formatter and vet reports rendered by the CLI.
type Combined struct {
	Formatter formatterengine.Report `json:"formatter"`
	Vet       vet.Report             `json:"vet"`
}

type jsonErrorMessage struct {
	File    string `json:"file"`
	Message string `json:"message"`
}

// Render writes the report in the requested output format.
func Render(w io.Writer, format, cwd, mode string, report Combined) error {
	switch format {
	case "text":
		return RenderText(w, cwd, mode, report)
	case "json":
		return RenderJSON(w, cwd, report)
	case "agent":
		return RenderAgent(w, cwd, report)
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

	return report.Formatter.Result
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
