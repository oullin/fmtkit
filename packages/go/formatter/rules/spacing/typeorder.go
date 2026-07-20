package spacing

import (
	"bytes"
	"go/ast"
	"go/parser"
	"go/token"
	"slices"

	"go.ollin.sh/fmtkit/formatter/rules"
)

type declBlock struct {
	decl         ast.Decl
	effectivePos token.Pos
	anchored     bool
}

type declRegion struct {
	start int
	end   int
}

func typeOrderViolations(file *ast.File, fset *token.FileSet, filename string) []rules.Violation {
	var violations []rules.Violation
	seenNonType := false

	for _, block := range topLevelDeclBlocks(file) {
		if isImportDecl(block.decl) {
			continue
		}

		if block.anchored {
			seenNonType = false

			continue
		}

		if isTypeDecl(block.decl) {
			if seenNonType {
				violations = append(violations, rules.Violation{
					Rule:    "spacing",
					File:    filename,
					Line:    fset.Position(block.decl.Pos()).Line,
					Message: "type definitions must appear at the beginning of the file",
				})
			}

			continue
		}

		seenNonType = true
	}

	return violations
}

func reorderTypeDecls(filename string, src []byte) ([]byte, bool, error) {
	fset := token.NewFileSet()
	file, err := parser.ParseFile(fset, filename, src, parser.ParseComments)

	if err != nil {
		return nil, false, err
	}

	attachEmbedDirectiveDocs(file)

	desired := desiredDeclOrder(file)

	if declOrdersEqual(file.Decls, desired) {
		return src, false, nil
	}

	// go/printer places comments by their recorded source positions, so
	// reprinting a file whose declarations moved detaches every comment from
	// its declaration and hoists body comments to file scope. Splice the
	// original text instead: each declaration travels as the literal bytes it
	// was written as, doc comment and body comments included.
	importsEnd := leadingImportDeclsEnd(file.Decls)
	regions := declSourceRegions(file.Decls[importsEnd:], fset, src)

	var out bytes.Buffer

	out.Write(bytes.TrimRight(src[:regions[file.Decls[importsEnd]].start], " \t\n"))

	for _, decl := range desired[importsEnd:] {
		region := regions[decl]

		out.WriteString("\n\n")
		out.Write(bytes.TrimSpace(src[region.start:region.end]))
	}

	// Trailing comments and blank lines past the last declaration in source
	// order belong to the file, not to whichever declaration ended up last, so
	// pin them to the end rather than letting them travel with a hoisted type.
	lastInSource := file.Decls[len(file.Decls)-1]

	if trailing := bytes.TrimSpace(src[regions[lastInSource].end:]); len(trailing) > 0 {
		out.WriteString("\n\n")
		out.Write(trailing)
	}

	out.WriteByte('\n')

	return out.Bytes(), true, nil
}

// declSourceRegions partitions the source text after the leading imports into
// one contiguous region per declaration, cut at the first line of each
// declaration's doc comment. Every byte belongs to exactly one region, so
// free-floating comments and blank lines travel with the declaration they
// follow and nothing is lost in a reorder.
func declSourceRegions(decls []ast.Decl, fset *token.FileSet, src []byte) map[ast.Decl]declRegion {
	lineStarts := buildLineStarts(src)
	regions := make(map[ast.Decl]declRegion, len(decls))
	starts := make([]int, len(decls))

	for i, decl := range decls {
		pos := decl.Pos()

		if doc := declDocComment(decl); doc != nil && doc.Pos() < pos {
			pos = doc.Pos()
		}

		start := lineStartOffset(lineStarts, fset.Position(pos).Line)

		// A doc comment group can never reach back past the previous
		// declaration; clamp defensively so no byte lands in two regions.
		if i > 0 {
			prevEnd := lineStartOffset(lineStarts, fset.Position(decls[i-1].End()).Line+1)

			if start < prevEnd {
				start = prevEnd
			}
		}

		starts[i] = start
	}

	for i, decl := range decls {
		var end int

		if i+1 < len(decls) {
			end = starts[i+1]
		} else {
			// The last declaration ends at the line after its body, not at EOF:
			// trailing comments and blank lines beyond it are the file's, not the
			// declaration's, so they must not travel when this declaration moves.
			end = lineStartOffset(lineStarts, fset.Position(decl.End()).Line+1)

			if end > len(src) {
				end = len(src)
			}
		}

		regions[decl] = declRegion{start: starts[i], end: end}
	}

	return regions
}

func declDocComment(decl ast.Decl) *ast.CommentGroup {
	switch typed := decl.(type) {
	case *ast.FuncDecl:
		return typed.Doc
	case *ast.GenDecl:
		return typed.Doc
	}

	return nil
}

func topLevelDeclBlocks(file *ast.File) []declBlock {
	matches := embedDirectiveMatches(file)
	blocks := make([]declBlock, 0, len(file.Decls))

	for _, decl := range file.Decls {
		block := declBlock{
			decl:         decl,
			effectivePos: decl.Pos(),
		}

		if group, ok := matches[decl]; ok && group.Pos() < block.effectivePos {
			block.effectivePos = group.Pos()
			block.anchored = true
		}

		blocks = append(blocks, block)
	}

	slices.SortStableFunc(blocks, func(a declBlock, b declBlock) int {
		switch {
		case a.effectivePos < b.effectivePos:
			return -1
		case a.effectivePos > b.effectivePos:
			return 1
		default:
			return 0
		}
	})

	return blocks
}

func desiredDeclOrder(file *ast.File) []ast.Decl {
	blocks := topLevelDeclBlocks(file)
	importsEnd := leadingImportDeclsEnd(file.Decls)
	preservedImports := map[ast.Decl]struct{}{}

	reordered := make([]ast.Decl, 0, len(blocks))

	for _, decl := range file.Decls[:importsEnd] {
		reordered = append(reordered, decl)
		preservedImports[decl] = struct{}{}
	}

	segment := make([]declBlock, 0, len(blocks)-importsEnd)
	flush := func() {
		for _, block := range segment {
			if isTypeDecl(block.decl) {
				reordered = append(reordered, block.decl)
			}
		}

		for _, block := range segment {
			if !isTypeDecl(block.decl) {
				reordered = append(reordered, block.decl)
			}
		}

		segment = segment[:0]
	}

	for _, block := range blocks {
		if _, ok := preservedImports[block.decl]; ok {
			continue
		}

		if block.anchored {
			flush()
			reordered = append(reordered, block.decl)

			continue
		}

		segment = append(segment, block)
	}

	flush()

	return reordered
}

func leadingImportDeclsEnd(decls []ast.Decl) int {
	importsEnd := 0

	for importsEnd < len(decls) && isImportDecl(decls[importsEnd]) {
		importsEnd++
	}

	return importsEnd
}

func declOrdersEqual(current []ast.Decl, desired []ast.Decl) bool {
	if len(current) != len(desired) {
		return false
	}

	for i := range current {
		if current[i] != desired[i] {
			return false
		}
	}

	return true
}

func hasOutOfOrderTypeDecls(file *ast.File) bool {
	seenNonType := false

	for _, block := range topLevelDeclBlocks(file) {
		if isImportDecl(block.decl) {
			continue
		}

		if block.anchored {
			seenNonType = false

			continue
		}

		if isTypeDecl(block.decl) {
			if seenNonType {
				return true
			}

			continue
		}

		seenNonType = true
	}

	return false
}
