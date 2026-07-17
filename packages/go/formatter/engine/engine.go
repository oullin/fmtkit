package engine

import (
	"bytes"
	"fmt"
	"os"
	"runtime"
	"sync"

	"go.ollin.sh/fmtkit/formatter/config"
	"go.ollin.sh/fmtkit/formatter/rules"
)

// Formatter applies a single formatting pass to Go source.
type Formatter interface {
	Name() string
	Format(src []byte) ([]byte, error)
}

// Engine runs rules and formatter passes across a set of Go files.
type Engine struct {
	cfg        config.Config
	rules      []rules.Rule
	formatters []Formatter
}

// New builds an Engine with the supplied rule and formatter pipeline.
func New(cfg config.Config, rr []rules.Rule, ff []Formatter) *Engine {
	return &Engine{
		cfg:        cfg,
		rules:      rr,
		formatters: ff,
	}
}

// Check reports formatting changes without writing them to disk.
func (e *Engine) Check(paths []string) (Report, error) {
	files, err := CollectGoFiles(paths, e.cfg)

	if err != nil {
		return Report{}, err
	}

	return e.run(files, false)
}

// Format applies formatting changes and writes them to disk.
func (e *Engine) Format(paths []string) (Report, error) {
	files, err := CollectGoFiles(paths, e.cfg)

	if err != nil {
		return Report{}, err
	}

	return e.run(files, true)
}

// CheckFiles reports formatting changes for an explicit list of files.
func (e *Engine) CheckFiles(files []string) (Report, error) {
	return e.run(files, false)
}

// FormatFiles applies formatting changes to an explicit list of files.
func (e *Engine) FormatFiles(files []string) (Report, error) {
	return e.run(files, true)
}

func (e *Engine) run(files []string, write bool) (Report, error) {
	report := Report{
		Result: "pass",
		Files:  len(files),
	}

	if len(files) == 0 {
		return report, nil
	}

	workers := effectiveConcurrency(e.cfg.Concurrency, len(files))
	results := make([]FileResult, len(files))

	if workers <= 1 {
		for i, file := range files {
			results[i] = e.processFile(file, write)
		}
	} else {
		sem := make(chan struct{}, workers)

		var wg sync.WaitGroup

		for i, file := range files {
			wg.Add(1)
			sem <- struct{}{}

			go func(i int, file string) {
				defer wg.Done()

				defer func() { <-sem }()

				results[i] = e.processFile(file, write)
			}(i, file)
		}

		wg.Wait()
	}

	report.Results = results

	for _, r := range results {
		if r.Changed {
			report.Changed++
		}
	}

	switch {
	case report.ErrorCount() > 0:
		report.Result = "fail"
	case write && report.Changed > 0:
		report.Result = "fixed"
	case !write && report.Changed > 0:
		report.Result = "fail"
	}

	return report, nil
}

// effectiveConcurrency resolves the worker count for a run.
// Zero/negative configured values default to runtime.NumCPU(); the result
// is clamped to at most len(files) so small runs stay goroutine-free.
func effectiveConcurrency(configured, fileCount int) int {
	n := configured

	if n <= 0 {
		n = runtime.NumCPU()
	}

	if n > fileCount {
		n = fileCount
	}

	if n < 1 {
		n = 1
	}

	return n
}

func (e *Engine) processFile(path string, write bool) FileResult {
	original, err := os.ReadFile(path)

	if err != nil {
		return FileResult{File: path, Error: fmt.Sprintf("read file: %v", err)}
	}

	current := append([]byte(nil), original...)
	result := FileResult{File: path}

	for _, rule := range e.rules {
		violations, formatted, err := rule.Apply(path, current)

		if err != nil {
			result.Error = fmt.Sprintf("%s: %v", rule.Name(), err)

			return result
		}

		result.Violations = append(result.Violations, violations...)

		if !bytes.Equal(formatted, current) {
			current = formatted
			result.Applied = append(result.Applied, rule.Name())
		}
	}

	for _, f := range e.formatters {
		formatted, err := f.Format(current)

		if err != nil {
			result.Error = fmt.Sprintf("%s: %v", f.Name(), err)

			return result
		}

		if !bytes.Equal(formatted, current) {
			current = formatted
			result.Applied = append(result.Applied, f.Name())
		}
	}

	if bytes.Equal(original, current) {
		return result
	}

	result.Changed = true
	result.Diff = generateDiff(string(original), string(current))

	if write {
		if err := os.WriteFile(path, current, 0o644); err != nil {
			result.Error = fmt.Sprintf("write file: %v", err)
			result.Changed = false

			return result
		}
	}

	return result
}
