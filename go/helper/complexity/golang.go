// Package complexity scores Go function declarations with gocyclo and gocognit.
package complexity

import (
	"go/ast"
	"go/parser"
	"go/printer"
	"go/token"
	"strings"

	"github.com/fzipp/gocyclo"
	"github.com/uudashr/gocognit"
)

// Function is one scored function. Key is "<rel>#<Name>", so the key alone
// locates the function a finding or an allow entry talks about.
type Function struct {
	Key        string
	Name       string
	Line       int
	Cyclomatic int
	Cognitive  int
}

// Score parses one Go file and scores every function declaration with a body.
// rel is the repository-relative path that keys the scores. A file that does
// not parse is an error rather than a silent zero.
func Score(rel string, src []byte) ([]Function, error) {
	fset := token.NewFileSet()
	parsed, err := parser.ParseFile(fset, rel, src, parser.SkipObjectResolution)

	if err != nil {
		return nil, err
	}

	return ScoreFile(rel, fset, parsed), nil
}

// ScoreFile is Score on a file already parsed, so a caller that parses it
// anyway does not parse it twice.
func ScoreFile(rel string, fset *token.FileSet, parsed *ast.File) []Function {
	var out []Function

	for _, decl := range parsed.Decls {
		fn, ok := decl.(*ast.FuncDecl)

		if !ok || fn.Body == nil {
			continue
		}

		name := FunctionName(fn)

		out = append(out, Function{
			Key:        rel + "#" + name,
			Name:       name,
			Line:       fset.Position(fn.Pos()).Line,
			Cyclomatic: gocyclo.Complexity(fn),
			Cognitive:  gocognit.Complexity(fn),
		})
	}

	return out
}

// FunctionName is a declaration's key name: the plain identifier for a
// function, and the receiver-qualified "(*Source).Read" form for a method.
func FunctionName(fn *ast.FuncDecl) string {
	receiver := receiverType(fn)

	if receiver == "" {
		return fn.Name.Name
	}

	return "(" + receiver + ")." + fn.Name.Name
}

// receiverType renders a method's receiver type as it is written in source
// ("*Source", "Source", "*Store[T]"), or "" for a plain function.
func receiverType(fn *ast.FuncDecl) string {
	if fn.Recv == nil || len(fn.Recv.List) == 0 {
		return ""
	}

	var buffer strings.Builder

	if err := printer.Fprint(&buffer, token.NewFileSet(), fn.Recv.List[0].Type); err != nil {
		return ""
	}

	return buffer.String()
}
