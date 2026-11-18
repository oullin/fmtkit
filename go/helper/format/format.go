// Package format runs the Go formatting pipeline on one file held in memory:
// the spacing rule, gofmt, goimports, then the naming checks, the go/* nesting
// and line-length checks, and complexity scoring of the result.
package format

import (
	"bytes"
	"fmt"
	"go/ast"
	goformat "go/format"
	"go/parser"
	"go/token"

	"go.ollin.sh/fmtkit/go/helper/complexity"
	"go.ollin.sh/fmtkit/go/helper/lint"
	"go.ollin.sh/fmtkit/go/helper/naming"
	"go.ollin.sh/fmtkit/go/helper/proto"
	"go.ollin.sh/fmtkit/go/helper/spacing"
	"golang.org/x/tools/imports"
)

// step is one formatter pass after the spacing rule.
type step struct {
	name    string
	enabled bool
	run     func([]byte) ([]byte, error)
}

// tree parses the final text once, the first time a check needs it, and
// hands the same result to every check after it.
type tree struct {
	rel    string
	src    []byte
	mode   parser.Mode
	fset   *token.FileSet
	file   *ast.File
	err    error
	parsed bool
}

// formatOnly is goimports without import resolution: it groups, sorts and
// formats the import blocks that are already there and never reads the disk.
var formatOnly = imports.Options{FormatOnly: true, Comments: true, TabIndent: true, TabWidth: 8}

// resolving is goimports with full import resolution, as the goimports CLI
// runs it. It reads the file's package directory and the module cache.
var resolving = imports.Options{Comments: true, TabIndent: true, TabWidth: 8}

// Process runs the steps req selects, in v1's order: spacing, gofmt,
// goimports, then the naming and go/* checks and complexity on the final
// text. A step that fails stops the pipeline; the reply then carries the error
// as "<step>: <cause>", the original source as its output, and whatever
// earlier steps reported.
func Process(req proto.Request) proto.Reply {
	current := req.Source
	reply := proto.Reply{}

	if req.Steps.Spacing {
		violations, formatted, err := spacing.New().Apply(req.Rel, current)

		if err != nil {
			return failed(reply, req, "spacing", err)
		}

		for _, v := range violations {
			reply.Violations = append(reply.Violations, proto.Violation{
				Rule:    v.Rule,
				Line:    wireUint(v.Line),
				Message: v.Message,
			})
		}

		if !bytes.Equal(formatted, current) {
			current = formatted
			reply.Applied = append(reply.Applied, "spacing")
		}
	}

	steps := []step{
		{name: "gofmt", enabled: req.Steps.Gofmt, run: goformat.Source},
		{name: "goimports", enabled: req.Steps.Goimports, run: goimports(req)},
	}

	for _, s := range steps {
		if !s.enabled {
			continue
		}

		formatted, err := s.run(current)

		if err != nil {
			return failed(reply, req, s.name, err)
		}

		if !bytes.Equal(formatted, current) {
			current = formatted
			reply.Applied = append(reply.Applied, s.name)
		}
	}

	final := newTree(req, current)
	lint, step, err := checks(req, final)

	if err != nil {
		return failed(reply, req, step, err)
	}

	reply.Lint = lint

	if req.Steps.Complexity {
		fset, file, err := final.parse()

		if err != nil {
			return failed(reply, req, "complexity", err)
		}

		for _, fn := range complexity.ScoreFile(req.Rel, fset, file) {
			reply.Complexity = append(reply.Complexity, proto.Score{
				Key:        fn.Key,
				Name:       fn.Name,
				Line:       wireUint(fn.Line),
				Cyclomatic: wireUint(fn.Cyclomatic),
				Cognitive:  wireUint(fn.Cognitive),
			})
		}
	}

	reply.Output = current

	return reply
}

// newTree prepares to parse src for the checks req turns on. Comments are
// kept only for the line-length check, the one check that reads them.
func newTree(req proto.Request, src []byte) *tree {
	mode := parser.SkipObjectResolution

	if req.Steps.Lint.MaxLineLength > 0 {
		mode = lint.Mode
	}

	return &tree{rel: req.Rel, src: src, mode: mode}
}

func (t *tree) parse() (*token.FileSet, *ast.File, error) {
	if !t.parsed {
		t.fset = token.NewFileSet()
		t.file, t.err = parser.ParseFile(t.fset, t.rel, t.src, t.mode)
		t.parsed = true
	}

	return t.fset, t.file, t.err
}

// checks runs the naming checks, then the go/* checks, against the final
// text. On failure it also returns the name of the step that failed: the
// first one that needed the text parsed.
func checks(req proto.Request, final *tree) ([]proto.Violation, string, error) {
	named, err := names(req, final)

	if err != nil {
		return nil, "naming", err
	}

	checked, err := lintFindings(req, final)

	if err != nil {
		return nil, "lint", err
	}

	return append(named, checked...), "", nil
}

// names runs the naming checks req turns on against the final text.
func names(req proto.Request, final *tree) ([]proto.Violation, error) {
	options := naming.Options{
		MaxWords:       int(req.Steps.MaxWords),
		MaxLocalWords:  int(req.Steps.MaxLocalWords),
		PackageStutter: req.Steps.PackageStutter,
	}

	if !options.Enabled() {
		return nil, nil
	}

	fset, file, err := final.parse()

	if err != nil {
		return nil, err
	}

	var lint []proto.Violation

	for _, f := range naming.CheckFile(fset, file, options) {
		lint = append(lint, proto.Violation{
			Rule:    f.Rule,
			Line:    wireUint(f.Line),
			Column:  wireUint(f.Column),
			Message: f.Message,
		})
	}

	return lint, nil
}

// lintFindings runs the go/* nesting and line-length checks req's limits turn
// on against the final text.
func lintFindings(req proto.Request, final *tree) ([]proto.Violation, error) {
	limits := lint.Limits{
		MaxDepth:       int(req.Steps.Lint.MaxDepth),
		MaxNestedFuncs: int(req.Steps.Lint.MaxNestedFuncs),
		MaxLineLength:  int(req.Steps.Lint.MaxLineLength),
		TabWidth:       int(req.Steps.Lint.TabWidth),
	}

	if limits.Off() {
		return nil, nil
	}

	fset, file, err := final.parse()

	if err != nil {
		return nil, err
	}

	var out []proto.Violation

	for _, f := range lint.CheckFile(fset, file, final.src, limits) {
		out = append(out, proto.Violation{
			Rule:    f.Rule,
			Line:    wireUint(f.Line),
			Column:  wireUint(f.Column),
			Message: f.Message,
		})
	}

	return out, nil
}

// goimports returns the goimports pass for req: format-only unless the request
// opts in to import resolution, which needs the file's real location.
func goimports(req proto.Request) func([]byte) ([]byte, error) {
	if !req.Steps.ResolveImports {
		return func(src []byte) ([]byte, error) {
			return imports.Process("", src, &formatOnly)
		}
	}

	filename := req.Abs

	if filename == "" {
		filename = req.Rel
	}

	return func(src []byte) ([]byte, error) {
		return imports.Process(filename, src, &resolving)
	}
}

func failed(reply proto.Reply, req proto.Request, step string, err error) proto.Reply {
	reply.Error = fmt.Sprintf("%s: %v", step, err)
	reply.Output = req.Source
	reply.Complexity = nil
	reply.Lint = nil

	return reply
}

// wireUint narrows a non-negative line number or score to the wire's u32.
func wireUint(n int) uint32 {
	if n < 0 {
		return 0
	}

	return uint32(min(n, int(^uint32(0))))
}
