package report

type projectedReport struct {
	Result    string
	Formatter projectedFormatterReport
	Vet       projectedVetReport
}

type projectedFormatterReport struct {
	Result     string
	Files      int
	Changed    int
	Violations int
	Results    []projectedFileResult
	Errors     []jsonErrorMessage
}

type projectedVetReport struct {
	Status string
	Errors []jsonErrorMessage
}

type projectedFileResult struct {
	File       string
	Applied    []string
	Violations []jsonViolation
	Changed    bool
	Error      string
}

func projectReport(cwd string, report Combined) projectedReport {
	return projectedReport{
		Result:    combinedResult(report),
		Formatter: projectFormatterReport(cwd, report),
		Vet:       projectVetReport(cwd, report),
	}
}

func projectFormatterReport(cwd string, report Combined) projectedFormatterReport {
	out := projectedFormatterReport{
		Result:     report.Formatter.Result,
		Files:      report.Formatter.Files,
		Changed:    report.Formatter.Changed,
		Violations: report.Formatter.ViolationCount(),
	}

	for _, result := range report.Formatter.Errors {
		out.Errors = append(out.Errors, jsonErrorMessage{
			File:    relativePath(cwd, result.File),
			Message: result.Message,
		})
	}

	for _, result := range report.Formatter.Results {
		item := projectedFileResult{
			File:    relativePath(cwd, result.File),
			Applied: result.Applied,
			Changed: result.Changed,
			Error:   result.Error,
		}

		for _, violation := range result.Violations {
			item.Violations = append(item.Violations, jsonViolation{
				Rule:    violation.Rule,
				Line:    violation.Line,
				Message: violation.Message,
			})
		}

		if item.Error != "" {
			out.Errors = append(out.Errors, jsonErrorMessage{
				File:    item.File,
				Message: item.Error,
			})
		}

		out.Results = append(out.Results, item)
	}

	return out
}

func projectVetReport(cwd string, report Combined) projectedVetReport {
	out := projectedVetReport{
		Status: vetStatus(report.Vet),
	}

	for _, result := range report.Vet.Errors {
		out.Errors = append(out.Errors, jsonErrorMessage{
			File:    relativePath(cwd, result.File),
			Message: result.Message,
		})
	}

	return out
}
