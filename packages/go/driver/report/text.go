package report

import (
	"fmt"
	"io"
	"strings"

	"github.com/fatih/color"
	formatterengine "go.ollin.sh/fmtkit/formatter/engine"
)

// renderText writes the human-readable text report representation.
func (r Renderer) renderText(w io.Writer, report Combined) error {
	if _, err := color.New(color.Bold).Fprintf(w, "\nFormatter\n\n"); err != nil {
		return err
	}

	if err := renderFormatterText(w, r.Root, r.Mode, report.Formatter); err != nil {
		return err
	}

	if _, err := color.New(color.Bold).Fprintf(w, "Vet\n\n"); err != nil {
		return err
	}

	return renderVetText(w, r.Root, report)
}

func renderFormatterText(w io.Writer, cwd string, mode Mode, report formatterengine.Report) error {
	if report.Files == 0 && len(report.Errors) == 0 {
		if _, err := color.New(color.FgYellow).Fprintf(w, "  No Go files found.\n\n"); err != nil {
			return err
		}

		return nil
	}

	if report.Files == 0 {
		if _, err := color.New(color.FgYellow).Fprintf(w, "  No Go files found.\n\n"); err != nil {
			return err
		}
	} else {
		action := "Checked"

		if mode == ModeFormat {
			action = "Formatted"
		}

		if _, err := color.New(color.FgGreen, color.Bold).Fprintf(w, "  %s %d file(s).\n\n", action, report.Files); err != nil {
			return err
		}
	}

	for _, result := range report.Results {
		if len(result.Violations) == 0 && result.Error == "" && !result.Changed {
			continue
		}

		rel := relativePath(cwd, result.File)

		if _, err := color.New(color.FgCyan, color.Bold).Fprintf(w, "  %s\n", rel); err != nil {
			return err
		}

		if result.Error != "" {
			if _, err := color.New(color.FgRed).Fprintf(w, "    ! %s\n\n", result.Error); err != nil {
				return err
			}

			continue
		}

		for _, violation := range result.Violations {
			ruleColor := color.New(color.FgMagenta).Sprintf("[%s]", violation.Rule)

			if violation.Line > 0 {
				if _, err := fmt.Fprintf(w, "    %s line %d: %s\n", ruleColor, violation.Line, violation.Message); err != nil {
					return err
				}
			} else {
				if _, err := fmt.Fprintf(w, "    %s %s\n", ruleColor, violation.Message); err != nil {
					return err
				}
			}
		}

		if result.Changed {
			verb := "would apply"

			if mode == ModeFormat {
				verb = "applied"
			}

			if _, err := color.New(color.FgGreen).Fprintf(w, "    ✓ %s %s\n", verb, strings.Join(result.Applied, ", ")); err != nil {
				return err
			}
		}

		if _, err := fmt.Fprintln(w); err != nil {
			return err
		}
	}

	for _, result := range report.Errors {
		if err := renderErrorEntry(w, cwd, result.File, result.Message); err != nil {
			return err
		}
	}

	summaryColor := color.New(color.Bold)

	if report.ErrorCount() > 0 {
		summaryColor.Add(color.FgRed)
	} else if report.ViolationCount() > 0 {
		summaryColor.Add(color.FgYellow)
	} else {
		summaryColor.Add(color.FgGreen)
	}

	_, err := summaryColor.Fprintf(w, "  Result: %s. %d changed, %d violation(s), %d error(s).\n\n", report.Result, report.Changed, report.ViolationCount(), report.ErrorCount())

	return err
}

// renderErrorEntry writes one error: its path relative to cwd (or "workspace"
// when the error belongs to no single file), then the message.
//
// It takes the two fields rather than a record because the formatter and vet
// carry their own identically shaped error types. Go has no field-access
// constraint, so a type parameter here would have to be handed accessor
// closures by every caller — more code than the two strings it would abstract.
func renderErrorEntry(w io.Writer, cwd string, file string, message string) error {
	rel := relativePath(cwd, file)

	label := "workspace"

	if rel != "" && rel != "." {
		label = rel
	}

	if _, err := color.New(color.FgCyan, color.Bold).Fprintf(w, "  %s\n", label); err != nil {
		return err
	}

	_, err := color.New(color.FgRed).Fprintf(w, "    ! %s\n\n", message)

	return err
}

func renderVetText(w io.Writer, cwd string, report Combined) error {
	switch VetStatus(report.Vet) {
	case "skipped":
		if _, err := color.New(color.FgYellow).Fprint(w, "  "+VetSummary(report.Vet)+"\n\n"); err != nil {
			return err
		}
	case "pass":
		if _, err := color.New(color.FgGreen).Fprint(w, "  "+VetSummary(report.Vet)+"\n\n"); err != nil {
			return err
		}
	default:
		for _, result := range report.Vet.Errors {
			if err := renderErrorEntry(w, cwd, result.File, result.Message); err != nil {
				return err
			}
		}
	}

	summaryColor := color.New(color.Bold)

	switch VetStatus(report.Vet) {
	case "pass":
		summaryColor.Add(color.FgGreen)
	case "skipped":
		summaryColor.Add(color.FgYellow)
	default:
		summaryColor.Add(color.FgRed)
	}

	_, err := summaryColor.Fprintf(w, "  Result: %s. %d error(s).\n\n", VetStatus(report.Vet), report.Vet.ErrorCount())

	return err
}
