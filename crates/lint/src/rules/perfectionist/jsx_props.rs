//! `sort-jsx-props`.

use oxc_ast::AstKind;
use oxc_ast::ast::{JSXAttributeItem, JSXAttributeName, JSXElement};
use oxc_semantic::AstNode;
use oxc_span::GetSpan;
use serde_json::{Value, json};

use super::options::{Options, PartitionComment, Schema, predefined_groups};
use super::report::{Messages, Problem, Report};
use super::sort::{Item, Mode, Sorter};
use super::{Base, File, emit, should_partition, size};
use crate::rules::{Context, Rule};

pub static SCHEMA: Schema = Schema {
    selectors: &["prop"],
    modifiers: &["shorthand", "multiline"],
    match_keys: &["elementValuePattern", "selector", "modifiers"],
    sort_by: false,
    partition_by_comment: false,
    when_keys: &["matchesAstSelector", "tagMatchesPattern"],
    extra_keys: &[],
    defaults,
};

const MESSAGES: Messages = Messages {
    order: "unexpectedJSXPropsOrder",
    group_order: "unexpectedJSXPropsGroupOrder",
    extra_spacing: "extraSpacingBetweenJSXPropsMembers",
    missed_spacing: "missedSpacingBetweenJSXPropsMembers",
    dependency_order: None,
};

fn defaults() -> Value {
    json!({
        "fallbackSort": {"type": "unsorted"},
        "newlinesInside": "newlinesBetween",
        "specialCharacters": "keep",
        "newlinesBetween": "ignore",
        "partitionByNewLine": false,
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

pub struct SortJsxProps {
    pub base: Base,
}

impl Rule for SortJsxProps {
    fn name(&self) -> &'static str {
        "perfectionist/sort-jsx-props"
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let AstKind::JSXElement(element) = node.kind() else { return };

        if element.opening_element.attributes.len() < 2 {
            return;
        }

        let source = ctx.source();
        let nodes = ctx.semantic.nodes();
        let names: Vec<String> = element
            .opening_element
            .attributes
            .iter()
            .filter_map(|attribute| match attribute {
                JSXAttributeItem::Attribute(attribute) => Some(attribute_name(&attribute.name)),
                JSXAttributeItem::SpreadAttribute(_) => None,
            })
            .collect();
        let tag = source.get(element.opening_element.name.span().start as usize..element.opening_element.name.span().end as usize).unwrap_or_default();
        let options = self.base.select(|options| {
            options.when.all_names_match(&names)
                && options.when.tag.as_ref().is_none_or(|pattern| pattern.matches(tag))
                && options.when.selector.as_ref().is_none_or(|selector| selector.matches(nodes, node.id()))
        });
        let file = File::new(ctx, &self.base.directive);
        let problems = sort(source, &file, options, element);

        emit(ctx, problems);
    }
}

/// `computeNodeName`.
fn attribute_name(name: &JSXAttributeName<'_>) -> String {
    match name {
        JSXAttributeName::Identifier(identifier) => identifier.name.to_string(),
        JSXAttributeName::NamespacedName(name) => format!("{}:{}", name.namespace.name, name.name.name),
    }
}

fn sort(text: &str, file: &File, options: &Options, element: &JSXElement<'_>) -> Vec<Problem> {
    let src = file.src(text);
    let mut items: Vec<Item> = Vec::new();
    let mut partitions: Vec<Vec<usize>> = vec![Vec::new()];

    for attribute in &element.opening_element.attributes {
        let JSXAttributeItem::Attribute(attribute) = attribute else {
            partitions.push(Vec::new());
            continue;
        };
        let name = attribute_name(&attribute.name);
        let span = attribute.span;
        let mut modifiers = Vec::new();

        if attribute.value.is_none() {
            modifiers.push("shorthand");
        }

        if src.line(span.start) != src.line(span.end) {
            modifiers.push("multiline");
        }

        let value = attribute.value.as_ref().map(|value| src.text_of(value.span()));
        let group = options.compute_group(&predefined_groups(&["prop"], &modifiers), |matcher| matcher.matches(&name, value, &["prop"], &modifiers));
        let last = partitions.last().and_then(|partition| partition.last()).map(|&i| items[i].span);

        if should_partition(src, &PartitionComment::Off, options.partition_newline, last, span) {
            partitions.push(Vec::new());
        }

        if let Some(partition) = partitions.last_mut() {
            partition.push(items.len());
        }

        items.push(Item { span, size: size(src, span), disabled: file.is_disabled(src, span), group, name, partition: partitions.len(), ..Item::default() });
    }

    let sorter = Sorter { options, mode: Mode::Name };
    let report = Report { src, options, items: &items, messages: MESSAGES, partition_comment: &PartitionComment::Off };

    partitions
        .iter()
        .flat_map(|partition| {
            let ordered = sorter.sort_by_groups(&items, partition, false, |_, _| false);
            let excluding = sorter.sort_by_groups(&items, partition, true, |_, _| false);

            report.run(partition, &ordered, &excluding)
        })
        .collect()
}
