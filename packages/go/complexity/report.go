package complexity

import (
	"cmp"
	"fmt"
	"os"
	"path/filepath"
	"slices"
)

// Function is one scored function: its key, where it starts, and both metrics.
// File is repository-relative and Key is "<File>#<name>", so the key alone
// locates the function a finding or an allow entry talks about.
type Function struct {
	Key        string `json:"key"`
	File       string `json:"file"`
	Line       int    `json:"line"`
	Cyclomatic int    `json:"cyclomatic"`
	Cognitive  int    `json:"cognitive"`
}

// Finding is one reported breach, shaped like the formatter's violations so
// the renderers treat both the same way.
type Finding struct {
	Rule    string `json:"rule"`
	File    string `json:"file"`
	Line    int    `json:"line,omitempty"`
	Key     string `json:"key,omitempty"`
	Message string `json:"message"`
}

// ErrorResult describes a file the lane could not score.
type ErrorResult struct {
	File    string `json:"file,omitempty"`
	Message string `json:"message"`
}

// Scan is one lane's measurement: every function it scored and every file it
// covered. Files carries the coverage an allow entry is judged against, which
// is why it is kept even though Functions already names the files that hold a
// function.
type Scan struct {
	Lane      Lane
	Functions []Function
	Files     []string
	Errors    []ErrorResult
}

// Report is the complexity check's outcome for one run.
type Report struct {
	// Skipped marks a run that measured nothing because no lane ran.
	Skipped bool `json:"skipped,omitempty"`

	Files     int           `json:"files"`
	Functions int           `json:"functions"`
	Findings  []Finding     `json:"findings,omitempty"`
	Errors    []ErrorResult `json:"errors,omitempty"`
}

// FindingCount returns the number of reported breaches.
func (r Report) FindingCount() int { return len(r.Findings) }

// ErrorCount returns the number of files the run failed to score.
func (r Report) ErrorCount() int { return len(r.Errors) }

// Status classifies a report as "skipped", "fail", or "pass".
func (r Report) Status() string {
	switch {
	case r.Skipped:
		return "skipped"
	case r.FindingCount() > 0 || r.ErrorCount() > 0:
		return "fail"
	default:
		return "pass"
	}
}

// Evaluate applies the policy to the lanes' measurements: one finding per
// metric a function breaches without an allow entry, plus one per allow entry
// that matched nothing the run could have matched.
//
// root is the directory the keys' relative paths resolve against; it is only
// read to tell a stale allow entry (its file is gone) from one this run simply
// did not cover.
func Evaluate(root string, cfg Config, scans []Scan) Report {
	out := Report{Skipped: len(scans) == 0}

	for _, scan := range scans {
		out.Files += len(scan.Files)
		out.Functions += len(scan.Functions)
		out.Errors = append(out.Errors, scan.Errors...)
		out.Findings = append(out.Findings, breaches(cfg, scan)...)
	}

	out.Findings = append(out.Findings, staleAllowEntries(root, cfg, scans)...)

	slices.SortStableFunc(out.Findings, compareFindings)

	return out
}

// breaches reports the limits one lane's functions exceed, skipping the ones
// the allow list exempts.
func breaches(cfg Config, scan Scan) []Finding {
	var findings []Finding

	for _, fn := range scan.Functions {
		if cfg.Allowed(fn.Key) {
			continue
		}

		findings = append(findings,
			breach(RuleCyclomatic, fn, fn.Cyclomatic, cfg.Cyclomatic)...)
		findings = append(findings,
			breach(RuleCognitive, fn, fn.Cognitive, cfg.Cognitive)...)
	}

	return findings
}

// breach is the one-or-none finding a single metric produces. A limit of zero
// or less turns the metric off.
func breach(rule string, fn Function, score, limit int) []Finding {
	if limit <= 0 || score <= limit {
		return nil
	}

	return []Finding{{
		Rule:    rule,
		File:    fn.File,
		Line:    fn.Line,
		Key:     fn.Key,
		Message: fmt.Sprintf("%s scores %d (limit %d)", fn.Key, score, limit),
	}}
}

// staleAllowEntries reports the baseline entries that no longer name a
// function. An entry is only judged by the lane that owns its extension, and
// only when this run could have matched it: either the run scored its file, or
// the file is no longer on disk.
func staleAllowEntries(root string, cfg Config, scans []Scan) []Finding {
	matched, covered, lanes := allowContext(scans)

	var findings []Finding

	for _, entry := range cfg.Allow {
		lane := LaneOf(entry.Key)

		if !slices.Contains(lanes, lane) || matched[entry.Key] {
			continue
		}

		file := KeyFile(entry.Key)

		if !covered[file] && exists(root, file) {
			continue
		}

		findings = append(findings, Finding{
			Rule:    RuleAllow,
			File:    file,
			Key:     entry.Key,
			Message: fmt.Sprintf("allow entry %q matches no function", entry.Key),
		})
	}

	return findings
}

// allowContext reduces the scans to what staleness needs: the keys that were
// scored, the files that were covered, and the lanes that ran.
func allowContext(scans []Scan) (map[string]bool, map[string]bool, []Lane) {
	matched := map[string]bool{}
	covered := map[string]bool{}

	lanes := make([]Lane, 0, len(scans))

	for _, scan := range scans {
		lanes = append(lanes, scan.Lane)

		for _, fn := range scan.Functions {
			matched[fn.Key] = true
		}

		for _, file := range scan.Files {
			covered[file] = true
		}
	}

	return matched, covered, lanes
}

// exists reports whether a key's file is still on disk under root.
func exists(root, file string) bool {
	_, err := os.Stat(filepath.Join(root, filepath.FromSlash(file)))

	return err == nil
}

func compareFindings(a, b Finding) int {
	return cmp.Or(
		cmp.Compare(a.File, b.File),
		cmp.Compare(a.Line, b.Line),
		cmp.Compare(a.Rule, b.Rule),
		cmp.Compare(a.Key, b.Key),
	)
}
