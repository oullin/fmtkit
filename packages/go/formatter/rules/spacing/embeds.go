package spacing

import (
	"bytes"
	"go/ast"
	"go/parser"
	"go/token"
	"slices"
	"strings"

	"go.ollin.sh/fmtkit/formatter/rules"
)

func attachEmbedDirectiveDocs(file *ast.File) {
	for decl, group := range embedDirectiveMatches(file) {
		genDecl, ok := decl.(*ast.GenDecl)

		if !ok || genDecl.Doc != nil {
			continue
		}

		genDecl.Doc = group
	}
}

func embedAdjacencyViolations(file *ast.File, fset *token.FileSet, filename string) []rules.Violation {
	var violations []rules.Violation

	for decl, group := range embedDirectiveMatches(file) {
		commentEndLine := fset.Position(group.End()).Line
		declLine := fset.Position(decl.Pos()).Line

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

func repairDetachedEmbedDirectives(filename string, src []byte) ([]byte, error) {
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

	slices.SortStableFunc(moves, func(a embedMove, b embedMove) int {
		switch {
		case a.commentStartLine > b.commentStartLine:
			return -1
		case a.commentStartLine < b.commentStartLine:
			return 1
		default:
			return 0
		}
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

func isEmbedDirectiveText(text string) bool {
	return hasEmbedDirectivePrefix(strings.TrimSpace(text))
}

func containsEmbedDirective(group *ast.CommentGroup) bool {
	if group == nil {
		return false
	}

	for _, comment := range group.List {
		if isEmbedDirectiveText(comment.Text) {
			return true
		}
	}

	return false
}

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
	return hasEmbedDirectiveLinePrefix(bytes.TrimSpace(line))
}

func hasEmbedDirectivePrefix(text string) bool {
	const prefix = "//go:embed"

	if !strings.HasPrefix(text, prefix) || len(text) == len(prefix) {
		return false
	}

	switch text[len(prefix)] {
	case ' ', '\t':
		return true
	default:
		return false
	}
}

func hasEmbedDirectiveLinePrefix(line []byte) bool {
	const prefix = "//go:embed"

	if !bytes.HasPrefix(line, []byte(prefix)) || len(line) == len(prefix) {
		return false
	}

	switch line[len(prefix)] {
	case ' ', '\t':
		return true
	default:
		return false
	}
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
