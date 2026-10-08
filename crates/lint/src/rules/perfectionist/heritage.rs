//! `sort-heritage-clauses`.

use oxc_ast::AstKind;
use oxc_ast::ast::TSTypeName;
use oxc_semantic::AstNode;
use oxc_span::Span;
use serde_json::{Value, json};

use super::options::{Options, Schema};
use super::report::{Messages, Problem, Report};
use super::sort::{Item, Mode, Sorter};
use super::{Base, File, emit, should_partition, size};
use crate::rules::{Context, Rule};

pub static SCHEMA: Schema = Schema {
    selectors: &[],
    modifiers: &[],
    match_keys: &[],
    sort_by: false,
    partition_by_comment: true,
    when_keys: &["matchesAstSelector"],
    extra_keys: &[],
    defaults,
};

const MESSAGES: Messages = Messages {
    order: "unexpectedHeritageClausesOrder",
    group_order: "unexpectedHeritageClausesGroupOrder",
    extra_spacing: "extraSpacingBetweenHeritageClauses",
    missed_spacing: "missedSpacingBetweenHeritageClauses",
    dependency_order: None,
};

fn defaults() -> Value {
    json!({
        "fallbackSort": {"type": "unsorted"},
        "newlinesInside": "newlinesBetween",
        "specialCharacters": "keep",
        "newlinesBetween": "ignore",
        "partitionByNewLine": false,
        "partitionByComment": false,
        "useConfigurationIf": {},
        "type": "alphabetical",
        "ignoreCase": true,
        "customGroups": [],
        "locales": "en-US",
        "alphabet": "",
        "order": "asc",
        "groups": [],
    })
}

pub struct SortHeritageClauses {
    pub base: Base,
}

impl Rule for SortHeritageClauses {
    fn name(&self) -> &'static str {
        "perfectionist/sort-heritage-clauses"
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let clauses: Vec<(Span, &TSTypeName<'a>)> = match node.kind() {
            AstKind::TSInterfaceDeclaration(declaration) => declaration.extends.iter().map(|clause| (clause.span, &clause.type_name)).collect(),
            AstKind::Class(class) if class.is_declaration() => class.implements.iter().map(|clause| (clause.span, &clause.expression)).collect(),
            _ => return,
        };

        if clauses.len() < 2 {
            return;
        }

        let Some(names) = clauses.iter().map(|(_, name)| last_name(name)).collect::<Option<Vec<String>>>() else { return };
        let nodes = ctx.semantic.nodes();
        let options = self
            .base
            .select(|options| options.when.all_names_match(&names) && options.when.selector.as_ref().is_none_or(|selector| selector.matches(nodes, node.id())));
        let file = File::new(ctx, &self.base.directive);
        let spans: Vec<Span> = clauses.iter().map(|(span, _)| *span).collect();
        let problems = sort(ctx.source(), &file, options, &spans, names);

        emit(ctx, problems);
    }
}

/// `computeNodeName`: the rightmost name of `A.B.C`.
fn last_name(name: &TSTypeName<'_>) -> Option<String> {
    match name {
        TSTypeName::IdentifierReference(identifier) => Some(identifier.name.to_string()),
        TSTypeName::QualifiedName(qualified) => Some(qualified.right.name.to_string()),
        TSTypeName::ThisExpression(_) => None,
    }
}

fn sort(text: &str, file: &File, options: &Options, spans: &[Span], names: Vec<String>) -> Vec<Problem> {
    let src = file.src(text);
    let mut items: Vec<Item> = Vec::with_capacity(spans.len());
    let mut partitions: Vec<Vec<usize>> = vec![Vec::new()];

    for (&span, name) in spans.iter().zip(names) {
        let group = options.compute_group(&[], |matcher| matcher.matches(&name, None, &[], &[]));
        let last = partitions.last().and_then(|partition| partition.last()).map(|&i| items[i].span);

        if should_partition(src, &options.partition_comment, options.partition_newline, last, span) {
            partitions.push(Vec::new());
        }

        if let Some(partition) = partitions.last_mut() {
            partition.push(items.len());
        }

        items.push(Item { span, size: size(src, span), disabled: file.is_disabled(src, span), group, name, ..Item::default() });
    }

    let sorter = Sorter { options, mode: Mode::Name };
    let report = Report { src, options, items: &items, messages: MESSAGES, partition_comment: &options.partition_comment };

    partitions
        .iter()
        .flat_map(|partition| {
            let ordered = sorter.sort_by_groups(&items, partition, false, |_, _| false);
            let excluding = sorter.sort_by_groups(&items, partition, true, |_, _| false);

            report.run(partition, &ordered, &excluding)
        })
        .collect()
}
