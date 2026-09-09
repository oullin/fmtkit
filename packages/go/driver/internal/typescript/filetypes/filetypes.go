// Package filetypes is the extension taxonomy that decides which files the
// formatter and linter own. It is pure path classification — no git, no
// filesystem — so callers can filter a discovered file list without any I/O.
package filetypes

import (
	"path/filepath"
	"slices"
	"strings"
)

// Filter classifies paths by extension. IncludeDeclarations, when set, keeps
// the .d.ts declaration family that would otherwise be dropped.
type Filter struct {
	IncludeDeclarations bool
}

// tsFamilySuffixes are the TypeScript-family extensions oxc parses directly.
// The extension is what selects the parser dialect, so .tsx is what turns JSX
// on; a .tsx file read as .ts fails to parse the moment it holds an element.
var tsFamilySuffixes = []string{".ts", ".tsx", ".mts", ".cts"}

// documentSuffixes are the host documents that embed scripts rather than being
// scripts: their fenced or <script> blocks are what gets formatted.
var documentSuffixes = []string{".vue", ".html", ".htm", ".md", ".markdown"}

// declarationSuffixes are the declaration forms dropped unless a caller asks
// for them. There is no .d.tsx: JSX cannot appear in a declaration file.
var declarationSuffixes = []string{".d.ts", ".d.mts", ".d.cts"}

// hasSuffix reports whether path ends in any of suffixes.
func hasSuffix(path string, suffixes []string) bool {
	return slices.ContainsFunc(suffixes, func(suffix string) bool {
		return strings.HasSuffix(path, suffix)
	})
}

// Formattable reports whether path is one the formatter owns: the TS and Vue
// families plus the HTML and Markdown documents whose embedded scripts get
// formatted.
func (f Filter) Formattable(path string) bool {
	if !hasSuffix(path, tsFamilySuffixes) && !hasSuffix(path, documentSuffixes) {
		return false
	}

	return f.IncludeDeclarations || !hasSuffix(path, declarationSuffixes)
}

// scriptSuffixes are the plain JavaScript extensions the complexity check
// scores alongside the TS family. oxfmt and oxlint are not pointed at them
// here, so they are kept out of Formattable and Lintable.
var scriptSuffixes = []string{".js", ".jsx", ".mjs", ".cjs"}

// testInfixes mark a script as a test rather than the source under test. The
// complexity check leaves tests out — a long table of cases is not the
// complexity it is about — matching the Go lane's *_test.go exclusion.
var testInfixes = []string{".test.", ".spec."}

// Scorable reports whether the complexity check can score path: the TS family
// plus plain JavaScript, minus declarations and tests.
func (f Filter) Scorable(path string) bool {
	if !hasSuffix(path, tsFamilySuffixes) && !hasSuffix(path, scriptSuffixes) {
		return false
	}

	if IsTestScript(path) {
		return false
	}

	return f.IncludeDeclarations || !hasSuffix(path, declarationSuffixes)
}

// IsTestScript reports whether a script path names a test file.
func IsTestScript(path string) bool {
	base := filepath.Base(path)

	return slices.ContainsFunc(testInfixes, func(infix string) bool {
		return strings.Contains(base, infix)
	})
}

// Lintable reports whether oxlint can lint path: the TS family (minus
// declarations unless IncludeDeclarations) and Vue, but not HTML or Markdown.
func (f Filter) Lintable(path string) bool {
	if !hasSuffix(path, tsFamilySuffixes) && !strings.HasSuffix(path, ".vue") {
		return false
	}

	return f.IncludeDeclarations || !hasSuffix(path, declarationSuffixes)
}
