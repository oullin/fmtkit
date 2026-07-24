package report

import (
	"encoding/json"
	"io"
)

type agentReport struct {
	Result    string               `json:"result"`
	Formatter formatterAgentReport `json:"formatter"`
	Vet       vetAgentReport       `json:"vet"`
}

type formatterAgentReport struct {
	Result     string             `json:"result"`
	Summary    agentSummary       `json:"summary"`
	Changed    []agentChange      `json:"changed,omitempty"`
	Violations []agentViolation   `json:"violations,omitempty"`
	Errors     []jsonErrorMessage `json:"errors,omitempty"`
}

type vetAgentReport struct {
	Status string             `json:"status"`
	Errors []jsonErrorMessage `json:"errors,omitempty"`
}

type agentSummary struct {
	Files      int `json:"files"`
	Changed    int `json:"changed"`
	Violations int `json:"violations"`
}

type agentChange struct {
	File  string   `json:"file"`
	Steps []string `json:"steps"`
}

type agentViolation struct {
	File    string `json:"file"`
	Rule    string `json:"rule"`
	Line    int    `json:"line,omitempty"`
	Message string `json:"message"`
}

// renderAgent writes the agent-oriented JSON report representation.
func (r Renderer) renderAgent(w io.Writer, report Combined) error {
	encoder := json.NewEncoder(w)
	encoder.SetIndent("", "  ")

	return encoder.Encode(toAgentReport(projectReport(r.Root, report)))
}

func toAgentReport(report projectedReport) agentReport {
	return agentReport{
		Result:    report.Result,
		Formatter: toFormatterAgentReport(report.Formatter),
		Vet:       toVetAgentReport(report.Vet),
	}
}

func toFormatterAgentReport(report projectedFormatterReport) formatterAgentReport {
	out := formatterAgentReport{
		Result: report.Result,
		Summary: agentSummary{
			Files:      report.Files,
			Changed:    report.Changed,
			Violations: report.Violations,
		},
	}

	out.Errors = append(out.Errors, report.Errors...)

	for _, result := range report.Results {
		if result.Changed {
			out.Changed = append(out.Changed, agentChange{
				File:  result.File,
				Steps: result.Applied,
			})
		}

		for _, violation := range result.Violations {
			out.Violations = append(out.Violations, agentViolation{
				File:    result.File,
				Rule:    violation.Rule,
				Line:    violation.Line,
				Message: violation.Message,
			})
		}
	}

	return out
}

func toVetAgentReport(report projectedVetReport) vetAgentReport {
	return vetAgentReport(report)
}
