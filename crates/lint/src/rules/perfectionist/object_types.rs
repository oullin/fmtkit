//! `sort-interfaces` and `sort-object-types`: one sorter over type members.

use oxc_ast::AstKind;
use oxc_ast::ast::{PropertyKey, TSSignature, TSType};
use oxc_semantic::{AstNode, AstNodes, NodeId};
use oxc_span::GetSpan;
use serde_json::{Value, json};

use super::options::{By, Options, Schema, predefined_groups};
use super::report::{Messages, Problem, Report};
use super::sort::{Item, Mode, Sorter};
use super::source::{Src, js_trim};
use super::{Base, File, emit, estree, key_name, should_partition, size};
use crate::rules::{Context, Rule};

pub static SCHEMA: Schema = Schema {
    selectors: &["index-signature", "member", "method", "property"],
    modifiers: &["optional", "required", "multiline"],
    match_keys: &["elementValuePattern", "selector", "modifiers"],
    sort_by: true,
    partition_by_comment: true,
    when_keys: &["hasNumericKeysOnly", "declarationCommentMatchesPattern", "matchesAstSelector", "declarationMatchesPattern"],
    extra_keys: &[],
    defaults,
};

const INTERFACE_MESSAGES: Messages = Messages {
    order: "unexpectedInterfacePropertiesOrder",
    group_order: "unexpectedInterfacePropertiesGroupOrder",
    extra_spacing: "extraSpacingBetweenInterfaceMembers",
    missed_spacing: "missedSpacingBetweenInterfaceMembers",
    dependency_order: None,
};

const OBJECT_TYPE_MESSAGES: Messages = Messages {
    order: "unexpectedObjectTypesOrder",
    group_order: "unexpectedObjectTypesGroupOrder",
    extra_spacing: "extraSpacingBetweenObjectTypeMembers",
    missed_spacing: "missedSpacingBetweenObjectTypeMembers",
    dependency_order: None,
};

fn defaults() -> Value {
    json!({
        "fallbackSort": {"type": "unsorted", "sortBy": "name"},
        "newlinesInside": "newlinesBetween",
        "partitionByComment": false,
        "partitionByNewLine": false,
        "newlinesBetween": "ignore",
        "specialCharacters": "keep",
        "useConfigurationIf": {},
        "type": "alphabetical",
        "ignoreCase": true,
        "customGroups": [],
        "locales": "en-US",
        "sortBy": "name",
        "alphabet": "",
        "order": "asc",
        "groups": [],
    })
}

pub struct SortObjectTypes {
    /// `sort-interfaces` rather than `sort-object-types`.
    pub interfaces: bool,
    pub base: Base,
}

impl Rule for SortObjectTypes {
    fn name(&self) -> &'static str {
        if self.interfaces { "perfectionist/sort-interfaces" } else { "perfectionist/sort-object-types" }
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let nodes = ctx.semantic.nodes();
        let (elements, parents) = match node.kind() {
            AstKind::TSInterfaceDeclaration(declaration) if self.interfaces => (&declaration.body.body, vec![node.id()]),
            AstKind::TSTypeLiteral(literal) if !self.interfaces => (&literal.members, parents(nodes, node.id())),
            _ => return,
        };

        if elements.len() < 2 {
            return;
        }

        let source = ctx.source();
        let file = File::new(ctx, &self.base.directive);
        let src = file.src(source);
        let names: Vec<String> = elements.iter().map(|element| element_name(src, element)).collect();
        let options = self.base.select(|options| {
            let when = &options.when;

            when.all_names_match(&names)
                && when.declaration.as_ref().is_none_or(|pattern| pattern.matches_scoped(&parents, |_| true, |&parent| vec![parent_name(src, nodes, parent)]))
                && when.numeric_keys_only.is_none_or(|wanted| wanted == numeric_keys_only(elements))
                && when
                    .declaration_comment
                    .as_ref()
                    .is_none_or(|pattern| pattern.matches_scoped(&parents, |_| true, |&parent| declaration_comments(src, nodes, parent)))
                && when.selector.as_ref().is_none_or(|selector| selector.matches(nodes, node.id()))
        });
        let messages = if self.interfaces { INTERFACE_MESSAGES } else { OBJECT_TYPE_MESSAGES };
        let problems = sort(src, &file, options, elements, messages);

        emit(ctx, problems);
    }
}

/// The ESTree ancestors a type literal's options can match against.
fn parents(nodes: &AstNodes<'_>, id: NodeId) -> Vec<NodeId> {
    estree::ancestors(nodes, id)
        .filter(|&ancestor| {
            matches!(
                nodes.kind(ancestor),
                AstKind::TSTypeAliasDeclaration(_)
                    | AstKind::TSInterfaceDeclaration(_)
                    | AstKind::TSPropertySignature(_)
                    | AstKind::VariableDeclarator(_)
                    | AstKind::PropertyDefinition(_)
            )
        })
        .collect()
}

/// `computeNodeParentName`.
fn parent_name(src: Src<'_>, nodes: &AstNodes<'_>, id: NodeId) -> String {
    let key_text = |key: &PropertyKey<'_>| key_name(key).unwrap_or_else(|| src.text_of(key.span()).to_owned());

    match nodes.kind(id) {
        AstKind::TSTypeAliasDeclaration(declaration) => declaration.id.name.to_string(),
        AstKind::TSInterfaceDeclaration(declaration) => declaration.id.name.to_string(),
        AstKind::TSPropertySignature(signature) => key_text(&signature.key),
        AstKind::PropertyDefinition(definition) => key_text(&definition.key),
        AstKind::VariableDeclarator(declarator) => {
            let id = declarator.id.span();

            match &declarator.type_annotation {
                Some(annotation) if annotation.span.start >= id.start => src.slice(id.start, annotation.span.start).to_owned(),
                _ => src.text_of(id).to_owned(),
            }
        }
        _ => String::new(),
    }
}

/// The comments before the declaration a parent belongs to, trimmed.
fn declaration_comments(src: Src<'_>, nodes: &AstNodes<'_>, id: NodeId) -> Vec<String> {
    let target = match nodes.kind(id) {
        AstKind::VariableDeclarator(_) => estree::parent(nodes, id).unwrap_or(id),
        _ => id,
    };

    src.comments_before(nodes.kind(target).span().start).into_iter().map(|comment| js_trim(src.comment_value(comment)).to_owned()).collect()
}

fn numeric_keys_only(elements: &[TSSignature<'_>]) -> bool {
    elements.iter().all(|element| matches!(element, TSSignature::TSPropertySignature(signature) if matches!(signature.key, PropertyKey::NumericLiteral(_))))
}

/// `formatName`: drops one trailing `,` or `;`.
fn format_name(text: &str) -> String {
    text.strip_suffix([',', ';']).unwrap_or(text).to_owned()
}

/// `computeNodeName`.
fn element_name(src: Src<'_>, element: &TSSignature<'_>) -> String {
    match element {
        TSSignature::TSCallSignatureDeclaration(_) | TSSignature::TSConstructSignatureDeclaration(_) => format_name(src.text_of(element.span())),
        TSSignature::TSPropertySignature(signature) => key_name(&signature.key).unwrap_or_else(|| {
            let end = signature.type_annotation.as_ref().map_or(signature.span.end - u32::from(signature.optional), |annotation| annotation.span.start);

            src.slice(signature.span.start, end).to_owned()
        }),
        TSSignature::TSMethodSignature(signature) => match &signature.key {
            PropertyKey::StaticIdentifier(identifier) => identifier.name.to_string(),
            PropertyKey::PrivateIdentifier(identifier) => identifier.name.to_string(),
            PropertyKey::Identifier(identifier) => identifier.name.to_string(),
            _ => format_name(src.text_of(signature.span)),
        },
        TSSignature::TSIndexSignature(signature) => format_name(src.slice(signature.span.start, signature.type_annotation.span.start)),
    }
}

/// `isNodeFunctionType`.
fn is_function_type(element: &TSSignature<'_>) -> bool {
    match element {
        TSSignature::TSMethodSignature(_) => true,
        TSSignature::TSPropertySignature(signature) => signature.type_annotation.as_ref().is_some_and(|annotation| is_function(&annotation.type_annotation)),
        _ => false,
    }
}

fn is_function(ty: &TSType<'_>) -> bool {
    match ty {
        TSType::TSParenthesizedType(parenthesized) => is_function(&parenthesized.type_annotation),
        TSType::TSFunctionType(_) => true,
        TSType::TSUnionType(union) => union.types.iter().all(is_function),
        TSType::TSIntersectionType(intersection) => intersection.types.iter().all(is_function),
        _ => false,
    }
}

fn sort(src: Src<'_>, file: &File, options: &Options, elements: &[TSSignature<'_>], messages: Messages) -> Vec<Problem> {
    let mut items: Vec<Item> = Vec::new();
    let mut partitions: Vec<Vec<usize>> = vec![Vec::new()];

    for element in elements {
        if matches!(element, TSSignature::TSCallSignatureDeclaration(_) | TSSignature::TSConstructSignatureDeclaration(_)) {
            partitions.push(Vec::new());
            continue;
        }

        let span = element.span();
        let last = partitions.last().and_then(|partition| partition.last()).map(|&i| items[i].span);
        let mut selectors = Vec::new();
        let mut modifiers = Vec::new();

        if matches!(element, TSSignature::TSIndexSignature(_)) {
            selectors.push("index-signature");
        }

        if is_function_type(element) {
            selectors.push("method");
        }

        if src.line(span.start) != src.line(span.end) {
            modifiers.push("multiline");
        }

        if !selectors.contains(&"index-signature") && !selectors.contains(&"method") {
            selectors.push("property");
        }

        selectors.push("member");

        let optional = match element {
            TSSignature::TSPropertySignature(signature) => signature.optional,
            TSSignature::TSMethodSignature(signature) => signature.optional,
            _ => false,
        };

        modifiers.push(if optional { "optional" } else { "required" });

        let name = element_name(src, element);
        let value = match element {
            TSSignature::TSPropertySignature(signature) => signature.type_annotation.as_ref().map(|annotation| src.text_of(annotation.type_annotation.span())),
            _ => None,
        };
        let group = options.compute_group(&predefined_groups(&selectors, &modifiers), |matcher| matcher.matches(&name, value, &selectors, &modifiers));

        if should_partition(src, &options.partition_comment, options.partition_newline, last, span) {
            partitions.push(Vec::new());
        }

        if let Some(partition) = partitions.last_mut() {
            partition.push(items.len());
        }

        items.push(Item {
            span,
            name,
            value: value.unwrap_or_default().to_owned(),
            size: size(src, span),
            group,
            partition: partitions.len(),
            disabled: file.is_disabled(src, span),
            semicolon: true,
            ..Item::default()
        });
    }

    let sorter = Sorter { options, mode: Mode::NameOrValue };
    let ordering = |ignore_disabled: bool| -> Vec<usize> {
        partitions
            .iter()
            .flat_map(|partition| sorter.sort_by_groups(&items, partition, ignore_disabled, |spec, item| spec.sort.by == By::Value && item.value.is_empty()))
            .collect()
    };
    let all: Vec<usize> = (0..items.len()).collect();
    let report = Report { src, options, items: &items, messages, partition_comment: &options.partition_comment };

    report.run(&all, &ordering(false), &ordering(true))
}
