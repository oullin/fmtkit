package spacing

import (
	"bytes"
	"cmp"
	"go/ast"
	"go/parser"
	"go/token"
	"slices"
	"strings"

	"go.ollin.sh/fmtkit/formatter/rules"
)

// embedDirectiveRepairer keeps go:embed directives immediately above the var
// declaration they annotate, reading the shared parse state through ctx.
type embedDirectiveRepairer struct {
	ctx *fileContext
}

// newEmbedDirectiveRepairer returns a repairer bound to the shared parse state.
func newEmbedDirectiveRepairer(ctx *fileContext) *embedDirectiveRepairer {
	return &embedDirectiveRepairer{ctx: ctx}
}

func attachEmbedDirectiveDocs(file *ast.File) {
	for decl, group := range embedDirectiveMatches(file) {
		genDecl, ok := decl.(*ast.GenDecl)

		if !ok || genDecl.Doc != nil {
			continue
		}

		genDecl.Doc = group
	}
}

// analyze reports every go:embed directive separated from the var declaration it
// annotates, labelling the returned violations with filename.
func (e *embedDirectiveRepairer) analyze(filename string) []rules.Violation {
	var violations []rules.Violation

	for decl, group := range embedDirectiveMatches(e.ctx.file) {
		commentEndLine := e.ctx.fset.Position(group.End()).Line
		declLine := e.ctx.fset.Position(decl.Pos()).Line

		if declLine == commentEndLine+1 {
			continue
		}

		violations = append(violations, rules.Violation{
			Rule:    "spacing",
			File:    filename,
			Line:    declLine,
			Message: "go:embed directives must remain immediately above the following var declaration",
		})
	}

	return violations
}

// repair re-parses src — which may already carry the type reorder — and moves
// every detached go:embed directive group back immediately above its var
// declaration. It parses its own file rather than reusing ctx because it operates
// on the transformed bytes.
func (e *embedDirectiveRepairer) repair(filename string, src []byte) ([]byte, error) {
	fset := token.NewFileSet()
	file, err := parser.ParseFile(fset, filename, src, parser.ParseComments)

	if err != nil {
		return nil, err
	}

	type embedMove struct {
		commentStartLine int
		commentEndLine   int
		declLine         int
	}

	var moves []embedMove

	for decl, group := range embedDirectiveMatches(file) {
		commentEndLine := fset.Position(group.End()).Line
		declLine := fset.Position(decl.Pos()).Line

		if declLine == commentEndLine+1 {
			continue
		}

		moves = append(moves, embedMove{
			commentStartLine: fset.Position(group.Pos()).Line,
			commentEndLine:   commentEndLine,
			declLine:         declLine,
		})
	}

	if len(moves) == 0 {
		return src, nil
	}

	lines := bytes.SplitAfter(src, []byte{'\n'})

	// Descending: moves are applied bottom-up so the line indices of the moves
	// still to come stay valid. Hence b before a.
	slices.SortStableFunc(moves, func(a embedMove, b embedMove) int {
		return cmp.Compare(b.commentStartLine, a.commentStartLine)
	})

	for _, move := range moves {
		groupStart := move.commentStartLine - 1
		groupEnd := move.commentEndLine
		insertAt := move.declLine - 1
		removeEnd := groupEnd

		if groupEnd < len(lines) && len(bytes.TrimSpace(lines[groupEnd])) == 0 {
			removeEnd++
		}

		groupLines := append([][]byte(nil), lines[groupStart:groupEnd]...)
		lines = append(lines[:groupStart], lines[removeEnd:]...)

		if insertAt > groupStart {
			insertAt -= removeEnd - groupStart
		}

		lines = append(lines[:insertAt], append(groupLines, lines[insertAt:]...)...)
	}

	return bytes.Join(lines, nil), nil
}

func embedDirectiveMatches(file *ast.File) map[ast.Decl]*ast.CommentGroup {
	matches := map[ast.Decl]*ast.CommentGroup{}
	docGroups := map[*ast.CommentGroup]struct{}{}
	varDecls := topLevelVarDecls(file)

	for _, decl := range varDecls {
		genDecl, ok := decl.(*ast.GenDecl)

		if !ok || genDecl.Doc == nil || !containsEmbedDirective(genDecl.Doc) {
			continue
		}

		matches[decl] = genDecl.Doc
		docGroups[genDecl.Doc] = struct{}{}
	}

	for _, group := range file.Comments {
		if !containsEmbedDirective(group) {
			continue
		}

		if _, ok := docGroups[group]; ok {
			continue
		}

		if decl, ok := nextTopLevelVarDeclAfter(varDecls, group.End()); ok {
			if _, seen := matches[decl]; !seen {
				matches[decl] = group
			}
		}
	}

	return matches
}

func topLevelVarDecls(file *ast.File) []ast.Decl {
	var decls []ast.Decl

	for _, decl := range file.Decls {
		genDecl, ok := decl.(*ast.GenDecl)

		if ok && genDecl.Tok == token.VAR {
			decls = append(decls, decl)
		}
	}

	return decls
}

func nextTopLevelVarDeclAfter(decls []ast.Decl, pos token.Pos) (ast.Decl, bool) {
	for _, decl := range decls {
		if decl.Pos() > pos {
			return decl, true
		}
	}

	return nil, false
}

// isEmbedDirectiveText reports whether text is a go:embed directive carrying at
// least one pattern. The directive grammar comes from go/ast, so a bare
// //go:embed, a longer name such as //go:embedded, and anything that is not a
// directive at all are rejected without restating those rules here.
func isEmbedDirectiveText(text string) bool {
	directive, ok := ast.ParseDirective(token.NoPos, strings.TrimSpace(text))

	return ok && directive.Tool == "go" && directive.Name == "embed" && directive.Args != ""
}

func containsEmbedDirective(group *ast.CommentGroup) bool {
	if group == nil {
		return false
	}

	return slices.ContainsFunc(group.List, func(comment *ast.Comment) bool {
		return isEmbedDirectiveText(comment.Text)
	})
}

// collapseEmbedSpacing removes a single blank line left between a go:embed
// directive and the var declaration it annotates. It works purely on bytes with
// no parse state, so it stays a free function the repairer's collapse step calls.
func collapseEmbedSpacing(src []byte) []byte {
	lines := bytes.Split(src, []byte{'\n'})
	out := make([][]byte, 0, len(lines))

	for i := 0; i < len(lines); i++ {
		out = append(out, lines[i])

		if i+2 >= len(lines) {
			continue
		}

		if !isEmbedDirectiveLine(lines[i]) {
			continue
		}

		if len(bytes.TrimSpace(lines[i+1])) != 0 {
			continue
		}

		next := bytes.TrimSpace(lines[i+2])

		if isVarDeclStart(next) {
			i++
		}
	}

	return bytes.Join(out, []byte{'\n'})
}

func isEmbedDirectiveLine(line []byte) bool {
	return isEmbedDirectiveText(string(line))
}

func isVarDeclStart(line []byte) bool {
	if !bytes.HasPrefix(line, []byte("var")) {
		return false
	}

	if len(line) == len("var") {
		return true
	}

	switch line[len("var")] {
	case ' ', '\t', '\n', '\r', '\f', '\v', '(':
		return true
	default:
		return false
	}
}
