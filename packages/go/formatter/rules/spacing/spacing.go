package spacing

import (
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

// Apply parses the source once into a shared fileContext, then runs the three
// spacing analyzers over it in a fixed order: the blank-line inserter, the
// type-order rewriter, and the embed-directive repairer. Analysis collects every
// violation up front; the rewrite phase only runs when at least one violation
// was reported, and preserves the current sequencing of insertions, type
// reordering, embed repair, and embed collapse.
func (r Rule) Apply(path string, src []byte) ([]rules.Violation, []byte, error) {
	ctx, err := newFileContext(path, src)

	if err != nil {
		return nil, nil, err
	}

	inserter := newBlankLineInserter(ctx)
	typeOrder := newTypeOrderRewriter(ctx)
	embeds := newEmbedDirectiveRepairer(ctx)

	var violations []rules.Violation

	violations = append(violations, inserter.analyze(path)...)
	violations = append(violations, typeOrder.analyze(path)...)
	violations = append(violations, embeds.analyze(path)...)

	formatted := inserter.apply()

	if len(violations) > 0 {
		reordered, changed, err := typeOrder.rewrite(path, formatted)

		if err != nil {
			return nil, nil, err
		}

		if changed {
			formatted = reordered
		}

		formatted, err = embeds.repair(path, formatted)

		if err != nil {
			return nil, nil, err
		}

		formatted = collapseEmbedSpacing(formatted)
	}

	return violations, formatted, nil
}
