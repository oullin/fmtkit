// Package format runs the Go formatting pipeline on one file held in memory:
// the spacing rule, gofmt, goimports, and complexity scoring of the result.
package format

import (
	"bytes"
	"fmt"
	goformat "go/format"

	"go.ollin.sh/fmtkit/go/helper/complexity"
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

// formatOnly is goimports without import resolution: it groups, sorts and
// formats the import blocks that are already there and never reads the disk.
var formatOnly = imports.Options{FormatOnly: true, Comments: true, TabIndent: true, TabWidth: 8}

// resolving is goimports with full import resolution, as the goimports CLI
// runs it. It reads the file's package directory and the module cache.
var resolving = imports.Options{Comments: true, TabIndent: true, TabWidth: 8}

// Process runs the steps req selects, in v1's order: spacing, gofmt,
// goimports, then complexity on the final text. A step that fails stops the
// pipeline; the reply then carries the error as "<step>: <cause>", the
// original source as its output, and whatever earlier steps reported.
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

	if req.Steps.Complexity {
		functions, err := complexity.Score(req.Rel, current)

		if err != nil {
			return failed(reply, req, "complexity", err)
		}

		for _, fn := range functions {
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

	return reply
}

// wireUint narrows a non-negative line number or score to the wire's u32.
func wireUint(n int) uint32 {
	if n < 0 {
		return 0
	}

	return uint32(min(n, int(^uint32(0))))
}
