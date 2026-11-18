// Package naming checks the names a Go file declares, as fmtkit's
// naming/max-name-words and naming/no-package-stutter rules.
//
// naming/max-name-words reports a declared name with more words than its
// limit. Package-level functions, types, variables and constants, methods,
// struct fields and interface methods share one limit; local variables,
// constants and types, parameters, results and receivers have their own, since
// they live in a short scope. A long name usually carries context that Go
// already gives it at the call site: the package name (archive.Pack, not
// release.PackDirectory) or the receiver type ((*Archive).Limit, not
// MaxArchiveBytes).
//
// Some words are Go idiom rather than description and are not counted:
//
//   - a leading New or Must on a function (a constructor): NewServer;
//   - a leading Is or Has on a predicate, a function or method returning one
//     bool, or a variable, field or parameter that is a bool: IsReady;
//   - the Err prefix of a package-level variable (a sentinel): ErrNotFound;
//   - the Err suffix of a local variable or parameter: closeErr.
//
// Initialisms (ID, URL, HTTP, ...) are one word, plural or not.
//
// Names the author does not choose freely are not judged: _, the init and
// main functions, Test, Benchmark, Fuzz and Example functions and methods,
// methods named for a standard library interface (see [standard]), imports,
// type parameters, labels and embedded fields.
//
// naming/no-package-stutter reports an exported package-level name that
// starts with its package's name: WorkerSpec in package worker reads
// worker.WorkerSpec at the call site, and worker.Spec says the same. A name
// that is only the package name, as time.Time, is kept. Words are compared
// without case, and the package name may span several words: HTTPUtilProxy in
// package httputil stutters too. Package main and external test packages are
// not judged.
package naming

import (
	"fmt"
	"go/ast"
	"go/parser"
	"go/token"
	"strings"
	"unicode"
	"unicode/utf8"
)

// Options selects the checks. Both are off at their zero value.
type Options struct {
	// MaxWords is the word limit for package-level names, methods, struct
	// fields and interface methods. 0 turns naming/max-name-words off.
	MaxWords int
	// MaxLocalWords is the limit for locals, parameters, results and
	// receivers. 0 means MaxWords.
	MaxLocalWords int
	// PackageStutter turns naming/no-package-stutter on.
	PackageStutter bool
}

// Finding is one reported name. Line and Column are 1-based.
type Finding struct {
	Rule    string
	Line    int
	Column  int
	Message string
}

// kind is what a declared name is, which picks its limit and its message.
type kind int

// name is one declared identifier with the context its idioms need.
type name struct {
	ident *ast.Ident
	kind  kind
	// owner is the receiver, struct or interface type the name belongs to,
	// when it has one.
	owner     string
	function  bool
	predicate bool
	variable  bool
}

type checker struct {
	fset     *token.FileSet
	pkg      string
	options  Options
	findings []Finding
	// seen dedupes locals within one top-level declaration, so a name
	// redeclared with := is judged once.
	seen map[string]bool
}

// visitor walks expressions and statements for the names they declare.
type visitor struct {
	c *checker
	// owner names the type whose fields or methods come next.
	owner string
}

// The rule names findings are reported under.
const (
	MaxWordsRule = "naming/max-name-words"
	StutterRule  = "naming/no-package-stutter"
)

// standard holds the methods that implement a standard library interface,
// whose names the interface dictates. Only names of more than one word can be
// over a limit, so the list holds only those: fmt.GoStringer, the io
// readers and writers, encoding's and encoding/json's, encoding/xml's and
// encoding/gob's marshalers, http.Handler and http.RoundTripper,
// slog.LogValuer and slog.Handler.
var standard = map[string]bool{
	"GoString":         true,
	"ReadAt":           true,
	"WriteAt":          true,
	"ReadFrom":         true,
	"WriteTo":          true,
	"ReadByte":         true,
	"UnreadByte":       true,
	"WriteByte":        true,
	"ReadRune":         true,
	"UnreadRune":       true,
	"WriteString":      true,
	"MarshalText":      true,
	"UnmarshalText":    true,
	"AppendText":       true,
	"MarshalBinary":    true,
	"UnmarshalBinary":  true,
	"AppendBinary":     true,
	"MarshalJSON":      true,
	"UnmarshalJSON":    true,
	"MarshalXML":       true,
	"UnmarshalXML":     true,
	"MarshalXMLAttr":   true,
	"UnmarshalXMLAttr": true,
	"GobEncode":        true,
	"GobDecode":        true,
	"ServeHTTP":        true,
	"RoundTrip":        true,
	"LogValue":         true,
	"WithAttrs":        true,
	"WithGroup":        true,
}

// testPrefixes are the function name prefixes go test runs.
var testPrefixes = []string{"Test", "Benchmark", "Fuzz", "Example"}

// Enabled reports whether any check is on.
func (o Options) Enabled() bool {
	return o.MaxWords > 0 || o.PackageStutter
}

const (
	kindPackage kind = iota
	kindMethod
	kindField
	kindInterfaceMethod
	kindLocal
)

// Check parses one Go file and returns its findings in source order. rel only
// names the file in parse errors.
func Check(rel string, src []byte, options Options) ([]Finding, error) {
	if !options.Enabled() {
		return nil, nil
	}

	fset := token.NewFileSet()
	file, err := parser.ParseFile(fset, rel, src, parser.SkipObjectResolution)

	if err != nil {
		return nil, err
	}

	return CheckFile(fset, file, options), nil
}

// CheckFile is Check on a file already parsed, so a caller that parses it
// anyway does not parse it twice.
func CheckFile(fset *token.FileSet, file *ast.File, options Options) []Finding {
	if !options.Enabled() {
		return nil
	}

	c := &checker{fset: fset, pkg: file.Name.Name, options: options}

	for _, decl := range file.Decls {
		c.seen = map[string]bool{}

		switch d := decl.(type) {
		case *ast.FuncDecl:
			c.funcDecl(d)
		case *ast.GenDecl:
			c.genDecl(d, false)
		}
	}

	return c.findings
}

func (c *checker) funcDecl(fn *ast.FuncDecl) {
	method := fn.Recv != nil && len(fn.Recv.List) > 0
	n := name{ident: fn.Name, kind: kindPackage, function: !method, predicate: returnsBool(fn.Type)}

	if method {
		n.kind = kindMethod
		n.owner = receiverType(fn.Recv.List[0].Type)

		for _, ident := range fn.Recv.List[0].Names {
			c.judge(name{ident: ident, kind: kindLocal, variable: true})
		}
	}

	if !dictatedFunc(fn.Name.Name, method) {
		c.judge(n)
	}

	c.signature(fn.Type)

	if fn.Body != nil {
		ast.Walk(visitor{c: c}, fn.Body)
	}
}

// signature judges a function's parameters and named results.
func (c *checker) signature(fn *ast.FuncType) {
	for _, list := range []*ast.FieldList{fn.Params, fn.Results} {
		if list == nil {
			continue
		}

		for _, field := range list.List {
			for _, ident := range field.Names {
				c.judge(name{ident: ident, kind: kindLocal, variable: true, predicate: isBool(field.Type)})
			}
		}
	}
}

func (c *checker) genDecl(decl *ast.GenDecl, local bool) {
	scope := kindPackage

	if local {
		scope = kindLocal
	}

	for _, spec := range decl.Specs {
		switch s := spec.(type) {
		case *ast.ValueSpec:
			c.valueSpec(s, scope, decl.Tok == token.VAR)
		case *ast.TypeSpec:
			c.judge(name{ident: s.Name, kind: scope})
			ast.Walk(visitor{c: c, owner: s.Name.Name}, s.Type)
		}
	}
}

// valueSpec judges the names one var or const spec declares, then walks its
// values for the closures they hold.
func (c *checker) valueSpec(spec *ast.ValueSpec, scope kind, variable bool) {
	for i, ident := range spec.Names {
		predicate := isBool(spec.Type) || (i < len(spec.Values) && boolShaped(spec.Values[i]))

		c.judge(name{ident: ident, kind: scope, variable: variable, predicate: predicate})
	}

	for _, value := range spec.Values {
		ast.Walk(visitor{c: c}, value)
	}
}

func (v visitor) Visit(node ast.Node) ast.Visitor {
	c := v.c

	switch n := node.(type) {
	case *ast.FuncLit:
		c.signature(n.Type)
		ast.Walk(visitor{c: c}, n.Body)

		return nil
	case *ast.DeclStmt:
		if decl, ok := n.Decl.(*ast.GenDecl); ok {
			c.genDecl(decl, true)
		}

		return nil
	case *ast.AssignStmt:
		c.assignment(n)
	case *ast.RangeStmt:
		if n.Tok == token.DEFINE {
			c.locals(n.Key, n.Value)
		}
	case *ast.StructType:
		c.fields(n, v.owner)

		return nil
	case *ast.InterfaceType:
		c.methods(n, v.owner)

		return nil
	case *ast.FuncType:
		// Parameter names in a function type document it; they declare nothing.
		return nil
	}

	return v
}

// assignment judges the names a := declares.
func (c *checker) assignment(assign *ast.AssignStmt) {
	if assign.Tok != token.DEFINE {
		return
	}

	for i, lhs := range assign.Lhs {
		if ident, ok := lhs.(*ast.Ident); ok {
			predicate := len(assign.Lhs) == len(assign.Rhs) && boolShaped(assign.Rhs[i])

			c.judge(name{ident: ident, kind: kindLocal, variable: true, predicate: predicate})
		}
	}
}

// locals judges the plain identifiers among exprs as local variables.
func (c *checker) locals(exprs ...ast.Expr) {
	for _, expr := range exprs {
		if ident, ok := expr.(*ast.Ident); ok {
			c.judge(name{ident: ident, kind: kindLocal, variable: true})
		}
	}
}

// fields judges a struct's named fields, then walks their types for nested
// structs and interfaces.
func (c *checker) fields(st *ast.StructType, owner string) {
	for _, field := range st.Fields.List {
		for _, ident := range field.Names {
			c.judge(name{ident: ident, kind: kindField, owner: owner, variable: true, predicate: isBool(field.Type)})
		}

		ast.Walk(visitor{c: c}, field.Type)
	}
}

// methods judges an interface's methods, except those a standard interface
// names.
func (c *checker) methods(it *ast.InterfaceType, owner string) {
	for _, field := range it.Methods.List {
		fn, ok := field.Type.(*ast.FuncType)

		for _, ident := range field.Names {
			if !standard[ident.Name] {
				c.judge(name{ident: ident, kind: kindInterfaceMethod, owner: owner, predicate: ok && returnsBool(fn)})
			}
		}
	}
}

// judge reports n under every enabled check it fails.
func (c *checker) judge(n name) {
	if n.ident == nil || n.ident.Name == "_" {
		return
	}

	if n.kind == kindLocal {
		if c.seen[n.ident.Name] {
			return
		}

		c.seen[n.ident.Name] = true
	}

	pos := c.fset.Position(n.ident.Pos())

	if c.options.MaxWords > 0 {
		if message, ok := c.maxWords(n); ok {
			c.findings = append(c.findings, Finding{
				Rule:    MaxWordsRule,
				Line:    pos.Line,
				Column:  pos.Column,
				Message: message,
			})
		}
	}

	if c.options.PackageStutter && n.kind == kindPackage {
		if message, ok := c.stutter(n.ident.Name); ok {
			c.findings = append(c.findings, Finding{
				Rule:    StutterRule,
				Line:    pos.Line,
				Column:  pos.Column,
				Message: message,
			})
		}
	}
}

func (c *checker) maxWords(n name) (string, bool) {
	words := Split(n.ident.Name)
	counted, idiom := discount(n, words)
	limit := c.options.MaxWords

	if n.kind == kindLocal && c.options.MaxLocalWords > 0 {
		limit = c.options.MaxLocalWords
	}

	if counted <= limit {
		return "", false
	}

	head := fmt.Sprintf("%q has %d words, more than %d", n.ident.Name, counted, limit)

	if idiom != "" {
		head += fmt.Sprintf(" (not counting %q)", idiom)
	}

	switch n.kind {
	case kindMethod:
		return head + fmt.Sprintf(
			"; let the receiver type carry the context: callers read (%s).%s, so drop what %s already says, "+
				"or give the extra words a type of their own, as (*Archive).Limit instead of MaxArchiveBytes",
			n.owner, n.ident.Name, n.owner,
		), true
	case kindField:
		if n.owner == "" {
			return head + "; let the struct carry the context: drop what its type already says, " +
				"or group related fields under a field of their own type", true
		}

		return head + fmt.Sprintf(
			"; let the struct carry the context: drop what %s already says, "+
				"or group related fields under a field of their own type",
			n.owner,
		), true
	case kindInterfaceMethod:
		return head + fmt.Sprintf("; let the interface carry the context: drop what %s already says", n.owner), true
	case kindLocal:
		return head + " for a local or parameter; it lives in a short scope, " +
			"where the function, its receiver and its package already say the rest", true
	default:
		return head + fmt.Sprintf(
			"; let the package name carry the context: drop what package %s already says, "+
				"or move the name to a package or type named for it, as archive.Pack instead of release.PackDirectory",
			c.pkg,
		), true
	}
}

// discount returns how many of words count toward the limit, and the idiom
// word left out, if any.
func discount(n name, words []string) (int, string) {
	if len(words) < 2 {
		return len(words), ""
	}

	first, last := strings.ToLower(words[0]), strings.ToLower(words[len(words)-1])

	switch {
	case n.function && (first == "new" || first == "must"):
		return len(words) - 1, words[0]
	case n.predicate && (first == "is" || first == "has"):
		return len(words) - 1, words[0]
	case n.kind == kindPackage && n.variable && first == "err":
		return len(words) - 1, words[0]
	case n.kind == kindLocal && n.variable && last == "err":
		return len(words) - 1, words[len(words)-1]
	}

	return len(words), ""
}

// stutter reports an exported name that starts with the package name.
func (c *checker) stutter(ident string) (string, bool) {
	pkg := strings.ToLower(strings.Map(keepAlphanumeric, c.pkg))

	if !ast.IsExported(ident) || pkg == "" || c.pkg == "main" || strings.HasSuffix(c.pkg, "_test") {
		return "", false
	}

	joined, end := "", 0
	words := Split(ident)

	for _, word := range words[:max(len(words)-1, 0)] {
		end += strings.Index(ident[end:], word) + len(word)
		joined += strings.ToLower(word)

		if len(joined) > len(pkg) {
			break
		}

		if joined == pkg {
			rest := strings.TrimLeftFunc(ident[end:], func(r rune) bool { return keepAlphanumeric(r) < 0 })

			return fmt.Sprintf(
				"%q starts with its package name: callers read %s.%s; name it %s, read as %s.%s",
				ident, c.pkg, ident, rest, c.pkg, rest,
			), true
		}
	}

	return "", false
}

func keepAlphanumeric(r rune) rune {
	if unicode.IsLetter(r) || unicode.IsDigit(r) {
		return r
	}

	return -1
}

// dictatedFunc reports whether go test, the runtime or a standard interface
// fixes a function's or method's name.
func dictatedFunc(fn string, method bool) bool {
	if !method && (fn == "init" || fn == "main") {
		return true
	}

	if method && standard[fn] {
		return true
	}

	for _, prefix := range testPrefixes {
		if rest, ok := strings.CutPrefix(fn, prefix); ok {
			if r, _ := utf8.DecodeRuneInString(rest); rest == "" || !unicode.IsLower(r) {
				return true
			}
		}
	}

	return false
}

// receiverType names a method's receiver as callers see it: *Archive,
// Archive, or Set for a generic Set[T].
func receiverType(expr ast.Expr) string {
	switch t := expr.(type) {
	case *ast.StarExpr:
		return "*" + receiverType(t.X)
	case *ast.ParenExpr:
		return receiverType(t.X)
	case *ast.IndexExpr:
		return receiverType(t.X)
	case *ast.IndexListExpr:
		return receiverType(t.X)
	case *ast.Ident:
		return t.Name
	}

	return "T"
}

// returnsBool reports whether fn returns exactly one bool.
func returnsBool(fn *ast.FuncType) bool {
	if fn.Results == nil || len(fn.Results.List) != 1 || len(fn.Results.List[0].Names) > 1 {
		return false
	}

	return isBool(fn.Results.List[0].Type)
}

func isBool(expr ast.Expr) bool {
	ident, ok := expr.(*ast.Ident)

	return ok && ident.Name == "bool"
}

// boolShaped reports whether expr is plainly a bool without type checking: a
// comparison, a logical operator, a negation or a bool literal.
func boolShaped(expr ast.Expr) bool {
	switch e := expr.(type) {
	case *ast.ParenExpr:
		return boolShaped(e.X)
	case *ast.UnaryExpr:
		return e.Op == token.NOT
	case *ast.BinaryExpr:
		switch e.Op {
		case token.EQL, token.NEQ, token.LSS, token.GTR, token.LEQ, token.GEQ, token.LAND, token.LOR:
			return true
		}
	case *ast.Ident:
		return e.Name == "true" || e.Name == "false"
	}

	return false
}
