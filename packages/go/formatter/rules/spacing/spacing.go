package spacing

import (
	"fmt"
	"go/ast"
	"go/parser"
	"go/token"

	"go.ollin.sh/fmtkit/formatter/rules"
)

// Rule enforces blank-line and type-order spacing rules.
type Rule struct{}

// New returns the built-in spacing rule.
func New() Rule {
	return Rule{}
}

// Name returns the rule identifier used in reports.
func (Rule) Name() string {
	return "spacing"
}

func (r Rule) Apply(path string, src []byte) ([]rules.Violation, []byte, error) {
	return analyse(path, src)
}

func analyse(filename string, src []byte) ([]rules.Violation, []byte, error) {
	fset := token.NewFileSet()
	file, err := parser.ParseFile(fset, filename, src, parser.ParseComments)

	if err != nil {
		return nil, nil, err
	}

	tokenFile := fset.File(file.Pos())

	if tokenFile == nil {
		return nil, nil, fmt.Errorf("missing token file for %s", filename)
	}

	lineStarts := buildLineStarts(src)
	insertions := map[int]struct{}{}
	aliases := buildImportAliases(file)

	var violations []rules.Violation

	inspectStmtLists(file, func(list []ast.Stmt) {
		for i := 0; i < len(list)-1; i++ {
			current := list[i]
			next := list[i+1]
			endLine := fset.Position(current.End()).Line
			nextLine := fset.Position(next.Pos()).Line

			if currentLine, ok := setupSpacingLine(list, i, current, next, aliases, fset); ok {
				violations = append(violations, rules.Violation{
					Rule:    "spacing",
					File:    filename,
					Line:    currentLine,
					Message: "missing blank line before selector call setup",
				})

				offset := lineStartOffset(lineStarts, currentLine)
				insertions[offset] = struct{}{}
			}

			if endLine == nextLine {
				continue
			}

			if message, ok := statementGapRule(current, next, aliases, fset); ok {
				if nextLine < endLine+2 {
					violations = append(violations, rules.Violation{
						Rule:    "spacing",
						File:    filename,
						Line:    nextLine,
						Message: message,
					})

					offset := lineStartOffset(lineStarts, nextLine)
					insertions[offset] = struct{}{}
				}
			}
		}
	})

	for i := 0; i < len(file.Decls)-1; i++ {
		current := file.Decls[i]
		next := file.Decls[i+1]

		if !requiresTypeDeclSpacing(current, next) {
			continue
		}

		endLine := fset.Position(current.End()).Line
		nextLine := fset.Position(next.Pos()).Line

		if nextLine >= endLine+2 {
			continue
		}

		violations = append(violations, rules.Violation{
			Rule:    "spacing",
			File:    filename,
			Line:    nextLine,
			Message: "missing blank line around type definition",
		})

		offset := lineStartOffset(lineStarts, nextLine)
		insertions[offset] = struct{}{}
	}

	violations = append(violations, typeOrderViolations(file, fset, filename)...)
	violations = append(violations, embedAdjacencyViolations(file, fset, filename)...)

	formatted := src

	if len(insertions) > 0 {
		formatted = applyInsertions(formatted, insertions)
	}

	if len(violations) > 0 {
		reordered, changed, err := reorderTypeDecls(filename, formatted)

		if err != nil {
			return nil, nil, err
		}

		if changed {
			formatted = reordered
		}

		formatted, err = repairDetachedEmbedDirectives(filename, formatted)

		if err != nil {
			return nil, nil, err
		}

		formatted = collapseEmbedSpacing(formatted)
	}

	return violations, formatted, nil
}
