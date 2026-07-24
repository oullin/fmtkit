// Package filetypes is the extension taxonomy that decides which files the
// formatter and linter own. It is pure path classification — no git, no
// filesystem — so callers can filter a discovered file list without any I/O.
package filetypes

import "strings"

// Filter classifies paths by extension. IncludeDeclarations, when set, keeps
// .d.ts declaration files that would otherwise be dropped.
type Filter struct {
	IncludeDeclarations bool
}

// targetSuffixes are the extensions the sidecar knows how to format. The .ts
// entry also covers .d.ts; whether declarations are kept is decided separately.
var targetSuffixes = []string{".ts", ".vue", ".html", ".htm", ".md", ".markdown"}

// Formattable reports whether path is one the formatter owns: the TS and Vue
// families plus the HTML and Markdown documents whose embedded scripts get
// formatted.
func (f Filter) Formattable(path string) bool {
	matched := false

	for _, suffix := range targetSuffixes {
		if strings.HasSuffix(path, suffix) {
			matched = true

			break
		}
	}

	if !matched {
		return false
	}

	return f.IncludeDeclarations || !strings.HasSuffix(path, ".d.ts")
}

// Lintable reports whether oxlint can lint path: the TS family (minus
// declarations unless IncludeDeclarations) and Vue, but not HTML or Markdown.
func (f Filter) Lintable(path string) bool {
	if !strings.HasSuffix(path, ".ts") && !strings.HasSuffix(path, ".vue") {
		return false
	}

	return f.IncludeDeclarations || !strings.HasSuffix(path, ".d.ts")
}
