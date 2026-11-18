package spacing

import (
	"go/ast"
	"go/token"
	"strings"
)

// literalItemSplitter puts every item of a multi-line composite literal on its
// own line. A literal whose braces share a line is left alone: it is one item
// of layout, not a list. Once the braces sit on different lines, the opening
// brace, each item, and the closing brace each start a line, and the last item
// carries the trailing comma gofmt then requires.
//
// The splitter only records where newlines (and that comma) go, keyed by byte
// offset in the original source; gofmt re-indents the result.
type literalItemSplitter struct {
	ctx   *fileContext
	edits map[int]string
}

// newLiteralItemSplitter returns a splitter bound to the shared parse state.
func newLiteralItemSplitter(ctx *fileContext) *literalItemSplitter {
	return &literalItemSplitter{
		ctx:   ctx,
		edits: map[int]string{},
	}
}

// analyze records one violation per multi-line composite literal whose items
// share a line with a brace or with each other, and the edits that split it.
// filename labels the returned violations.
func (s *literalItemSplitter) analyze(filename string) []Violation {
	var violations []Violation

	ast.Inspect(s.ctx.file, func(node ast.Node) bool {
		lit, ok := node.(*ast.CompositeLit)

		if !ok || !s.split(lit) {
			return true
		}

		violations = append(violations, Violation{
			Rule:    "spacing",
			File:    filename,
			Line:    s.line(lit.Lbrace),
			Message: "multi-line composite literal needs each item on its own line",
		})

		return true
	})

	return violations
}

// split records the edits that put each item of lit on its own line and
// reports whether lit needed any.
func (s *literalItemSplitter) split(lit *ast.CompositeLit) bool {
	if len(lit.Elts) == 0 || s.line(lit.Lbrace) == s.line(lit.Rbrace) {
		return false
	}

	changed := false
	first := lit.Elts[0]

	if s.line(first.Pos()) == s.line(lit.Lbrace) {
		s.add(s.offset(lit.Lbrace)+1, "\n")
		changed = true
	}

	for i := 1; i < len(lit.Elts); i++ {
		if s.line(lit.Elts[i].Pos()) == s.line(lit.Elts[i-1].End()) {
			s.add(s.offset(lit.Elts[i].Pos()), "\n")
			changed = true
		}
	}

	last := lit.Elts[len(lit.Elts)-1]

	if s.line(last.End()) == s.line(lit.Rbrace) {
		if !hasComma(s.ctx.src[s.offset(last.End()):s.offset(lit.Rbrace)]) {
			s.add(s.offset(last.End()), ",")
		}

		s.add(s.offset(lit.Rbrace), "\n")
		changed = true
	}

	return changed
}

// add appends text to whatever is already inserted at offset, so edits that
// land on the same byte keep the order they were recorded in.
func (s *literalItemSplitter) add(offset int, text string) {
	s.edits[offset] += text
}

func (s *literalItemSplitter) line(pos token.Pos) int {
	return s.ctx.fset.Position(pos).Line
}

func (s *literalItemSplitter) offset(pos token.Pos) int {
	return s.ctx.fset.Position(pos).Offset
}

// hasComma reports whether gap, the text between a literal's last item and its
// closing brace on the same line, already holds the trailing comma. Block
// comments are skipped so a comma inside one does not count.
func hasComma(gap []byte) bool {
	text := string(gap)

	for {
		start := strings.Index(text, "/*")

		if start < 0 {
			return strings.Contains(text, ",")
		}

		if strings.Contains(text[:start], ",") {
			return true
		}

		end := strings.Index(text[start+2:], "*/")

		if end < 0 {
			return false
		}

		text = text[start+2+end+2:]
	}
}
