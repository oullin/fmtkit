package lint

import (
	"bytes"
	"fmt"
	"go/ast"
	"go/token"
	"sort"
	"unicode/utf8"
)

// span is the byte range of one string literal or comment in the source.
type span struct {
	start, end int
	comment    bool
}

// lineWalker holds what the line-length check needs about one file. spans is
// built from file and strings the first time a line is too long.
type lineWalker struct {
	src     []byte
	file    *ast.File
	tokens  *token.File
	strings []*ast.BasicLit
	spans   []span
	built   bool
	imports [][2]int
	limits  Limits
}

// defaultTabWidth is what a tab counts when the request names no width.
const defaultTabWidth = 4

// lineLength reports every line wider than the limit, except the ones that
// cannot reasonably be made shorter: import lines, //go: directives, comment
// lines whose overflow is one URL, and lines whose overflow sits inside one
// string literal (struct tags included). Width counts runes, and a tab
// advances to the next multiple of the tab width. strings are the file's
// string literals, which the nesting walk collects.
func lineLength(fset *token.FileSet, file *ast.File, src []byte, limits Limits, strings []*ast.BasicLit) []Finding {
	if limits.TabWidth < 1 {
		limits.TabWidth = defaultTabWidth
	}

	tokens := fset.File(file.Pos())
	w := lineWalker{
		src:     src,
		file:    file,
		tokens:  tokens,
		strings: strings,
		imports: importSpans(tokens, file),
		limits:  limits,
	}

	var out []Finding

	start := 0

	for number := 1; start <= len(src); number++ {
		end := bytes.IndexByte(src[start:], '\n')

		if end < 0 {
			end = len(src)
		} else {
			end += start
		}

		if f, ok := w.line(number, start, bytes.TrimSuffix(src[start:end], []byte("\r"))); ok {
			out = append(out, f)
		}

		start = end + 1
	}

	return out
}

// line judges one line, which starts at byte offset at in the source.
func (w *lineWalker) line(number, at int, text []byte) (Finding, bool) {
	limit := w.limits.MaxLineLength

	// A rune is at least a byte and a tab at most TabWidth columns, so this
	// bound settles most lines without decoding them.
	if len(text)+bytes.Count(text, []byte("\t"))*(w.limits.TabWidth-1) <= limit {
		return Finding{}, false
	}

	width, cut := measure(text, w.limits.TabWidth, limit)

	if width <= limit || w.imported(at, text, cut) {
		return Finding{}, false
	}

	if !w.built {
		w.spans, w.built = literals(w.tokens, w.file, w.src, w.strings), true
	}

	if w.directive(at, text) || w.unbreakable(at, text, cut) {
		return Finding{}, false
	}

	return Finding{
		Rule:    RuleLineLength,
		Line:    number,
		Column:  cut + 1,
		Message: fmt.Sprintf("This line has a length of %d. Maximum allowed is %d.", width, limit),
	}, true
}

// directive reports whether a line is a //go: comment, such as
// //go:generate. A raw string line that only starts with //go: is not one.
func (w *lineWalker) directive(at int, text []byte) bool {
	indent := bytes.IndexFunc(text, notBlank)

	if indent < 0 || !bytes.HasPrefix(text[indent:], []byte("//go:")) {
		return false
	}

	s := w.spanAt(at + indent)

	return s != nil && s.comment && s.start == at+indent
}

// unbreakable reports whether the overflow of a line, the bytes from cut on,
// is one URL in a comment line or sits inside one string literal.
func (w *lineWalker) unbreakable(at int, text []byte, cut int) bool {
	// The whole overflow sits inside a string that runs past the line, such
	// as a line inside a multi-line raw string, whatever its characters are.
	if s := w.spanAt(at + cut); s != nil && !s.comment && s.end >= at+len(text) {
		return true
	}

	first, last := significant(text, cut)

	if last < 0 {
		return false
	}

	head := w.spanAt(at + bytes.IndexFunc(text, notBlank))
	tail := w.spanAt(at + last)

	if tail == nil {
		return false
	}

	if tail.comment {
		return head != nil && head.comment && url(text, first)
	}

	return tail.start <= at+first
}

// spanAt returns the literal or comment holding byte offset, if any.
func (w *lineWalker) spanAt(offset int) *span {
	i := sort.Search(len(w.spans), func(i int) bool { return w.spans[i].end > offset })

	if i < len(w.spans) && w.spans[i].start <= offset {
		return &w.spans[i]
	}

	return nil
}

// measure returns a line's width and the byte offset of the first rune past
// limit columns (len(text) when it fits).
func measure(text []byte, tabWidth, limit int) (width, cut int) {
	cut = len(text)

	for i := 0; i < len(text); {
		r, size := utf8.DecodeRune(text[i:])

		if r == '\t' {
			width += tabWidth - width%tabWidth
		} else {
			width++
		}

		if width > limit && cut == len(text) {
			cut = i
		}

		i += size
	}

	return width, cut
}

// significant returns the overflow's first non-blank byte and the line's last
// byte that is neither blank nor closing punctuation, so a literal followed
// only by `)`, `,` and the like still counts as the whole overflow. first is
// never past last; last is -1 for a blank line.
func significant(text []byte, cut int) (first, last int) {
	last = len(bytes.TrimRight(text, " \t)]},;")) - 1
	first = cut

	for first < len(text) && (text[first] == ' ' || text[first] == '\t') {
		first++
	}

	return min(first, last), last
}

// url reports whether the word at byte first runs to the end of the line and
// is a URL: a scheme, then ://, then at least one byte. Punctuation such as
// ( or < may open the word.
func url(text []byte, first int) bool {
	start := bytes.LastIndexAny(text[:first], " \t") + 1
	rest := bytes.TrimRight(text[start:], " \t")

	if bytes.ContainsAny(rest, " \t") {
		return false
	}

	sep := bytes.Index(rest, []byte("://"))

	if sep < 0 || sep+3 == len(rest) {
		return false
	}

	return scheme(bytes.TrimLeft(rest[:sep], "([<{'\"`"))
}

// scheme reports whether s is a URL scheme: a letter, then letters, digits,
// +, - or . (RFC 3986, section 3.1).
func scheme(s []byte) bool {
	if len(s) == 0 || !isLetter(s[0]) {
		return false
	}

	for _, c := range s[1:] {
		if !isLetter(c) && !('0' <= c && c <= '9') && c != '+' && c != '-' && c != '.' {
			return false
		}
	}

	return true
}

func isLetter(c byte) bool {
	return 'a' <= c && c <= 'z' || 'A' <= c && c <= 'Z'
}

func notBlank(r rune) bool {
	return r != ' ' && r != '\t'
}

// importSpans returns the byte range of every import spec. A comment on its
// own line inside import ( ... ), or a declaration after one on the same line,
// lies outside every spec, so it is checked.
func importSpans(tokens *token.File, file *ast.File) [][2]int {
	spans := make([][2]int, 0, len(file.Imports))

	for _, spec := range file.Imports {
		spans = append(spans, [2]int{tokens.Offset(spec.Pos()), tokens.Offset(spec.End())})
	}

	return spans
}

// imported reports whether a line's overflow, from byte cut of the line at
// offset at, is an import spec: it starts inside one, and only blanks, a
// semicolon or a comment follow that spec on the line.
func (w *lineWalker) imported(at int, text []byte, cut int) bool {
	offset := at + cut

	for _, span := range w.imports {
		if span[0] > offset || offset >= span[1] {
			continue
		}

		rest := bytes.TrimLeft(text[min(span[1]-at, len(text)):], " \t;")

		return len(rest) == 0 || bytes.HasPrefix(rest, []byte("//")) || bytes.HasPrefix(rest, []byte("/*"))
	}

	return false
}

// literals returns the string literals and comments of the file in source
// order. Ends come from the source rather than the literal text, which drops
// carriage returns from raw strings and comments.
func literals(tokens *token.File, file *ast.File, src []byte, strings []*ast.BasicLit) []span {
	out := make([]span, 0, len(strings))

	for _, lit := range strings {
		start := tokens.Offset(lit.ValuePos)
		out = append(out, span{start: start, end: literalEnd(src, start, lit.Value)})
	}

	for _, group := range file.Comments {
		for _, c := range group.List {
			start := tokens.Offset(c.Slash)
			out = append(out, span{start: start, end: literalEnd(src, start, c.Text), comment: true})
		}
	}

	sort.Slice(out, func(i, j int) bool { return out[i].start < out[j].start })

	return out
}

// literalEnd is the offset just past the string or comment starting at start.
func literalEnd(src []byte, start int, lit string) int {
	var closing string

	switch {
	case lit == "" || lit[0] == '"' || lit[0] == '\'':
		return start + len(lit)
	case lit[0] == '`':
		closing = "`"
		start++
	case len(lit) > 1 && lit[1] == '*':
		closing = "*/"
		start += 2
	default:
		end := bytes.IndexByte(src[start:], '\n')

		if end < 0 {
			return len(src)
		}

		return start + end
	}

	end := bytes.Index(src[start:], []byte(closing))

	if end < 0 {
		return len(src)
	}

	return start + end + len(closing)
}
