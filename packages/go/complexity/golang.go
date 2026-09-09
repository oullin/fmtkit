package complexity

import (
	"go/ast"
	"go/parser"
	"go/printer"
	"go/token"
	"path/filepath"
	"strings"

	"github.com/fzipp/gocyclo"
	"github.com/uudashr/gocognit"
	formatterconfig "go.ollin.sh/fmtkit/formatter/config"
	formatterengine "go.ollin.sh/fmtkit/formatter/engine"
)

// ScanGo measures every function declaration in the Go files under paths.
//
// File discovery is the formatter's own: the same exclude / not_path /
// not_name settings and the same generated-file skipping, so the check covers
// exactly what fmtkit already claims to own. Test files are left out — a table
// of cases is not the complexity this check is about.
func ScanGo(root string, paths []string, cfg formatterconfig.Config) Scan {
	out := Scan{Lane: LaneGo}

	files, err := formatterengine.CollectGoFiles(paths, cfg)

	if err != nil {
		out.Errors = append(out.Errors, ErrorResult{File: root, Message: err.Error()})

		return out
	}

	for _, file := range files {
		if IsTestFile(file) {
			continue
		}

		relative := relativeTo(root, file)

		out.Files = append(out.Files, relative)
		out.append(scanFile(file, relative))
	}

	return out
}

// append folds one file's measurement into the lane's.
func (s *Scan) append(other Scan) {
	s.Functions = append(s.Functions, other.Functions...)
	s.Errors = append(s.Errors, other.Errors...)
}

// scanFile parses one Go file and scores its declarations. A file that does
// not parse is an error rather than a silent zero.
func scanFile(path, relative string) Scan {
	fset := token.NewFileSet()

	parsed, err := parser.ParseFile(fset, path, nil, parser.SkipObjectResolution)

	if err != nil {
		return Scan{Errors: []ErrorResult{{File: relative, Message: err.Error()}}}
	}

	var out Scan

	for _, decl := range parsed.Decls {
		fn, ok := decl.(*ast.FuncDecl)

		if !ok || fn.Body == nil {
			continue
		}

		out.Functions = append(out.Functions, Function{
			Key:        relative + "#" + FunctionName(fn),
			File:       relative,
			Line:       fset.Position(fn.Pos()).Line,
			Cyclomatic: gocyclo.Complexity(fn),
			Cognitive:  gocognit.Complexity(fn),
		})
	}

	return out
}

// IsTestFile reports whether a path is a Go test file. Tests are left out of
// the scan: a long table of cases is not the complexity this check is about,
// and the TypeScript lane leaves its own tests out for the same reason.
func IsTestFile(path string) bool {
	return strings.HasSuffix(path, "_test.go")
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

// relativeTo renders an absolute path relative to root in slash form, falling
// back to the absolute path when the two share no base.
func relativeTo(root, path string) string {
	relative, err := filepath.Rel(root, path)

	if err != nil || strings.HasPrefix(relative, "..") {
		return filepath.ToSlash(path)
	}

	return filepath.ToSlash(relative)
}
