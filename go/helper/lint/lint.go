// Package lint runs fmtkit's report-only Go checks: block nesting depth,
// nested function literals, and line length. They mirror the TypeScript lane's
// bundled max-depth, max-nested-callbacks and line-width policy, and never
// change the text.
package lint

import (
	"go/ast"
	"go/parser"
	"go/token"
	"sort"
)

// Limits holds each check's maximum; 0 turns that check off.
type Limits struct {
	MaxDepth       int
	MaxNestedFuncs int
	MaxLineLength  int
	// TabWidth is how many columns a tab advances to; values below 1 count 4.
	TabWidth int
}

// Finding is one breach. Line and Column are 1-based; Column counts bytes, as
// go/token does.
type Finding struct {
	Rule    string
	Line    int
	Column  int
	Message string
}

// The rule names, as reports show them.
const (
	RuleMaxDepth       = "go/max-depth"
	RuleMaxNestedFuncs = "go/max-nested-funcs"
	RuleLineLength     = "go/line-length"
)

// Off reports whether every check is turned off.
func (l Limits) Off() bool {
	return l.MaxDepth <= 0 && l.MaxNestedFuncs <= 0 && l.MaxLineLength <= 0
}

// Mode is the parser mode CheckFile needs: line length reads the comments.
const Mode = parser.SkipObjectResolution | parser.ParseComments

// Check parses one Go file and runs every check whose limit is set. A file
// that does not parse is an error. Findings are sorted by line, then column.
func Check(rel string, src []byte, limits Limits) ([]Finding, error) {
	if limits.Off() {
		return nil, nil
	}

	fset := token.NewFileSet()
	file, err := parser.ParseFile(fset, rel, src, Mode)

	if err != nil {
		return nil, err
	}

	return CheckFile(fset, file, src, limits), nil
}

// CheckFile is Check on a file already parsed from src with Mode, so a caller
// that parses it anyway does not parse it twice. One walk feeds every check.
func CheckFile(fset *token.FileSet, file *ast.File, src []byte, limits Limits) []Finding {
	if limits.Off() {
		return nil
	}

	w := &walk{fset: fset, limits: limits}

	ast.Walk(&walker{walk: w}, file)

	out := w.out

	if limits.MaxLineLength > 0 {
		out = append(out, lineLength(fset, file, src, limits, w.strings)...)
	}

	sort.SliceStable(out, func(i, j int) bool {
		if out[i].Line != out[j].Line {
			return out[i].Line < out[j].Line
		}

		return out[i].Column < out[j].Column
	})

	return out
}
