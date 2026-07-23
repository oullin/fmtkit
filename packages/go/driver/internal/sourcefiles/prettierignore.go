package sourcefiles

import (
	"os"
	"regexp"
	"strings"
)

// prettierIgnore matches repo-relative paths against a parsed .prettierignore
// file using gitignore semantics. Supported constructs: comments (#), blank
// lines, negation (!, last match wins), leading-/ anchoring, trailing-/
// directory patterns, and the * ? [...] and ** wildcards. Exotic constructs —
// escaped leading #/! (\# and \!) and trailing-space escapes — are not
// supported: a leading # or ! is always read as a comment or a negation.
type prettierIgnore struct {
	patterns []ignorePattern
}

// ignorePattern is one compiled .prettierignore line.
type ignorePattern struct {
	re      *regexp.Regexp
	negated bool
	dirOnly bool
}

// loadPrettierIgnore reads the .prettierignore at path and compiles it. It
// returns nil (and no error) when the file is absent, so callers can treat "no
// file" as "nothing filtered".
func loadPrettierIgnore(path string) (*prettierIgnore, error) {
	data, err := os.ReadFile(path)

	if err != nil {
		if os.IsNotExist(err) {
			return nil, nil
		}

		return nil, err
	}

	return compilePrettierIgnore(data), nil
}

// compilePrettierIgnore parses raw .prettierignore bytes into matchable
// patterns, skipping blank lines, comments, and lines that fail to compile.
func compilePrettierIgnore(data []byte) *prettierIgnore {
	ignore := &prettierIgnore{}

	for _, line := range strings.Split(string(data), "\n") {
		if pattern, ok := compilePattern(line); ok {
			ignore.patterns = append(ignore.patterns, pattern)
		}
	}

	return ignore
}

// compilePattern turns one raw line into an ignorePattern, reporting false for
// blank lines, comments, and lines whose glob yields an invalid regex — none of
// which carry a usable pattern.
func compilePattern(line string) (ignorePattern, bool) {
	line = strings.TrimRight(line, "\r")
	line = strings.TrimRight(line, " \t")

	if line == "" || strings.HasPrefix(line, "#") {
		return ignorePattern{}, false
	}

	negated := false

	if strings.HasPrefix(line, "!") {
		negated = true
		line = line[1:]
	}

	dirOnly := false

	if strings.HasSuffix(line, "/") {
		dirOnly = true
		line = strings.TrimSuffix(line, "/")
	}

	if line == "" {
		return ignorePattern{}, false
	}

	anchored := strings.HasPrefix(line, "/") || strings.Contains(line, "/")

	line = strings.TrimPrefix(line, "/")

	body := translateGlob(line)

	prefix := "(?:^|/)"

	if anchored {
		prefix = "^"
	}

	re, err := regexp.Compile(prefix + body + "$")

	if err != nil {
		return ignorePattern{}, false
	}

	return ignorePattern{
		re:      re,
		negated: negated,
		dirOnly: dirOnly,
	}, true
}

// ignores reports whether rel — a slash-separated path relative to the ignore
// file's directory — is excluded. Later matches win, so a negation can
// re-include a path an earlier pattern excluded.
func (p *prettierIgnore) ignores(rel string) bool {
	ignored := false

	for _, pattern := range p.patterns {
		if pattern.matches(rel) {
			ignored = !pattern.negated
		}
	}

	return ignored
}

// matches reports whether the pattern covers rel, either directly or because
// one of rel's ancestor directories matches — excluding a directory excludes
// everything beneath it.
func (p ignorePattern) matches(rel string) bool {
	if !p.dirOnly && p.re.MatchString(rel) {
		return true
	}

	for i := 0; i < len(rel); i++ {
		if rel[i] == '/' && p.re.MatchString(rel[:i]) {
			return true
		}
	}

	return false
}

// translateGlob converts a gitignore-style glob body (no leading slash, no
// trailing slash, no leading !) into a regular-expression fragment matched
// against a slash-separated path. * and ? stay within one segment; ** spans
// segments.
func translateGlob(pattern string) string {
	var b strings.Builder

	for i := 0; i < len(pattern); {
		switch c := pattern[i]; c {
		case '*':
			stars := i

			for stars < len(pattern) && pattern[stars] == '*' {
				stars++
			}

			doubled := stars-i >= 2

			if doubled && stars < len(pattern) && pattern[stars] == '/' {
				b.WriteString("(?:.*/)?")

				i = stars + 1
			} else if doubled {
				b.WriteString(".*")

				i = stars
			} else {
				b.WriteString("[^/]*")

				i = stars
			}
		case '?':
			b.WriteString("[^/]")

			i++
		case '[':
			if class, next, ok := translateClass(pattern, i); ok {
				b.WriteString(class)

				i = next
			} else {
				b.WriteString("\\[")

				i++
			}
		default:
			if isRegexMeta(c) {
				b.WriteByte('\\')
			}

			b.WriteByte(c)

			i++
		}
	}

	return b.String()
}

// translateClass converts the [...] character class starting at i into a regex
// class, mapping a leading ! to ^. It reports false when the bracket has no
// close, leaving the caller to treat [ literally.
func translateClass(pattern string, i int) (string, int, bool) {
	j := i + 1

	if j < len(pattern) && (pattern[j] == '!' || pattern[j] == '^') {
		j++
	}

	if j < len(pattern) && pattern[j] == ']' {
		j++
	}

	for j < len(pattern) && pattern[j] != ']' {
		j++
	}

	if j >= len(pattern) {
		return "", 0, false
	}

	inner := pattern[i+1 : j]

	if strings.HasPrefix(inner, "!") {
		inner = "^" + inner[1:]
	}

	return "[" + inner + "]", j + 1, true
}

func isRegexMeta(c byte) bool {
	switch c {
	case '.', '\\', '+', '(', ')', '|', '{', '}', '^', '$', ']':
		return true
	default:
		return false
	}
}
