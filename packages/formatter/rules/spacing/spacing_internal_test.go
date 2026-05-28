package spacing

import (
	"go/ast"
	"go/parser"
	"go/token"
	"testing"
)

func TestStatementLabelControlStatements(t *testing.T) {
	tests := []struct {
		name string
		stmt ast.Stmt
		want string
	}{
		{name: "for", stmt: firstStmt(t, "for i := 0; i < 1; i++ {}"), want: "for loop"},
		{name: "range", stmt: firstStmt(t, "for _, value := range values { _ = value }"), want: "range loop"},
		{name: "switch", stmt: firstStmt(t, "switch value { case 1: }"), want: "switch statement"},
		{name: "type switch", stmt: firstStmt(t, "switch value.(type) { case int: }"), want: "type switch"},
		{name: "select", stmt: firstStmt(t, "select {}"), want: "select statement"},
		{name: "return", stmt: firstStmt(t, "return"), want: "return statement"},
		{name: "continue", stmt: &ast.BranchStmt{Tok: token.CONTINUE}, want: "continue statement"},
		{name: "fallback", stmt: &ast.EmptyStmt{}, want: "statement"},
	}

	for _, tt := range tests {
		tt := tt

		t.Run(tt.name, func(t *testing.T) {
			if got := statementLabel(tt.stmt, nil); got != tt.want {
				t.Fatalf("expected %q, got %q", tt.want, got)
			}
		})
	}
}

func TestAssignedIdentifierBranches(t *testing.T) {
	tests := []struct {
		name string
		stmt ast.Stmt
		want string
		ok   bool
	}{
		{name: "single assignment", stmt: firstStmt(t, "value := 1"), want: "value", ok: true},
		{name: "multiple assignment", stmt: firstStmt(t, "a, b := 1, 2"), ok: false},
		{name: "selector assignment", stmt: firstStmt(t, "values[0] = 1"), ok: false},
		{name: "single var", stmt: firstStmt(t, "var value = 1"), want: "value", ok: true},
		{name: "multiple var names", stmt: firstStmt(t, "var a, b = 1, 2"), ok: false},
		{name: "const declaration", stmt: firstStmt(t, "const value = 1"), ok: false},
		{name: "other statement", stmt: firstStmt(t, "return"), ok: false},
	}

	for _, tt := range tests {
		tt := tt

		t.Run(tt.name, func(t *testing.T) {
			got, ok := assignedIdentifier(tt.stmt)

			if ok != tt.ok || got != tt.want {
				t.Fatalf("expected (%q, %v), got (%q, %v)", tt.want, tt.ok, got, ok)
			}
		})
	}
}

func TestSpansMultipleLinesNilInputs(t *testing.T) {
	if spansMultipleLines(nil, token.NewFileSet()) {
		t.Fatal("expected nil node to be single-line")
	}

	if spansMultipleLines(firstStmt(t, "return"), nil) {
		t.Fatal("expected nil fileset to be single-line")
	}
}

func firstStmt(t *testing.T, stmt string) ast.Stmt {
	t.Helper()

	file, err := parser.ParseFile(token.NewFileSet(), "sample.go", "package sample\n\nfunc run(values []int, value any) {\n"+stmt+"\n}\n", 0)

	if err != nil {
		t.Fatalf("parse %q: %v", stmt, err)
	}

	return file.Decls[0].(*ast.FuncDecl).Body.List[0]
}
