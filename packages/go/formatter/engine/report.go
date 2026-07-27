package engine

import "go.ollin.sh/fmtkit/formatter/rules"

// Result classifies the outcome of an engine run.
type Result string

// FileResult describes the outcome for a single file.
type FileResult struct {
	File       string            `json:"file"`
	Applied    []string          `json:"applied,omitempty"`
	Violations []rules.Violation `json:"violations,omitempty"`
	Error      string            `json:"error,omitempty"`
	Changed    bool              `json:"changed,omitempty"`
}

// ErrorResult describes an engine error associated with a file or workspace.
type ErrorResult struct {
	File    string `json:"file,omitempty"`
	Message string `json:"message"`
}

// Report summarizes an engine run across one or more files.
type Report struct {
	Result  Result        `json:"result"`
	Files   int           `json:"files"`
	Changed int           `json:"changed"`
	Results []FileResult  `json:"results,omitempty"`
	Errors  []ErrorResult `json:"errors,omitempty"`
}

// The Result values an engine run can report. The strings are part of the
// JSON report contract, so they must stay as they are.
const (
	ResultPass  Result = "pass"
	ResultFixed Result = "fixed"
	ResultFail  Result = "fail"
)

// ViolationCount returns the total number of rule violations in the report.
func (r Report) ViolationCount() int {
	total := 0

	for _, result := range r.Results {
		total += len(result.Violations)
	}

	return total
}

// ErrorCount returns the total number of engine errors in the report.
func (r Report) ErrorCount() int {
	total := len(r.Errors)

	for _, result := range r.Results {
		if result.Error != "" {
			total++
		}
	}

	return total
}
