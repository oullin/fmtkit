//! `sort-objects`: object literals and object patterns.

use oxc_ast::AstKind;
use oxc_ast::ast::{
    Argument, ArrayExpressionElement, AssignmentTarget, AssignmentTargetMaybeDefault, AssignmentTargetProperty, BindingPattern, ChainElement, Expression,
    ObjectPropertyKind, PropertyKey, SimpleAssignmentTarget,
};
use oxc_semantic::{AstNode, AstNodes, NodeId};
use oxc_span::{GetSpan, Span};
use rustc_hash::FxHashSet;
use serde_json::{Value, json};

use super::estree::{self, unparenthesized};
use super::options::{ObjectType, Options, Schema, predefined_groups};
use super::regex::RegexOption;
use super::report::{Messages, Problem, Report};
use super::sort::{Item, Mode, Sorter, sort_by_dependencies};
use super::source::{Src, js_trim};
use super::{Base, File, deps, emit, key_name, should_partition, size, subtree};
use crate::rules::{Context, Rule};

pub static SCHEMA: Schema = Schema {
    selectors: &["member", "method", "property"],
    modifiers: &["multiline"],
    match_keys: &["elementValuePattern", "selector", "modifiers"],
    sort_by: true,
    partition_by_comment: true,
    when_keys: &[
        "objectType",
        "hasNumericKeysOnly",
        "declarationCommentMatchesPattern",
        "callingFunctionNamePattern",
        "matchesAstSelector",
        "declarationMatchesPattern",
    ],
    extra_keys: &["partitionByComputedKey", "styledComponents", "useExperimentalDependencyDetection", "ignoreCallbackDependenciesPatterns"],
    defaults,
};

const MESSAGES: Messages = Messages {
    order: "unexpectedObjectsOrder",
    group_order: "unexpectedObjectsGroupOrder",
    extra_spacing: "extraSpacingBetweenObjectMembers",
    missed_spacing: "missedSpacingBetweenObjectMembers",
    dependency_order: Some("unexpectedObjectsDependencyOrder"),
};

fn defaults() -> Value {
    json!({
        "useExperimentalDependencyDetection": true,
        "ignoreCallbackDependenciesPatterns": [],
        "fallbackSort": {"type": "unsorted"},
        "newlinesInside": "newlinesBetween",
        "partitionByComputedKey": false,
        "partitionByNewLine": false,
        "partitionByComment": false,
        "newlinesBetween": "ignore",
        "specialCharacters": "keep",
        "styledComponents": true,
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

pub struct SortObjects {
    pub base: Base,
}

/// One ESTree `Property` of the object, or a spread or rest element.
enum Entry<'b, 'a> {
    Break,
    Property(Property<'b, 'a>),
}

struct Property<'b, 'a> {
    span: Span,
    computed: bool,
    name: String,
    /// The value is a function: the `method` selector.
    method: bool,
    /// `computeNodeValue`.
    value: Option<&'b str>,
    /// The default of an `AssignmentPattern` value, for the legacy detection.
    default: Option<&'b Expression<'a>>,
    /// For patterns: the names the property binds.
    bound: Vec<String>,
    numeric_key: bool,
}

impl Rule for SortObjects {
    fn name(&self) -> &'static str {
        "perfectionist/sort-objects"
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let source = ctx.source();
        let nodes = ctx.semantic.nodes();
        let (entries, destructured) = match node.kind() {
            AstKind::ObjectExpression(object) => (expression_entries(source, &object.properties), false),
            AstKind::ObjectPattern(pattern) => {
                let mut entries = pattern_entries(source, &pattern.properties);

                if pattern.rest.is_some() {
                    entries.push(Entry::Break);
                }

                (entries, true)
            }
            AstKind::ObjectAssignmentTarget(target) => {
                let mut entries = target_entries(source, &target.properties);

                if target.rest.is_some() {
                    entries.push(Entry::Break);
                }

                (entries, true)
            }
            _ => return,
        };

        if entries.len() < 2 {
            return;
        }

        let file = File::new(ctx, &self.base.directive);
        let src = file.src(source);
        let names: Vec<&str> = entries
            .iter()
            .filter_map(|entry| match entry {
                Entry::Property(property) => Some(property.name.as_str()),
                Entry::Break => None,
            })
            .collect();
        let parents = parents(nodes, node.id());
        let numeric_keys_only = !destructured && entries.iter().all(|entry| matches!(entry, Entry::Property(property) if property.numeric_key));
        let options = self.base.select(|options| {
            let when = &options.when;
            let calls = |&id: &NodeId| matches!(nodes.kind(id), AstKind::CallExpression(_));
            let declarations = |&id: &NodeId| !matches!(nodes.kind(id), AstKind::CallExpression(_));

            when.all_names_match(&names)
                && when.object_type.is_none_or(|wanted| (wanted == ObjectType::Destructured) == destructured)
                && when.calling_function.as_ref().is_none_or(|pattern| pattern.matches_scoped(&parents, calls, |&id| vec![callee_text(src, nodes, id)]))
                && when.declaration.as_ref().is_none_or(|pattern| pattern.matches_scoped(&parents, declarations, |&id| vec![declaration_name(src, nodes, id)]))
                && when.numeric_keys_only.is_none_or(|wanted| wanted == numeric_keys_only)
                && when
                    .declaration_comment
                    .as_ref()
                    .is_none_or(|pattern| pattern.matches_scoped(&parents, |_| true, |&id| declaration_comments(src, nodes, id)))
                && when.selector.as_ref().is_none_or(|selector| selector.matches(nodes, node.id()))
        });

        if !options.styled_components && !destructured && is_style_component(nodes, node.id()) {
            return;
        }

        let problems = sort(ctx, &file, options, node.id(), entries, destructured);

        emit(ctx, problems);
    }
}

fn sort<'a>(ctx: &Context<'_, 'a>, file: &File, options: &Options, object: NodeId, entries: Vec<Entry<'_, 'a>>, destructured: bool) -> Vec<Problem> {
    let src = file.src(ctx.source());
    let nodes = ctx.semantic.nodes();
    let mut items: Vec<Item> = Vec::new();
    let mut defaults = Vec::new();
    let mut partitions: Vec<Vec<usize>> = vec![Vec::new()];

    for entry in entries {
        let property = match entry {
            Entry::Property(property) if destructured || !options.partition_by_computed_key || !property.computed => property,
            _ => {
                partitions.push(Vec::new());
                continue;
            }
        };
        let span = property.span;
        let last = partitions.last().and_then(|partition| partition.last()).map(|&i| items[i].span);
        let selectors: [&str; 2] = [if property.method { "method" } else { "property" }, "member"];
        let modifiers: Vec<&str> = if src.line(span.start) == src.line(span.end) { Vec::new() } else { vec!["multiline"] };
        let group = options
            .compute_group(&predefined_groups(&selectors, &modifiers), |matcher| matcher.matches(&property.name, property.value, &selectors, &modifiers));
        let dependency_names = if destructured { unique(property.bound) } else { vec![property.name.clone()] };
        let mut dependencies = Vec::new();

        if !options.experimental
            && let Some(default) = property.default
        {
            legacy_dependencies(default, &mut dependencies);
        }

        if should_partition(src, &options.partition_comment, options.partition_newline, last, span) {
            partitions.push(Vec::new());
        }

        if let Some(partition) = partitions.last_mut() {
            partition.push(items.len());
        }

        defaults.push(property.default.is_some());
        items.push(Item {
            span,
            name: property.name,
            value: property.value.unwrap_or_default().to_owned(),
            size: size(src, span),
            group,
            partition: partitions.len(),
            disabled: file.is_disabled(src, span),
            dependency_names,
            dependencies,
            ..Item::default()
        });
    }

    if options.experimental {
        let spans: Vec<Span> = items.iter().map(|item| item.span).collect();
        let candidates = subtree(nodes, object);
        let ignore = |reference: &deps::Reference<'_>| deferred(nodes, reference.node, spans[reference.referencing], options.ignore_callback.as_ref());
        let referenced = deps::referenced(ctx.semantic, &candidates, &spans, ignore, |_| Vec::new());

        deps::apply(&mut items, &referenced);
    }

    let sorter = Sorter { options, mode: Mode::NameOrValue };
    let ordering = |ignore_disabled: bool| {
        let by_groups: Vec<usize> = partitions.iter().flat_map(|partition| sorter.sort_by_groups(&items, partition, ignore_disabled, |_, _| false)).collect();

        sort_by_dependencies(&items, &by_groups, ignore_disabled)
    };
    let all: Vec<usize> = (0..items.len()).collect();
    let report = Report { src, options, items: &items, messages: MESSAGES, partition_comment: &options.partition_comment };

    report.run(&all, &ordering(false), &ordering(true))
}

fn unique(names: Vec<String>) -> Vec<String> {
    let mut seen = FxHashSet::default();

    names.into_iter().filter(|name| seen.insert(name.clone())).collect()
}

fn is_function(expression: &Expression<'_>) -> bool {
    matches!(unparenthesized(expression), Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_))
}

/// `computeIdentifierName` for a property key.
fn key_text(source: &str, key: &PropertyKey<'_>) -> String {
    key_name(key).unwrap_or_else(|| text(source, key.span()).to_owned())
}

fn text(source: &str, span: Span) -> &str {
    source.get(span.start as usize..span.end as usize).unwrap_or_default()
}

/// `computeNodeValue` for an `AssignmentPattern` value's default.
fn default_value<'b>(source: &'b str, default: &Expression<'_>) -> Option<&'b str> {
    (!is_function(default)).then(|| text(source, unparenthesized(default).span()))
}

fn expression_entries<'b, 'a>(source: &'b str, properties: &'b [ObjectPropertyKind<'a>]) -> Vec<Entry<'b, 'a>> {
    properties
        .iter()
        .map(|property| match property {
            ObjectPropertyKind::SpreadProperty(_) => Entry::Break,
            ObjectPropertyKind::ObjectProperty(property) => {
                let method = is_function(&property.value);

                Entry::Property(Property {
                    span: property.span,
                    computed: property.computed,
                    name: key_text(source, &property.key),
                    method,
                    value: (!method).then(|| text(source, unparenthesized(&property.value).span())),
                    default: None,
                    bound: Vec::new(),
                    numeric_key: matches!(property.key, PropertyKey::NumericLiteral(_)),
                })
            }
        })
        .collect()
}

fn pattern_entries<'b, 'a>(source: &'b str, properties: &'b [oxc_ast::ast::BindingProperty<'a>]) -> Vec<Entry<'b, 'a>> {
    properties
        .iter()
        .map(|property| {
            let default = match &property.value {
                BindingPattern::AssignmentPattern(pattern) => Some(&pattern.right),
                _ => None,
            };
            let mut bound = Vec::new();

            binding_names(&property.value, &mut bound);

            Entry::Property(Property {
                span: property.span,
                computed: property.computed,
                name: key_text(source, &property.key),
                method: false,
                value: default.and_then(|default| default_value(source, default)),
                default,
                bound,
                numeric_key: false,
            })
        })
        .collect()
}

fn target_entries<'b, 'a>(source: &'b str, properties: &'b [AssignmentTargetProperty<'a>]) -> Vec<Entry<'b, 'a>> {
    properties
        .iter()
        .map(|property| match property {
            AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(property) => {
                let default = property.init.as_ref();

                Entry::Property(Property {
                    span: property.span,
                    computed: false,
                    name: property.binding.name.to_string(),
                    method: false,
                    value: default.and_then(|default| default_value(source, default)),
                    default,
                    bound: vec![property.binding.name.to_string()],
                    numeric_key: false,
                })
            }
            AssignmentTargetProperty::AssignmentTargetPropertyProperty(property) => {
                let (default, target) = match &property.binding {
                    AssignmentTargetMaybeDefault::AssignmentTargetWithDefault(with) => (Some(&with.init), Some(&with.binding)),
                    other => (None, other.as_assignment_target()),
                };
                let mut bound = Vec::new();

                if let Some(target) = target {
                    target_names(target, &mut bound);
                }

                Entry::Property(Property {
                    span: property.span,
                    computed: property.computed,
                    name: key_text(source, &property.name),
                    method: false,
                    value: default.and_then(|default| default_value(source, default)),
                    default,
                    bound,
                    numeric_key: false,
                })
            }
        })
        .collect()
}

/// `computeDependencyNames` for a binding pattern.
fn binding_names(pattern: &BindingPattern<'_>, out: &mut Vec<String>) {
    match pattern {
        BindingPattern::BindingIdentifier(identifier) => out.push(identifier.name.to_string()),
        BindingPattern::AssignmentPattern(pattern) => binding_names(&pattern.left, out),
        BindingPattern::ObjectPattern(pattern) => {
            for property in &pattern.properties {
                binding_names(&property.value, out);
            }

            if let Some(rest) = &pattern.rest {
                binding_names(&rest.argument, out);
            }
        }
        BindingPattern::ArrayPattern(pattern) => {
            for element in pattern.elements.iter().flatten() {
                binding_names(element, out);
            }

            if let Some(rest) = &pattern.rest {
                binding_names(&rest.argument, out);
            }
        }
    }
}

/// `computeDependencyNames` for an assignment target.
fn target_names(target: &AssignmentTarget<'_>, out: &mut Vec<String>) {
    let maybe_default = |target: &AssignmentTargetMaybeDefault<'_>, out: &mut Vec<String>| match target {
        AssignmentTargetMaybeDefault::AssignmentTargetWithDefault(with) => target_names(&with.binding, out),
        other => {
            if let Some(target) = other.as_assignment_target() {
                target_names(target, out);
            }
        }
    };

    match target {
        AssignmentTarget::AssignmentTargetIdentifier(identifier) => out.push(identifier.name.to_string()),
        AssignmentTarget::ObjectAssignmentTarget(object) => {
            for property in &object.properties {
                match property {
                    AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(property) => out.push(property.binding.name.to_string()),
                    AssignmentTargetProperty::AssignmentTargetPropertyProperty(property) => maybe_default(&property.binding, out),
                }
            }

            if let Some(rest) = &object.rest {
                target_names(&rest.target, out);
            }
        }
        AssignmentTarget::ArrayAssignmentTarget(array) => {
            for element in array.elements.iter().flatten() {
                maybe_default(element, out);
            }

            if let Some(rest) = &array.rest {
                target_names(&rest.target, out);
            }
        }
        _ => {}
    }
}

/// The ESTree `VariableDeclarator`, `CallExpression` and `Property` ancestors.
fn parents(nodes: &AstNodes<'_>, id: NodeId) -> Vec<NodeId> {
    estree::ancestors(nodes, id)
        .filter(|&ancestor| matches!(nodes.kind(ancestor), AstKind::VariableDeclarator(_) | AstKind::CallExpression(_)) || is_property(nodes.kind(ancestor)))
        .collect()
}

fn is_property(kind: AstKind<'_>) -> bool {
    matches!(
        kind,
        AstKind::ObjectProperty(_)
            | AstKind::BindingProperty(_)
            | AstKind::AssignmentTargetPropertyIdentifier(_)
            | AstKind::AssignmentTargetPropertyProperty(_)
    )
}

fn callee_text(src: Src<'_>, nodes: &AstNodes<'_>, id: NodeId) -> String {
    match nodes.kind(id) {
        AstKind::CallExpression(call) => src.text_of(unparenthesized(&call.callee).span()).to_owned(),
        _ => String::new(),
    }
}

/// `computePropertyOrVariableDeclaratorName`.
fn declaration_name(src: Src<'_>, nodes: &AstNodes<'_>, id: NodeId) -> String {
    match nodes.kind(id) {
        AstKind::VariableDeclarator(declarator) => match &declarator.id {
            BindingPattern::BindingIdentifier(identifier) => identifier.name.to_string(),
            pattern => src.text_of(pattern.span()).to_owned(),
        },
        AstKind::ObjectProperty(property) => key_text(src.text, &property.key),
        AstKind::BindingProperty(property) => key_text(src.text, &property.key),
        AstKind::AssignmentTargetPropertyIdentifier(property) => property.binding.name.to_string(),
        AstKind::AssignmentTargetPropertyProperty(property) => key_text(src.text, &property.name),
        _ => String::new(),
    }
}

/// The comments before the statement a parent belongs to, trimmed.
fn declaration_comments(src: Src<'_>, nodes: &AstNodes<'_>, id: NodeId) -> Vec<String> {
    let mut target = id;

    if matches!(nodes.kind(id), AstKind::VariableDeclarator(_)) {
        target = estree::parent(nodes, id).unwrap_or(id);

        if let Some(export) = estree::parent(nodes, target).filter(|&parent| estree::is_type(nodes, parent, "ExportNamedDeclaration")) {
            target = export;
        }
    }

    src.comments_before(nodes.kind(target).span().start).into_iter().map(|comment| js_trim(src.comment_value(comment)).to_owned()).collect()
}

/// `isNodeInsideDeferredFunction`, with the sorting node at `property`.
fn deferred(nodes: &AstNodes<'_>, identifier: NodeId, property: Span, ignore_callback: Option<&RegexOption>) -> bool {
    let mut functions = Vec::new();
    let mut calls = Vec::new();

    for ancestor in estree::ancestors(nodes, identifier) {
        let kind = nodes.kind(ancestor);

        if is_property(kind) && kind.span() == property {
            break;
        }

        match kind {
            AstKind::Function(function) if function.is_expression() => functions.push(ancestor),
            AstKind::ArrowFunctionExpression(_) => functions.push(ancestor),
            AstKind::CallExpression(call) => calls.push(&call.callee),
            _ => {}
        }
    }

    let skipped = ignore_callback.is_some_and(|pattern| {
        calls.iter().any(|callee| matches!(unparenthesized(callee), Expression::Identifier(identifier) if pattern.matches(&identifier.name)))
    });

    skipped
        || functions.into_iter().any(|function| {
            !estree::is_callee(nodes, function)
                && !estree::parent(nodes, function).is_some_and(|parent| matches!(nodes.kind(parent), AstKind::CallExpression(_)))
        })
}

/// `isStyleComponent`.
fn is_style_component(nodes: &AstNodes<'_>, id: NodeId) -> bool {
    let mut root = id;

    while let Some(property) = estree::parent(nodes, root).filter(|&parent| matches!(nodes.kind(parent), AstKind::ObjectProperty(_)))
        && let Some(object) = estree::parent(nodes, property).filter(|&parent| matches!(nodes.kind(parent), AstKind::ObjectExpression(_)))
    {
        root = object;
    }

    let Some(parent) = estree::parent(nodes, root) else { return false };

    is_style_node(nodes, parent)
        || (matches!(nodes.kind(parent), AstKind::ArrowFunctionExpression(_)) && estree::parent(nodes, parent).is_some_and(|grand| is_style_node(nodes, grand)))
}

fn is_style_node(nodes: &AstNodes<'_>, id: NodeId) -> bool {
    let named = |expression: &Expression<'_>, name: &str| matches!(unparenthesized(expression), Expression::Identifier(identifier) if identifier.name == name);

    match nodes.kind(id) {
        AstKind::JSXExpressionContainer(_) => estree::parent(nodes, id).is_some_and(|parent| {
            matches!(nodes.kind(parent), AstKind::JSXAttribute(attribute) if matches!(&attribute.name, oxc_ast::ast::JSXAttributeName::Identifier(name) if name.name == "style"))
        }),
        AstKind::CallExpression(call) => match unparenthesized(&call.callee) {
            callee @ Expression::Identifier(_) => named(callee, "css"),
            Expression::StaticMemberExpression(member) => named(&member.object, "styled"),
            Expression::ComputedMemberExpression(member) => named(&member.object, "styled"),
            Expression::PrivateFieldExpression(member) => named(&member.object, "styled"),
            Expression::CallExpression(inner) => named(&inner.callee, "styled"),
            _ => false,
        },
        _ => false,
    }
}

/// The legacy `computeDependencies`: identifiers in a pattern's default,
/// outside nested functions.
fn legacy_dependencies(expression: &Expression<'_>, out: &mut Vec<String>) {
    match unparenthesized(expression) {
        Expression::Identifier(identifier) => out.push(identifier.name.to_string()),
        Expression::ConditionalExpression(conditional) => {
            legacy_dependencies(&conditional.test, out);
            legacy_dependencies(&conditional.consequent, out);
            legacy_dependencies(&conditional.alternate, out);
        }
        Expression::ChainExpression(chain) => match &chain.expression {
            ChainElement::CallExpression(call) => {
                legacy_dependencies(&call.callee, out);
                arguments(&call.arguments, out);
            }
            ChainElement::TSNonNullExpression(expression) => legacy_dependencies(&expression.expression, out),
            element => {
                if let Some(member) = element.as_member_expression() {
                    legacy_dependencies(member.object(), out);
                }
            }
        },
        Expression::TSAsExpression(expression) => legacy_dependencies(&expression.expression, out),
        Expression::TSSatisfiesExpression(expression) => legacy_dependencies(&expression.expression, out),
        Expression::TSNonNullExpression(expression) => legacy_dependencies(&expression.expression, out),
        Expression::TSTypeAssertion(expression) => legacy_dependencies(&expression.expression, out),
        Expression::TSInstantiationExpression(expression) => legacy_dependencies(&expression.expression, out),
        Expression::StaticMemberExpression(member) => legacy_dependencies(&member.object, out),
        Expression::ComputedMemberExpression(member) => legacy_dependencies(&member.object, out),
        Expression::PrivateFieldExpression(member) => legacy_dependencies(&member.object, out),
        Expression::CallExpression(call) => {
            legacy_dependencies(&call.callee, out);
            arguments(&call.arguments, out);
        }
        Expression::NewExpression(call) => {
            legacy_dependencies(&call.callee, out);
            arguments(&call.arguments, out);
        }
        Expression::BinaryExpression(binary) => {
            legacy_dependencies(&binary.left, out);
            legacy_dependencies(&binary.right, out);
        }
        Expression::LogicalExpression(logical) => {
            legacy_dependencies(&logical.left, out);
            legacy_dependencies(&logical.right, out);
        }
        Expression::PrivateInExpression(expression) => legacy_dependencies(&expression.right, out),
        Expression::AssignmentExpression(assignment) => {
            if let Some(target) = assignment.left.as_simple_assignment_target() {
                simple_target(target, out);
            }

            legacy_dependencies(&assignment.right, out);
        }
        Expression::ArrayExpression(array) => {
            for element in &array.elements {
                match element {
                    ArrayExpressionElement::SpreadElement(spread) => legacy_dependencies(&spread.argument, out),
                    ArrayExpressionElement::Elision(_) => {}
                    element => {
                        if let Some(expression) = element.as_expression() {
                            legacy_dependencies(expression, out);
                        }
                    }
                }
            }
        }
        Expression::UnaryExpression(unary) => legacy_dependencies(&unary.argument, out),
        Expression::AwaitExpression(expression) => legacy_dependencies(&expression.argument, out),
        Expression::YieldExpression(expression) => {
            if let Some(argument) = &expression.argument {
                legacy_dependencies(argument, out);
            }
        }
        Expression::UpdateExpression(update) => simple_target(&update.argument, out),
        Expression::ObjectExpression(object) => {
            for property in &object.properties {
                match property {
                    ObjectPropertyKind::SpreadProperty(spread) => legacy_dependencies(&spread.argument, out),
                    ObjectPropertyKind::ObjectProperty(property) => {
                        match &property.key {
                            PropertyKey::StaticIdentifier(key) => out.push(key.name.to_string()),
                            PropertyKey::PrivateIdentifier(_) => {}
                            key => {
                                if let Some(expression) = key.as_expression() {
                                    legacy_dependencies(expression, out);
                                }
                            }
                        }

                        legacy_dependencies(&property.value, out);
                    }
                }
            }
        }
        Expression::SequenceExpression(sequence) => sequence.expressions.iter().for_each(|expression| legacy_dependencies(expression, out)),
        Expression::TemplateLiteral(template) => template.expressions.iter().for_each(|expression| legacy_dependencies(expression, out)),
        _ => {}
    }
}

fn arguments(arguments: &[Argument<'_>], out: &mut Vec<String>) {
    for argument in arguments {
        match argument {
            Argument::SpreadElement(spread) => legacy_dependencies(&spread.argument, out),
            argument => {
                if let Some(expression) = argument.as_expression() {
                    legacy_dependencies(expression, out);
                }
            }
        }
    }
}

fn simple_target(target: &SimpleAssignmentTarget<'_>, out: &mut Vec<String>) {
    match target {
        SimpleAssignmentTarget::AssignmentTargetIdentifier(identifier) => out.push(identifier.name.to_string()),
        SimpleAssignmentTarget::TSAsExpression(expression) => legacy_dependencies(&expression.expression, out),
        SimpleAssignmentTarget::TSSatisfiesExpression(expression) => legacy_dependencies(&expression.expression, out),
        SimpleAssignmentTarget::TSNonNullExpression(expression) => legacy_dependencies(&expression.expression, out),
        SimpleAssignmentTarget::TSTypeAssertion(expression) => legacy_dependencies(&expression.expression, out),
        target => {
            if let Some(member) = target.as_member_expression() {
                legacy_dependencies(member.object(), out);
            }
        }
    }
}
