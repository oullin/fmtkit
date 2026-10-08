package spacing

import (
	"fmt"
	"go/ast"
	"go/parser"
	"go/token"
)

// importAliases maps the identifier a file uses to the standard-library import
// path it refers to, restricted to the packages whose selector calls the
// spacing rule brackets with blank lines.
type importAliases map[string]string

// fileContext holds the parse state a spacing pass shares: the file is parsed
// once and the derived line-start and import-alias tables are built alongside it
// so every analyzer reads the same view of the source.
type fileContext struct {
	fset       *token.FileSet
	file       *ast.File
	src        []byte
	lineStarts []int
	aliases    importAliases
}

// stdlibSpacingImports returns the standard-library import paths whose selector
// calls receive blank-line spacing, keyed to the identifier each import binds by
// default. It replaces a package-level map so the table cannot be mutated at run
// time and is rebuilt fresh for every file.
func stdlibSpacingImports() map[string]string {
	return map[string]string{
		"sort":         "sort",
		"slices":       "slices",
		"math/rand":    "rand",
		"math/rand/v2": "rand",
	}
}

// newFileContext parses src and precomputes the shared line-start and
// import-alias tables. It returns an error when the source cannot be parsed or
// the parsed file has no backing token file.
func newFileContext(filename string, src []byte) (*fileContext, error) {
	fset := token.NewFileSet()
	file, err := parser.ParseFile(fset, filename, src, parser.ParseComments)

	if err != nil {
		return nil, err
	}

	if fset.File(file.Pos()) == nil {
		return nil, fmt.Errorf("missing token file for %s", filename)
	}

	return &fileContext{
		fset:       fset,
		file:       file,
		src:        src,
		lineStarts: buildLineStarts(src),
		aliases:    buildImportAliases(file),
	}, nil
}

// lineStartOffset returns the byte offset at which the given 1-based line begins,
// using the precomputed line-start table.
func (c *fileContext) lineStartOffset(line int) int {
	return lineStartOffset(c.lineStarts, line)
}
