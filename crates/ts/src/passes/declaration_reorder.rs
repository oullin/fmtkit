//! Group runs of imports and of consts: single-line entries first, multiline
//! ones after, with a blank line around every multiline entry.

use fmtkit_core::{Edit, EditSet};
use oxc_ast::AstKind;
use oxc_ast::ast::{ArrayExpressionElement, BindingPattern, Expression, ObjectPropertyKind, Program, Statement, VariableDeclaration};
use oxc_ast_visit::Visit;
use rustc_hash::FxHashSet;

use super::lists::{Item, for_each_list};
use crate::syntax::{contains_comment_opener, line_indent, line_start};

/// One rewrite per eligible run. A run nested inside another run's rewrite is
/// left for the next run of the pass, so the set never overlaps.
pub(crate) fn edits<'a>(text: &'a str, program: &'a Program<'a>) -> EditSet {
    let mut edits = EditSet::new();

    for_each_list(program, |items| {
        for group in runs(items, Item::is_import) {
            edits.extend(group_edit(text, group, true));
        }

        for group in runs(items, Item::is_const) {
            edits.extend(group_edit(text, group, can_reorder_consts(text, group)));
        }
    });

    edits.retain_non_overlapping();

    edits
}

/// Maximal runs of two or more consecutive items matching `matches`.
fn runs<'l, 'a>(items: &'l [Item<'a>], matches: impl Fn(Item<'a>) -> bool + Copy + 'l) -> impl Iterator<Item = &'l [Item<'a>]> + 'l {
    items.chunk_by(move |a, b| matches(*a) == matches(*b)).filter(move |run| run.len() > 1 && matches(run[0]))
}

fn source<'t>(text: &'t str, item: Item<'_>) -> &'t str {
    let span = item.span();

    &text[span.start as usize..span.end as usize]
}

fn is_multiline(text: &str, item: Item<'_>) -> bool {
    source(text, item).contains('\n')
}

fn group_edit(text: &str, group: &[Item<'_>], can_reorder: bool) -> Option<Edit> {
    // The rewrite rebuilds the span from node text alone, so a comment in a gap
    // would be deleted; losing an ordering is recoverable, losing a comment is not.
    if group.windows(2).any(|pair| {
        let (end, start) = (pair[0].span().end, pair[1].span().start);

        start > end && contains_comment_opener(&text[end as usize..start as usize])
    }) {
        return None;
    }

    let multiline_count = group.iter().filter(|item| is_multiline(text, **item)).count();

    if multiline_count == 0 || multiline_count == group.len() {
        return None;
    }

    let first_start = group[0].span().start;
    let last_end = group[group.len() - 1].span().end;
    let start = line_start(text, first_start);

    // v1 rewrote from the line start and dropped any code sharing the first
    // declaration's line; decline instead.
    if !text[start as usize..first_start as usize].bytes().all(|b| b == b' ' || b == b'\t') {
        return None;
    }

    let ordered: Vec<Item<'_>> = if can_reorder {
        group.iter().filter(|item| !is_multiline(text, **item)).chain(group.iter().filter(|item| is_multiline(text, **item))).copied().collect()
    } else {
        group.to_vec()
    };

    let mut replacement = String::with_capacity((last_end - start) as usize + 2 * group.len());

    for (index, item) in ordered.iter().enumerate() {
        if index > 0 {
            let spaced = is_multiline(text, ordered[index - 1]) || is_multiline(text, *item);

            replacement.push_str(if spaced { "\n\n" } else { "\n" });
        }

        replacement.push_str(line_indent(text, item.span().start));
        replacement.push_str(source(text, *item));
    }

    (replacement != text[start as usize..last_end as usize]).then(|| Edit::new(start, last_end, replacement))
}

fn declaration(item: Item<'_>) -> Option<&VariableDeclaration<'_>> {
    match item.statement() {
        Some(Statement::VariableDeclaration(declaration)) => Some(declaration),
        _ => None,
    }
}

/// Consts may move when every initializer is free of side effects and no
/// multiline const is read by a later single-line one.
fn can_reorder_consts(text: &str, group: &[Item<'_>]) -> bool {
    let safe = group.iter().all(|item| {
        declaration(*item).is_some_and(|declaration| {
            declaration
                .declarations
                .iter()
                .all(|declarator| matches!(declarator.id, BindingPattern::BindingIdentifier(_)) && declarator.init.as_ref().is_none_or(is_side_effect_free))
        })
    });

    if !safe {
        return false;
    }

    for (index, item) in group.iter().enumerate() {
        if !is_multiline(text, *item) {
            continue;
        }

        let names: FxHashSet<&str> = declaration(*item)
            .into_iter()
            .flat_map(|declaration| declaration.declarations.iter())
            .filter_map(|declarator| match &declarator.id {
                BindingPattern::BindingIdentifier(id) => Some(id.name.as_str()),
                _ => None,
            })
            .collect();

        if group[index + 1..].iter().any(|later| !is_multiline(text, *later) && uses_any(*later, &names)) {
            return false;
        }
    }

    true
}

fn is_side_effect_free(expression: &Expression<'_>) -> bool {
    match expression {
        Expression::ArrowFunctionExpression(_)
        | Expression::FunctionExpression(_)
        | Expression::Identifier(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::NumericLiteral(_)
        | Expression::BigIntLiteral(_)
        | Expression::RegExpLiteral(_)
        | Expression::StringLiteral(_) => true,
        Expression::ArrayExpression(array) => array.elements.iter().all(|element| match element {
            ArrayExpressionElement::Elision(_) => true,
            ArrayExpressionElement::SpreadElement(_) => false,
            element => element.as_expression().is_some_and(is_side_effect_free),
        }),
        Expression::ObjectExpression(object) => object.properties.iter().all(|property| match property {
            ObjectPropertyKind::SpreadProperty(spread) => is_side_effect_free(&spread.argument),
            ObjectPropertyKind::ObjectProperty(property) => {
                (!property.computed || property.key.as_expression().is_some_and(is_side_effect_free)) && is_side_effect_free(&property.value)
            }
        }),
        Expression::TemplateLiteral(template) => template.expressions.iter().all(is_side_effect_free),
        _ => false,
    }
}

/// Whether any ESTree `Identifier` below `item` has one of `names`.
fn uses_any(item: Item<'_>, names: &FxHashSet<&str>) -> bool {
    let mut finder = NameFinder { names, found: false };

    if let Some(statement) = item.statement() {
        finder.visit_statement(statement);
    }

    finder.found
}

struct NameFinder<'n, 's> {
    names: &'n FxHashSet<&'s str>,
    found: bool,
}

impl<'a> Visit<'a> for NameFinder<'_, '_> {
    fn enter_node(&mut self, kind: AstKind<'a>) {
        let name = match kind {
            AstKind::IdentifierReference(it) => it.name.as_str(),
            AstKind::IdentifierName(it) => it.name.as_str(),
            AstKind::BindingIdentifier(it) => it.name.as_str(),
            AstKind::LabelIdentifier(it) => it.name.as_str(),
            _ => return,
        };

        self.found |= self.names.contains(name);
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{apply_once, compute, until_stable};
    use crate::passes::Pass;

    fn reorder(source: &str) -> String {
        apply_once(Pass::DeclarationReorder, "sample.ts", source)
    }

    fn edits_for(source: &str) -> fmtkit_core::EditSet {
        compute(Pass::DeclarationReorder, "sample.ts", source)
    }

    #[test]
    fn moves_single_line_consts_ahead_of_a_multiline_const_when_initializers_are_side_effect_free() {
        let result = reorder("const big = {\n\tone: 1,\n\ttwo: 2,\n};\nconst small = 1;\n");

        assert!(result.starts_with("const small = 1;\n\nconst big = {"), "{result}");
    }

    #[test]
    fn keeps_order_when_a_const_initializer_can_have_side_effects() {
        let source = "const big = load({\n\tone: 1,\n});\nconst small = 1;\n";
        let result = reorder(source);

        assert_eq!(result.find("const big"), source.find("const big"));
        assert!(result.find("const big = load({").unwrap() < result.find("const small = 1;").unwrap());
    }

    #[test]
    fn keeps_order_when_a_later_single_line_const_uses_a_multiline_const() {
        let result = reorder("const big = {\n\tone: 1,\n};\nconst small = big;\n");

        assert!(result.find("const big = {").unwrap() < result.find("const small = big;").unwrap());
    }

    #[test]
    fn reorders_import_groups_so_single_line_imports_precede_multiline_ones() {
        let result = reorder("import {\n\talpha,\n\tbeta,\n} from 'wide';\nimport { tiny } from 'narrow';\n");

        assert!(result.starts_with("import { tiny } from 'narrow';\n\nimport {"), "{result}");
    }

    #[test]
    fn returns_no_edits_for_source_with_syntax_errors() {
        assert!(edits_for("const broken = {;\nconst small = 1;\n").is_empty());
    }

    #[test]
    fn declines_to_reorder_a_const_group_holding_a_comment_between_declarations() {
        let source = "const small = 1;\n// Explains the next declaration.\n// Second line of the explanation.\nconst big = {\n\tone: 1,\n};\n";

        assert!(edits_for(source).is_empty());
        assert_eq!(reorder(source), source);
    }

    #[test]
    fn declines_to_reorder_a_const_group_holding_a_trailing_comment_beside_a_declaration() {
        assert!(edits_for("const small = 1; // Why this value.\nconst big = {\n\tone: 1,\n};\n").is_empty());
    }

    #[test]
    fn declines_to_reorder_a_const_group_holding_a_block_comment_between_declarations() {
        assert!(edits_for("const small = 1;\n/* Explains the next declaration. */\nconst big = {\n\tone: 1,\n};\n").is_empty());
    }

    #[test]
    fn declines_to_reorder_an_import_group_holding_a_comment_between_declarations() {
        assert!(edits_for("import { tiny } from 'narrow';\n// Explains the wide import.\nimport {\n\talpha,\n\tbeta,\n} from 'wide';\n").is_empty());
    }

    #[test]
    fn still_reorders_a_group_whose_only_comment_sits_above_the_first_declaration() {
        let result = reorder("// Explains the whole group.\nconst big = {\n\tone: 1,\n};\nconst small = 1;\n");

        assert!(result.starts_with("// Explains the whole group.\nconst small = 1;\n\nconst big = {"), "{result}");
    }

    /// v1 applied both rewrites against stale offsets and corrupted the file;
    /// the outer group now lands first and the inner one on the next run.
    #[test]
    fn nested_groups_reorder_without_overlapping_edits() {
        let source = "const make = () => {\n\tconst inner = {\n\t\ta: 1,\n\t};\n\tconst x = 1;\n\treturn [inner, x];\n};\nconst y = 2;\n";
        let expected = "const y = 2;\n\nconst make = () => {\n\tconst x = 1;\n\n\tconst inner = {\n\t\ta: 1,\n\t};\n\treturn [inner, x];\n};\n";

        assert_eq!(edits_for(source).len(), 1);
        assert_eq!(until_stable(Pass::DeclarationReorder, "sample.ts", source), expected);
    }

    #[test]
    fn declines_a_group_that_shares_its_first_line_with_other_code() {
        assert!(edits_for("let a = 1; const big = {\n\tone: 1,\n};\nconst small = 1;\n").is_empty());
    }
}
