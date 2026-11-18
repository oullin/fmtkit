package lint

import (
	"fmt"
	"go/ast"
	"go/token"
)

// walk holds what one pass over a file collects for every check.
type walk struct {
	fset    *token.FileSet
	limits  Limits
	out     []Finding
	strings []*ast.BasicLit
}

// walker visits a file once for both nesting checks, and collects its string
// literals for the line-length check on the way.
//
// depth counts the way ESLint's max-depth does: each if, for, range, switch,
// type switch and select opens one level, an else if stays on the level of
// the if it continues, and case and comm clauses belong to their switch or
// select. Every function, declared or literal, starts at zero.
//
// funcs counts function literals passed as call arguments, the way ESLint's
// max-nested-callbacks counts callbacks. A literal that is not an argument,
// such as `defer func() { ... }()`, neither counts nor resets.
//
// A walker is shared by every node on its levels; only a change of level
// makes a new one.
type walker struct {
	*walk

	depth, funcs int
}

func (w *walker) Visit(node ast.Node) ast.Visitor {
	switch n := node.(type) {
	case *ast.BasicLit:
		if n.Kind == token.STRING {
			w.strings = append(w.strings, n)
		}

		return nil
	case *ast.FuncDecl, *ast.FuncLit:
		return w.at(0, w.funcs)
	case *ast.IfStmt:
		w.enter(n).walkIf(n)

		return nil
	case *ast.ForStmt, *ast.RangeStmt, *ast.SwitchStmt, *ast.TypeSwitchStmt, *ast.SelectStmt:
		return w.enter(n)
	case *ast.CallExpr:
		w.call(n)

		return nil
	}

	return w
}

// at returns the walker for the given levels: w itself when they are w's.
func (w *walker) at(depth, funcs int) *walker {
	if depth == w.depth && funcs == w.funcs {
		return w
	}

	return &walker{walk: w.walk, depth: depth, funcs: funcs}
}

// enter opens one level for stmt, reporting it when that level is too deep,
// and returns the walker for what stmt contains.
func (w *walker) enter(stmt ast.Node) *walker {
	inner := w.at(w.depth+1, w.funcs)

	if limit := w.limits.MaxDepth; limit > 0 && inner.depth > limit {
		w.report(RuleMaxDepth, stmt.Pos(), "Blocks are nested too deeply (%d). Maximum allowed is %d.",
			inner.depth, limit)
	}

	return inner
}

// walkIf walks an if statement's parts on w's level, and an else if chained to
// it on the same level without counting or reporting it again.
func (w *walker) walkIf(stmt *ast.IfStmt) {
	if stmt.Init != nil {
		ast.Walk(w, stmt.Init)
	}

	ast.Walk(w, stmt.Cond)
	ast.Walk(w, stmt.Body)

	if next, ok := stmt.Else.(*ast.IfStmt); ok {
		w.walkIf(next)

		return
	}

	if stmt.Else != nil {
		ast.Walk(w, stmt.Else)
	}
}

// call walks a call, counting each function literal among its arguments one
// level deeper.
func (w *walker) call(call *ast.CallExpr) {
	ast.Walk(w, call.Fun)

	for _, arg := range call.Args {
		lit, ok := ast.Unparen(arg).(*ast.FuncLit)

		if !ok {
			ast.Walk(w, arg)

			continue
		}

		inner := w.at(w.depth, w.funcs+1)

		if limit := w.limits.MaxNestedFuncs; limit > 0 && inner.funcs > limit {
			w.report(RuleMaxNestedFuncs, lit.Pos(), "Too many nested function literals (%d). Maximum allowed is %d.",
				inner.funcs, limit)
		}

		ast.Walk(inner, arg)
	}
}

func (w *walker) report(rule string, pos token.Pos, format string, level, limit int) {
	at := w.fset.Position(pos)

	w.out = append(w.out, Finding{
		Rule:    rule,
		Line:    at.Line,
		Column:  at.Column,
		Message: fmt.Sprintf(format, level, limit),
	})
}
