//! `anti-slop-martinfowler/repeated-switches`: one discriminant branched on
//! over the same cases in several places in a file, every place reported with
//! the count.
//!
//! A place is a `switch` (not on a literal, so `switch (true)` is skipped), an
//! `if`/`else if` chain of two or more equality tests (`==`, `===`, `!=`,
//! `!==`, joined with `||` for several cases) on one value, a run of two or
//! more adjacent guard `if`s without `else` on one value, or a nested ternary
//! chain of two or more such tests. A case is a literal, a template without
//! substitutions, or an enum-like member (`Kind.Ready`). The discriminant is
//! matched by its source text, whitespace collapsed. Places are repeated when
//! at least `minOccurrences` (default 2) of them share `minSharedCases`
//! (default 2) case values with another place on the same text: the same text
//! can name different unions in different functions, and overlapping cases
//! show it is one.
//!
//! Options: `{ ignoreTests?: boolean = true, minOccurrences?: integer >= 2 = 2,
//! minSharedCases?: integer >= 1 = 2 }`. Test files (upstream's `isTestFile`)
//! are skipped unless `ignoreTests` is `false`.

use oxc_ast::AstKind;
use oxc_ast::ast::{Expression, LogicalOperator, Statement};
use oxc_span::{GetSpan, Span};
use oxc_syntax::operator::BinaryOperator;
use rustc_hash::{FxHashMap, FxHashSet};
use serde_json::Value;

use super::super::anti_slop::shared::strip_parens;
use super::super::{Context, Rule};
use super::shared::{Member, Options, collapse_whitespace, expression, is_literal, is_test_file, parent_past_wrappers, text, truncate, unwrap};

pub const NAME: &str = "anti-slop-martinfowler/repeated-switches";

pub fn build(options: &[Value]) -> Result<Box<dyn Rule>, String> {
    let options = Options::parse(options, &["ignoreTests", "minOccurrences", "minSharedCases"])?;

    Ok(Box::new(RepeatedSwitches {
        ignore_tests: options.boolean("ignoreTests", true)?,
        min_occurrences: options.integer("minOccurrences", 2, 2)?,
        min_shared_cases: options.integer("minSharedCases", 1, 2)?,
    }))
}

struct RepeatedSwitches {
    ignore_tests: bool,
    min_occurrences: i64,
    min_shared_cases: i64,
}

/// One value compared against some cases.
struct Comparison {
    value: String,
    cases: Vec<String>,
}

struct Occurrence {
    span: Span,
    cases: FxHashSet<String>,
}

/// Every place each discriminant text is branched on, in first-seen order.
struct Occurrences<'s> {
    source: &'s str,
    index: FxHashMap<String, usize>,
    entries: Vec<(String, Vec<Occurrence>)>,
}

impl Occurrences<'_> {
    /// The source text of an ESTree node, whitespace collapsed.
    fn text(&self, span: Span) -> String {
        collapse_whitespace(text(self.source, span))
    }

    /// The text of an expression as ESTree has it: without parentheses.
    fn text_of(&self, expression: &Expression<'_>) -> String {
        self.text(strip_parens(expression).span())
    }

    fn record(&mut self, value: String, span: Span, cases: Vec<String>) {
        let occurrence = Occurrence { span, cases: cases.into_iter().collect() };

        if let Some(&index) = self.index.get(&value) {
            if let Some((_, entries)) = self.entries.get_mut(index) {
                entries.push(occurrence);
            }
        } else {
            self.index.insert(value.clone(), self.entries.len());
            self.entries.push((value, vec![occurrence]));
        }
    }

    fn compare(&self, node: &Expression<'_>) -> Option<Comparison> {
        match unwrap(expression(node)) {
            AstKind::LogicalExpression(logical) if logical.operator == LogicalOperator::Or => {
                let left = self.compare(&logical.left)?;
                let right = self.compare(&logical.right)?;

                if left.value != right.value {
                    return None;
                }

                let mut cases = left.cases;

                cases.extend(right.cases);
                Some(Comparison { value: left.value, cases })
            }
            AstKind::BinaryExpression(binary)
                if matches!(
                    binary.operator,
                    BinaryOperator::Equality | BinaryOperator::StrictEquality | BinaryOperator::Inequality | BinaryOperator::StrictInequality
                ) =>
            {
                let (left, right) = (is_case_value(&binary.left), is_case_value(&binary.right));

                if right && !left {
                    return Some(Comparison { value: self.text_of(&binary.left), cases: vec![self.text_of(&binary.right)] });
                }

                if left && !right {
                    return Some(Comparison { value: self.text_of(&binary.right), cases: vec![self.text_of(&binary.left)] });
                }

                None
            }
            _ => None,
        }
    }

    /// Record each else-if chain, and each run of adjacent guard `if`s, over
    /// one value.
    fn record_if_runs(&mut self, statements: &[Statement<'_>]) {
        let mut run: Option<(String, Span, Vec<String>)> = None;
        let mut run_length = 0;

        for statement in statements {
            let Statement::IfStatement(statement) = statement else {
                self.flush(&mut run, &mut run_length);
                continue;
            };

            let Some(first) = self.compare(&statement.test) else {
                self.flush(&mut run, &mut run_length);
                continue;
            };

            let mut cases = first.cases;
            let mut chain_length = 1;
            let mut alternate = statement.alternate.as_ref();

            while let Some(Statement::IfStatement(next_if)) = alternate {
                let Some(next) = self.compare(&next_if.test).filter(|next| next.value == first.value) else {
                    break;
                };

                cases.extend(next.cases);
                chain_length += 1;
                alternate = next_if.alternate.as_ref();
            }

            if chain_length >= 2 {
                self.flush(&mut run, &mut run_length);
                self.record(first.value, statement.span, cases);
                continue;
            }

            if statement.alternate.is_some() {
                self.flush(&mut run, &mut run_length);
                continue;
            }

            if run.as_ref().is_none_or(|(value, _, _)| *value != first.value) {
                self.flush(&mut run, &mut run_length);
                run = Some((first.value, statement.span, Vec::new()));
            }

            if let Some((_, _, run_cases)) = run.as_mut() {
                run_cases.extend(cases);
            }

            run_length += 1;
        }

        self.flush(&mut run, &mut run_length);
    }

    fn flush(&mut self, run: &mut Option<(String, Span, Vec<String>)>, run_length: &mut usize) {
        if let Some((value, start, cases)) = run.take()
            && *run_length >= 2
        {
            self.record(value, start, cases);
        }

        *run_length = 0;
    }
}

/// A literal, a template without substitutions, or an enum-like member such
/// as `Kind.Ready`.
fn is_case_value(node: &Expression<'_>) -> bool {
    let value = unwrap(expression(node));

    if is_literal(value) {
        return true;
    }

    if let AstKind::TemplateLiteral(template) = value {
        return template.expressions.is_empty();
    }

    Member::of(value).is_some_and(|member| {
        !member.is_computed()
            && matches!(strip_parens(member.object), Expression::Identifier(object) if object.name.starts_with(|c: char| c.is_ascii_uppercase()))
    })
}

impl Rule for RepeatedSwitches {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run_once(&self, ctx: &mut Context<'_, '_>) {
        if self.ignore_tests && is_test_file(ctx.rel) {
            return;
        }

        let semantic = ctx.semantic;
        let nodes = semantic.nodes();
        let mut occurrences = Occurrences { source: semantic.source_text(), index: FxHashMap::default(), entries: Vec::new() };

        for node in nodes.iter() {
            match node.kind() {
                AstKind::Program(program) => occurrences.record_if_runs(&program.body),
                AstKind::BlockStatement(block) => occurrences.record_if_runs(&block.body),
                AstKind::FunctionBody(body) => occurrences.record_if_runs(&body.statements),
                AstKind::SwitchCase(case) => occurrences.record_if_runs(&case.consequent),
                AstKind::SwitchStatement(switch) => {
                    let discriminant = unwrap(expression(&switch.discriminant));

                    if is_literal(discriminant) {
                        continue;
                    }

                    let cases = switch.cases.iter().filter_map(|case| case.test.as_ref().map(|test| occurrences.text_of(test))).collect();

                    occurrences.record(occurrences.text(discriminant.span()), switch.span, cases);
                }
                AstKind::ConditionalExpression(conditional) => {
                    if matches!(parent_past_wrappers(nodes, node.id()).kind(), AstKind::ConditionalExpression(_)) {
                        continue;
                    }

                    let Some(first) = occurrences.compare(&conditional.test) else {
                        continue;
                    };

                    let mut cases = first.cases;
                    let mut chain_length = 1;
                    let mut alternate = unwrap(expression(&conditional.alternate));

                    while let AstKind::ConditionalExpression(next_conditional) = alternate {
                        let Some(next) = occurrences.compare(&next_conditional.test).filter(|next| next.value == first.value) else {
                            break;
                        };

                        cases.extend(next.cases);
                        chain_length += 1;
                        alternate = unwrap(expression(&next_conditional.alternate));
                    }

                    if chain_length >= 2 {
                        occurrences.record(first.value, conditional.span, cases);
                    }
                }
                _ => {}
            }
        }

        let min_shared_cases = usize::try_from(self.min_shared_cases).unwrap_or(usize::MAX);
        let min_occurrences = usize::try_from(self.min_occurrences).unwrap_or(usize::MAX);

        for (expression, entries) in &occurrences.entries {
            let repeated: Vec<&Occurrence> = entries
                .iter()
                .enumerate()
                .filter(|(index, entry)| {
                    entries.iter().enumerate().any(|(other_index, other)| {
                        other_index != *index && entry.cases.iter().filter(|value| other.cases.contains(*value)).count() >= min_shared_cases
                    })
                })
                .map(|(_, entry)| entry)
                .collect();

            if repeated.len() < min_occurrences {
                continue;
            }

            let (snippet, count) = (truncate(expression, 60), repeated.len());

            for entry in repeated {
                ctx.report(
                    entry.span,
                    format!(
                        "Possibly Repeated Switches: this file branches on `{snippet}` over the same cases in {count} places, so adding a case means finding and editing every one of them. Consider Replace Conditional with Polymorphism: model the cases as a `Data.TaggedEnum` (or `Schema.TaggedUnion`) and handle them in one exhaustive `Match.type`/`Match.tags` … `Match.exhaustive`, or move each case's behavior onto its variant."
                    ),
                );
            }
        }
    }
}
