package report

import (
	"fmt"
	"io"
	"strings"

	"github.com/fatih/color"
	formatterengine "github.com/oullin/go-fmt/packages/formatter/engine"
)

// RenderText writes the human-readable text report representation.
func RenderText(w io.Writer, cwd, mode string, report Combined) error {
	if _, err := color.New(color.Bold).Fprintf(w, "\nFormatter\n\n"); err != nil {
		return err
	}

	if err := renderFormatterText(w, cwd, mode, report.Formatter); err != nil {
		return err
	}

	if _, err := color.New(color.Bold).Fprintf(w, "Vet\n\n"); err != nil {
		return err
	}

	return renderVetText(w, cwd, report)
}

func renderFormatterText(w io.Writer, cwd, mode string, report formatterengine.Report) error {
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

		if mode == "format" {
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

			if mode == "format" {
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
		rel := relativePath(cwd, result.File)

		if rel != "" && rel != "." {
			if _, err := color.New(color.FgCyan, color.Bold).Fprintf(w, "  %s\n", rel); err != nil {
				return err
			}
		} else {
			if _, err := color.New(color.FgCyan, color.Bold).Fprintf(w, "  workspace\n"); err != nil {
				return err
			}
		}

		if _, err := color.New(color.FgRed).Fprintf(w, "    ! %s\n\n", result.Message); err != nil {
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

func renderVetText(w io.Writer, cwd string, report Combined) error {
	switch vetStatus(report.Vet) {
	case "skipped":
		reason := "no Go module or workspace was detected"

		if report.Vet.Skipped {
			reason = "the Go toolchain is not available"
		}

		if _, err := color.New(color.FgYellow).Fprint(w, "  Skipped automatic go vet ./... because "+reason+".\n\n"); err != nil {
			return err
		}
	case "pass":
		if _, err := color.New(color.FgGreen).Fprintf(w, "  go vet ./... passed.\n\n"); err != nil {
			return err
		}
	default:
		for _, result := range report.Vet.Errors {
			rel := relativePath(cwd, result.File)

			if rel != "" && rel != "." {
				if _, err := color.New(color.FgCyan, color.Bold).Fprintf(w, "  %s\n", rel); err != nil {
					return err
				}
			} else {
				if _, err := color.New(color.FgCyan, color.Bold).Fprintf(w, "  workspace\n"); err != nil {
					return err
				}
			}

			if _, err := color.New(color.FgRed).Fprintf(w, "    ! %s\n\n", result.Message); err != nil {
				return err
			}
		}
	}

	summaryColor := color.New(color.Bold)

	switch vetStatus(report.Vet) {
	case "pass":
		summaryColor.Add(color.FgGreen)
	case "skipped":
		summaryColor.Add(color.FgYellow)
	default:
		summaryColor.Add(color.FgRed)
	}

	_, err := summaryColor.Fprintf(w, "  Result: %s. %d error(s).\n\n", vetStatus(report.Vet), report.Vet.ErrorCount())

	return err
}
