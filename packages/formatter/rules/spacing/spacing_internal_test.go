package spacing

import (
	"go/ast"
	"go/parser"
	"go/token"
	"strings"
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

func TestRouteRegistryCallLabelBranches(t *testing.T) {
	tests := []struct {
		name string
		stmt ast.Stmt
		want string
		ok   bool
	}{
		{name: "routes add", stmt: firstStmt(t, `routes.Add("home")`), want: "routes call", ok: true},
		{name: "routes group", stmt: firstStmt(t, `routes.Group("home")`), want: "routes call", ok: true},
		{name: "other routes method", stmt: firstStmt(t, `routes.Delete("home")`), ok: false},
		{name: "other receiver", stmt: firstStmt(t, `router.Add("home")`), ok: false},
		{name: "not selector", stmt: firstStmt(t, `println("home")`), ok: false},
		{name: "selector expression receiver", stmt: firstStmt(t, `app.routes.Add("home")`), ok: false},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, ok := routeRegistryCallLabel(tt.stmt)

			if ok != tt.ok || got != tt.want {
				t.Fatalf("expected (%q, %v), got (%q, %v)", tt.want, tt.ok, got, ok)
			}
		})
	}
}

func TestSelectorReceiverNameBranches(t *testing.T) {
	aliases := importAliases{
		"ordered": "slices",
		"random":  "math/rand/v2",
	}

	tests := []struct {
		name string
		stmt ast.Stmt
		want string
		ok   bool
	}{
		{name: "stdlib receiver", stmt: firstStmt(t, `sort.Strings(values)`), want: "sort", ok: true},
		{name: "aliased slices sort", stmt: firstStmt(t, `ordered.Sort(values)`), want: "ordered", ok: true},
		{name: "aliased rand", stmt: firstStmt(t, `random.Int()`), want: "random", ok: true},
		{name: "unknown selector", stmt: firstStmt(t, `logger.Info()`), ok: false},
		{name: "selector expression receiver", stmt: firstStmt(t, `pkg.sort.Strings(values)`), ok: false},
		{name: "not selector call", stmt: firstStmt(t, `println("home")`), ok: false},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, ok := selectorReceiverName(tt.stmt, aliases)

			if ok != tt.ok || got != tt.want {
				t.Fatalf("expected (%q, %v), got (%q, %v)", tt.want, tt.ok, got, ok)
			}
		})
	}
}

func TestSelectorCallBranches(t *testing.T) {
	if _, ok := selectorCall(&ast.ReturnStmt{}); ok {
		t.Fatal("expected non-expression statement to be rejected")
	}

	if _, ok := selectorCall(&ast.ExprStmt{X: &ast.SelectorExpr{}}); ok {
		t.Fatal("expected selector expression without call to be rejected")
	}

	if _, ok := selectorCall(&ast.ExprStmt{X: &ast.CallExpr{Fun: &ast.Ident{Name: "println"}}}); ok {
		t.Fatal("expected non-selector call to be rejected")
	}
}

func TestBuildImportAliasesSkipsUnsupportedNames(t *testing.T) {
	file, err := parser.ParseFile(token.NewFileSet(), "sample.go", `package sample

import (
	. "sort"
	_ "slices"
	random "math/rand/v2"
	"fmt"
)
`, parser.ParseComments)

	if err != nil {
		t.Fatalf("parse: %v", err)
	}

	aliases := buildImportAliases(file)

	if len(aliases) != 1 || aliases["random"] != "math/rand/v2" {
		t.Fatalf("unexpected aliases: %#v", aliases)
	}
}

func TestContainsEmbedDirectiveNilAndMultipleComments(t *testing.T) {
	if containsEmbedDirective(nil) {
		t.Fatal("expected nil group to not contain embed directive")
	}

	group := &ast.CommentGroup{
		List: []*ast.Comment{
			{Text: "// ordinary comment"},
			{Text: "//go:embed fixtures/*.txt"},
		},
	}

	if !containsEmbedDirective(group) {
		t.Fatal("expected embed directive to be detected")
	}

	withoutEmbed := &ast.CommentGroup{
		List: []*ast.Comment{
			{Text: "// ordinary comment"},
			{Text: "//go:generate echo ok"},
		},
	}

	if containsEmbedDirective(withoutEmbed) {
		t.Fatal("expected ordinary comments to not contain embed directive")
	}
}

func TestEmbedDirectiveLinePrefixEdges(t *testing.T) {
	tests := []struct {
		line string
		want bool
	}{
		{line: "//go:embed fixtures/*.txt", want: true},
		{line: "//go:embed\tfixtures/*.txt", want: true},
		{line: "//go:embed", want: false},
		{line: "//go:embedded fixtures/*.txt", want: false},
		{line: "//go:embed-fixtures", want: false},
	}

	for _, tt := range tests {
		t.Run(tt.line, func(t *testing.T) {
			if got := hasEmbedDirectiveLinePrefix([]byte(tt.line)); got != tt.want {
				t.Fatalf("expected %v, got %v", tt.want, got)
			}
		})
	}
}

func TestDeclOrdersEqualBranches(t *testing.T) {
	orderedFile, err := parser.ParseFile(token.NewFileSet(), "sample.go", `package sample

type config struct{}

func run() {}
`, parser.ParseComments)

	if err != nil {
		t.Fatalf("parse ordered source: %v", err)
	}

	if !declOrdersEqual(orderedFile.Decls, orderedFile.Decls) {
		t.Fatal("expected identical declaration order to match")
	}

	reversed := []ast.Decl{orderedFile.Decls[1], orderedFile.Decls[0]}

	if declOrdersEqual(orderedFile.Decls, reversed) {
		t.Fatal("expected reordered declarations to differ")
	}

	if declOrdersEqual(orderedFile.Decls, orderedFile.Decls[:1]) {
		t.Fatal("expected declaration slices with different lengths to differ")
	}
}

func TestNextTopLevelVarDeclAfterMissing(t *testing.T) {
	file, err := parser.ParseFile(token.NewFileSet(), "sample.go", `package sample

var one = 1
var two = 2
`, parser.ParseComments)

	if err != nil {
		t.Fatalf("parse: %v", err)
	}

	decls := topLevelVarDecls(file)

	if len(decls) != 2 {
		t.Fatalf("expected 2 var declarations, got %d", len(decls))
	}

	if _, ok := nextTopLevelVarDeclAfter(decls, decls[1].Pos()); ok {
		t.Fatal("expected no var declaration after the final var")
	}
}

func TestIsVarDeclStartEdges(t *testing.T) {
	tests := []struct {
		line string
		want bool
	}{
		{line: "var", want: true},
		{line: "var\troot embed.FS", want: true},
		{line: "var\n", want: true},
		{line: "var\r", want: true},
		{line: "var\f", want: true},
		{line: "var\v", want: true},
		{line: "varName := true", want: false},
		{line: "variant := true", want: false},
	}

	for _, tt := range tests {
		t.Run(strings.ReplaceAll(tt.line, "\n", "\\n"), func(t *testing.T) {
			if got := isVarDeclStart([]byte(tt.line)); got != tt.want {
				t.Fatalf("expected %v, got %v", tt.want, got)
			}
		})
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
