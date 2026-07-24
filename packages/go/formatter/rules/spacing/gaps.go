package spacing

import (
	"bytes"
	"fmt"
	"go/ast"
	"go/token"
	"slices"
	"strconv"
	"strings"

	"go.ollin.sh/fmtkit/formatter/rules"
)

// blankLineInserter records the blank lines the spacing rule must add between
// statements and top-level declarations, then applies them to the source. It
// reads the shared parse state through ctx and accumulates the byte offsets to
// split in insertions.
type blankLineInserter struct {
	ctx        *fileContext
	insertions map[int]struct{}
}

// newBlankLineInserter returns an inserter bound to the shared parse state.
func newBlankLineInserter(ctx *fileContext) *blankLineInserter {
	return &blankLineInserter{
		ctx:        ctx,
		insertions: map[int]struct{}{},
	}
}

// analyze walks the file's statement lists and top-level declarations, recording
// a violation and an insertion offset for every missing blank line. filename
// labels the returned violations.
func (b *blankLineInserter) analyze(filename string) []rules.Violation {
	fset := b.ctx.fset

	var violations []rules.Violation

	b.inspectStmtLists(func(list []ast.Stmt) {
		for i := 0; i < len(list)-1; i++ {
			current := list[i]
			next := list[i+1]
			endLine := fset.Position(current.End()).Line
			nextLine := fset.Position(next.Pos()).Line

			if currentLine, ok := b.setupSpacingLine(list, i, current, next); ok {
				violations = append(violations, rules.Violation{
					Rule:    "spacing",
					File:    filename,
					Line:    currentLine,
					Message: "missing blank line before selector call setup",
				})

				b.insertions[b.ctx.lineStartOffset(currentLine)] = struct{}{}
			}

			if endLine == nextLine {
				continue
			}

			if message, ok := b.statementGapRule(current, next); ok {
				if nextLine < endLine+2 {
					violations = append(violations, rules.Violation{
						Rule:    "spacing",
						File:    filename,
						Line:    nextLine,
						Message: message,
					})

					b.insertions[b.ctx.lineStartOffset(nextLine)] = struct{}{}
				}
			}
		}
	})

	for i := 0; i < len(b.ctx.file.Decls)-1; i++ {
		current := b.ctx.file.Decls[i]
		next := b.ctx.file.Decls[i+1]

		if !requiresTypeDeclSpacing(current, next) {
			continue
		}

		endLine := fset.Position(current.End()).Line
		nextLine := fset.Position(next.Pos()).Line

		if nextLine >= endLine+2 {
			continue
		}

		violations = append(violations, rules.Violation{
			Rule:    "spacing",
			File:    filename,
			Line:    nextLine,
			Message: "missing blank line around type definition",
		})

		b.insertions[b.ctx.lineStartOffset(nextLine)] = struct{}{}
	}

	return violations
}

// apply returns the source with the recorded blank-line insertions applied, or
// the source unchanged when the analysis recorded none.
func (b *blankLineInserter) apply() []byte {
	if len(b.insertions) == 0 {
		return b.ctx.src
	}

	return applyInsertions(b.ctx.src, b.insertions)
}

// inspectStmtLists visits every statement list in the file: block bodies, case
// clause bodies, and communication clause bodies.
func (b *blankLineInserter) inspectStmtLists(visit func([]ast.Stmt)) {
	ast.Inspect(b.ctx.file, func(node ast.Node) bool {
		switch typed := node.(type) {
		case *ast.BlockStmt:
			visit(typed.List)
		case *ast.CaseClause:
			visit(typed.Body)
		case *ast.CommClause:
			visit(typed.Body)
		}

		return true
	})
}

// setupSpacingLine reports the line that needs a leading blank line when the
// current statement assigns the receiver of a following spaced selector call and
// the two sit flush against the preceding statement.
func (b *blankLineInserter) setupSpacingLine(list []ast.Stmt, index int, current ast.Stmt, next ast.Stmt) (int, bool) {
	if index == 0 {
		return 0, false
	}

	receiverName, ok := selectorReceiverName(next, b.ctx.aliases)

	if !ok {
		return 0, false
	}

	assignedName, ok := assignedIdentifier(current)

	if !ok || assignedName != receiverName {
		return 0, false
	}

	currentLine := b.ctx.fset.Position(current.Pos()).Line
	prevEndLine := b.ctx.fset.Position(list[index-1].End()).Line

	if currentLine >= prevEndLine+2 {
		return 0, false
	}

	return currentLine, true
}

// statementGapRule reports the message for a missing blank line between current
// and next, checking the leading rule for next before the trailing rule for
// current.
func (b *blankLineInserter) statementGapRule(current ast.Stmt, next ast.Stmt) (string, bool) {
	if label, ok := b.requiresLeadingBlankLine(next); ok {
		return fmt.Sprintf("missing blank line before %s", label), true
	}

	if label, ok := b.requiresTrailingBlankLine(current, next); ok {
		return fmt.Sprintf("missing blank line after %s", label), true
	}

	return "", false
}

// requiresTrailingBlankLine reports whether current must be followed by a blank
// line, returning the label used in the violation message.
func (b *blankLineInserter) requiresTrailingBlankLine(current ast.Stmt, next ast.Stmt) (string, bool) {
	if isTestingHelperCall(current) {
		return "t.Helper call", true
	}

	if isAnonymousFuncAssignmentStmt(current, b.ctx.fset) {
		return "anonymous function assignment", true
	}

	if label, ok := stdlibSpacedCallLabel(current, b.ctx.aliases); ok {
		return label, true
	}

	switch current.(type) {
	case *ast.IfStmt, *ast.ForStmt, *ast.RangeStmt, *ast.SwitchStmt, *ast.TypeSwitchStmt, *ast.SelectStmt, *ast.DeferStmt, *ast.BranchStmt:
		return statementLabel(current, b.ctx.aliases), true
	case *ast.DeclStmt:
		if isTypeDeclStmt(current) {
			return statementLabel(current, b.ctx.aliases), true
		}

		if isVarDeclStmt(current) {
			if !isShortAssignStmt(next) && !isVarDeclStmt(next) {
				return statementLabel(current, b.ctx.aliases), true
			}
		}
	}

	return "", false
}

// requiresLeadingBlankLine reports whether stmt must be preceded by a blank
// line, returning the label used in the violation message.
func (b *blankLineInserter) requiresLeadingBlankLine(stmt ast.Stmt) (string, bool) {
	if label, ok := routeRegistryCallLabel(stmt); ok {
		return label, true
	}

	if label, ok := stdlibSpacedCallLabel(stmt, b.ctx.aliases); ok {
		return label, true
	}

	switch stmt.(type) {
	case *ast.IfStmt, *ast.ForStmt, *ast.RangeStmt, *ast.SwitchStmt, *ast.TypeSwitchStmt, *ast.SelectStmt, *ast.DeferStmt, *ast.ReturnStmt, *ast.BranchStmt:
		return statementLabel(stmt, b.ctx.aliases), true
	case *ast.DeclStmt:
		if isTypeDeclStmt(stmt) || isVarDeclStmt(stmt) {
			return statementLabel(stmt, b.ctx.aliases), true
		}
	}

	return "", false
}

func routeRegistryCallLabel(stmt ast.Stmt) (string, bool) {
	selector, ok := selectorCall(stmt)

	if !ok {
		return "", false
	}

	receiver, ok := selector.X.(*ast.Ident)

	if !ok || receiver.Name != "routes" {
		return "", false
	}

	switch selector.Sel.Name {
	case "Add", "Group":
		return "routes call", true
	default:
		return "", false
	}
}

func isTestingHelperCall(stmt ast.Stmt) bool {
	selector, ok := selectorCall(stmt)

	if !ok || selector.Sel.Name != "Helper" {
		return false
	}

	receiver, ok := selector.X.(*ast.Ident)

	return ok && receiver.Name == "t"
}

func statementLabel(stmt ast.Stmt, aliases importAliases) string {
	if label, ok := stdlibSpacedCallLabel(stmt, aliases); ok {
		return label
	}

	switch typed := stmt.(type) {
	case *ast.IfStmt:
		return "if statement"
	case *ast.ForStmt:
		return "for loop"
	case *ast.RangeStmt:
		return "range loop"
	case *ast.SwitchStmt:
		return "switch statement"
	case *ast.TypeSwitchStmt:
		return "type switch"
	case *ast.SelectStmt:
		return "select statement"
	case *ast.DeferStmt:
		return "defer statement"
	case *ast.ReturnStmt:
		return "return statement"
	case *ast.BranchStmt:
		return fmt.Sprintf("%s statement", typed.Tok)
	case *ast.DeclStmt:
		if isTypeDeclStmt(stmt) {
			return "type definition"
		}

		if isVarDeclStmt(stmt) {
			return "var declaration"
		}
	}

	return "statement"
}

func buildImportAliases(file *ast.File) importAliases {
	aliases := make(importAliases)
	stdlib := stdlibSpacingImports()

	for _, spec := range file.Imports {
		path, err := strconv.Unquote(spec.Path.Value)

		if err != nil {
			continue
		}

		defaultName, ok := stdlib[path]

		if !ok {
			continue
		}

		name := defaultName

		if spec.Name != nil {
			name = spec.Name.Name
		}

		if name == "_" || name == "." || strings.TrimSpace(name) == "" {
			continue
		}

		aliases[name] = path
	}

	return aliases
}

func stdlibSpacedCallLabel(stmt ast.Stmt, aliases importAliases) (string, bool) {
	selector, ok := selectorCall(stmt)

	if !ok {
		return "", false
	}

	pkgIdent, ok := selector.X.(*ast.Ident)

	if !ok {
		return "", false
	}

	return selectorLabel(pkgIdent.Name, selector.Sel.Name, aliases)
}

func selectorCall(stmt ast.Stmt) (*ast.SelectorExpr, bool) {
	exprStmt, ok := stmt.(*ast.ExprStmt)

	if !ok {
		return nil, false
	}

	call, ok := exprStmt.X.(*ast.CallExpr)

	if !ok {
		return nil, false
	}

	selector, ok := call.Fun.(*ast.SelectorExpr)

	if !ok {
		return nil, false
	}

	return selector, true
}

func selectorReceiverName(stmt ast.Stmt, aliases importAliases) (string, bool) {
	selector, ok := selectorCall(stmt)

	if !ok {
		return "", false
	}

	pkgIdent, ok := selector.X.(*ast.Ident)

	if !ok {
		return "", false
	}

	if _, ok := selectorLabel(pkgIdent.Name, selector.Sel.Name, aliases); !ok {
		return "", false
	}

	return pkgIdent.Name, true
}

func selectorLabel(receiverName, selectorName string, aliases importAliases) (string, bool) {
	switch aliases[receiverName] {
	case "sort":
		return "sort call", true
	case "slices":
		return "sort call", strings.HasPrefix(selectorName, "Sort")
	case "math/rand", "math/rand/v2":
		return "rand call", true
	}

	switch receiverName {
	case "sort":
		return "sort call", true
	case "slices":
		return "sort call", strings.HasPrefix(selectorName, "Sort")
	case "rand":
		return "rand call", true
	default:
		return "", false
	}
}

func assignedIdentifier(stmt ast.Stmt) (string, bool) {
	switch typed := stmt.(type) {
	case *ast.AssignStmt:
		if len(typed.Lhs) != 1 {
			return "", false
		}

		ident, ok := typed.Lhs[0].(*ast.Ident)

		if !ok {
			return "", false
		}

		return ident.Name, true
	case *ast.DeclStmt:
		genDecl, ok := typed.Decl.(*ast.GenDecl)

		if !ok || genDecl.Tok != token.VAR || len(genDecl.Specs) != 1 {
			return "", false
		}

		valueSpec, ok := genDecl.Specs[0].(*ast.ValueSpec)

		if !ok || len(valueSpec.Names) != 1 {
			return "", false
		}

		return valueSpec.Names[0].Name, true
	default:
		return "", false
	}
}

func isTypeDeclStmt(stmt ast.Stmt) bool {
	return isTokenDeclStmt(stmt, token.TYPE)
}

func isVarDeclStmt(stmt ast.Stmt) bool {
	return isTokenDeclStmt(stmt, token.VAR)
}

func isTokenDeclStmt(stmt ast.Stmt, tok token.Token) bool {
	declStmt, ok := stmt.(*ast.DeclStmt)

	if !ok {
		return false
	}

	genDecl, ok := declStmt.Decl.(*ast.GenDecl)

	return ok && genDecl.Tok == tok
}

func isShortAssignStmt(stmt ast.Stmt) bool {
	assign, ok := stmt.(*ast.AssignStmt)

	return ok && assign.Tok == token.DEFINE
}

func isAnonymousFuncAssignmentStmt(stmt ast.Stmt, fset *token.FileSet) bool {
	switch typed := stmt.(type) {
	case *ast.AssignStmt:
		return hasAnonymousFuncInitializerExpr(typed.Rhs, fset)
	case *ast.DeclStmt:
		genDecl, ok := typed.Decl.(*ast.GenDecl)

		if !ok || genDecl.Tok != token.VAR {
			return false
		}

		for _, spec := range genDecl.Specs {
			valueSpec, ok := spec.(*ast.ValueSpec)

			if !ok {
				continue
			}

			if hasAnonymousFuncInitializerExpr(valueSpec.Values, fset) {
				return true
			}
		}
	}

	return false
}

func hasAnonymousFuncInitializerExpr(exprs []ast.Expr, fset *token.FileSet) bool {
	for _, expr := range exprs {
		if !isMultiLineAnonymousFuncInitializerExpr(expr, fset) {
			continue
		}

		return true
	}

	return false
}

func isMultiLineAnonymousFuncInitializerExpr(expr ast.Expr, fset *token.FileSet) bool {
	switch typed := expr.(type) {
	case *ast.FuncLit:
		return spansMultipleLines(typed, fset)
	case *ast.CallExpr:
		if _, ok := typed.Fun.(*ast.FuncLit); ok {
			return spansMultipleLines(typed, fset)
		}
	}

	return false
}

func spansMultipleLines(node ast.Node, fset *token.FileSet) bool {
	if node == nil || fset == nil {
		return false
	}

	return fset.Position(node.Pos()).Line != fset.Position(node.End()).Line
}

func requiresTypeDeclSpacing(current ast.Decl, next ast.Decl) bool {
	return isTypeDecl(current) || isTypeDecl(next)
}

func isTypeDecl(decl ast.Decl) bool {
	genDecl, ok := decl.(*ast.GenDecl)

	return ok && genDecl.Tok == token.TYPE
}

func isImportDecl(decl ast.Decl) bool {
	genDecl, ok := decl.(*ast.GenDecl)

	return ok && genDecl.Tok == token.IMPORT
}

func buildLineStarts(src []byte) []int {
	starts := []int{0, 0}

	for i, b := range src {
		if b == '\n' {
			starts = append(starts, i+1)
		}
	}

	return starts
}

func lineStartOffset(starts []int, line int) int {
	if line < 1 {
		return 0
	}

	if line >= len(starts) {
		return starts[len(starts)-1]
	}

	return starts[line]
}

func applyInsertions(src []byte, insertions map[int]struct{}) []byte {
	offsets := make([]int, 0, len(insertions))

	for offset := range insertions {
		offsets = append(offsets, offset)
	}

	slices.Sort(offsets)

	var out bytes.Buffer
	last := 0

	for _, offset := range offsets {
		out.Write(src[last:offset])
		out.WriteByte('\n')
		last = offset
	}

	out.Write(src[last:])

	return out.Bytes()
}
