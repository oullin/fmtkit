//! `sort-enums`.

use oxc_ast::AstKind;
use oxc_ast::ast::{
    Argument, ArrayExpressionElement, BinaryOperator, Expression, ObjectPropertyKind, PropertyKey, SimpleAssignmentTarget, TSEnumDeclaration, TSEnumMemberName,
    UnaryOperator,
};
use oxc_semantic::{AstNode, AstNodes, NodeId};
use oxc_span::GetSpan;
use serde_json::{Value, json};

use super::estree::{self, unparenthesized};
use super::options::{Options, Schema};
use super::report::{Messages, Report};
use super::sort::{Item, Mode, Sorter, sort_by_dependencies};
use super::{Base, File, deps, emit, jsnum, literal, should_partition, size, subtree};
use crate::rules::{Context, Rule};

pub static SCHEMA: Schema = Schema {
    selectors: &[],
    modifiers: &[],
    match_keys: &["elementValuePattern"],
    sort_by: false,
    partition_by_comment: true,
    when_keys: &["matchesAstSelector"],
    extra_keys: &["sortByValue", "useExperimentalDependencyDetection"],
    defaults,
};

const MESSAGES: Messages = Messages {
    order: "unexpectedEnumsOrder",
    group_order: "unexpectedEnumsGroupOrder",
    extra_spacing: "extraSpacingBetweenEnumsMembers",
    missed_spacing: "missedSpacingBetweenEnumsMembers",
    dependency_order: Some("unexpectedEnumsDependencyOrder"),
};

fn defaults() -> Value {
    json!({
        "useExperimentalDependencyDetection": true,
        "fallbackSort": {"type": "unsorted"},
        "newlinesInside": "newlinesBetween",
        "sortByValue": "ifNumericEnum",
        "partitionByComment": false,
        "partitionByNewLine": false,
        "specialCharacters": "keep",
        "newlinesBetween": "ignore",
        "useConfigurationIf": {},
        "type": "alphabetical",
        "ignoreCase": true,
        "locales": "en-US",
        "customGroups": [],
        "alphabet": "",
        "order": "asc",
        "groups": [],
    })
}

pub struct SortEnums {
    pub base: Base,
}

impl Rule for SortEnums {
    fn name(&self) -> &'static str {
        "perfectionist/sort-enums"
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let AstKind::TSEnumDeclaration(declaration) = node.kind() else { return };
        let members = &declaration.body.members;

        if members.len() < 2 || members.iter().any(|member| member.initializer.is_none()) {
            return;
        }

        let nodes = ctx.semantic.nodes();
        let source = ctx.source();
        let names: Vec<String> = members.iter().map(|member| member_name(source, &member.id)).collect();
        let options = self
            .base
            .select(|options| options.when.all_names_match(&names) && options.when.selector.as_ref().is_none_or(|selector| selector.matches(nodes, node.id())));
        let file = File::new(ctx, &self.base.directive);
        let problems = sort(ctx, &file, options, declaration, names);

        emit(ctx, problems);
    }
}

fn sort<'a>(ctx: &Context<'_, 'a>, file: &File, options: &Options, declaration: &TSEnumDeclaration<'a>, names: Vec<String>) -> Vec<super::report::Problem> {
    let src = file.src(ctx.source());
    let nodes = ctx.semantic.nodes();
    let enum_name = declaration.id.name.as_str();
    let mut items: Vec<Item> = Vec::with_capacity(names.len());
    let mut partitions: Vec<Vec<usize>> = vec![Vec::new()];

    for (member, name) in declaration.body.members.iter().zip(names) {
        let Some(initializer) = member.initializer.as_ref().map(unparenthesized) else { continue };
        let initializer_text = src.text_of(initializer.span());
        let group = options.compute_group(&[], |matcher| matcher.matches(&name, Some(initializer_text), &[], &[]));
        let last = items.last().map(|item| item.span);

        if should_partition(src, &options.partition_comment, options.partition_newline, last, member.span) {
            partitions.push(Vec::new());
        }

        let mut dependencies = Vec::new();

        if !options.experimental {
            legacy_dependencies(initializer, enum_name, &mut dependencies);
        }

        if let Some(partition) = partitions.last_mut() {
            partition.push(items.len());
        }

        items.push(Item {
            span: member.span,
            value: if matches!(initializer, Expression::NullLiteral(_)) { String::new() } else { literal(initializer).unwrap_or_default() },
            numeric: number(initializer),
            size: size(src, member.span),
            disabled: file.is_disabled(src, member.span),
            dependency_names: vec![name.clone()],
            dependencies,
            group,
            partition: partitions.len(),
            name,
            semicolon: false,
        });
    }

    if options.experimental {
        let spans: Vec<_> = items.iter().map(|item| item.span).collect();
        let candidates = subtree(nodes, declaration.body.node_id.get());
        let referenced = deps::referenced(
            ctx.semantic,
            &candidates,
            &spans,
            |_| false,
            |reference| {
                if reference.name != enum_name {
                    return Vec::new();
                }

                let names = member_expression_names(nodes, reference.node);

                items.iter().enumerate().filter(|(_, item)| item.dependency_names.iter().any(|name| names.contains(name))).map(|(i, _)| i).collect()
            },
        );

        deps::apply(&mut items, &referenced);
    }

    let numeric = items.iter().all(|item| item.numeric.is_some());
    let sorter = Sorter { options, mode: Mode::Enum { by_value: options.sort_by_value, numeric } };
    let ordering = |ignore_disabled: bool| {
        let by_groups: Vec<usize> = partitions.iter().flat_map(|partition| sorter.sort_by_groups(&items, partition, ignore_disabled, |_, _| false)).collect();

        sort_by_dependencies(&items, &by_groups, ignore_disabled)
    };
    let all: Vec<usize> = (0..items.len()).collect();
    let report = Report { src, options, items: &items, messages: MESSAGES, partition_comment: &options.partition_comment };

    report.run(&all, &ordering(false), &ordering(true))
}

/// `computeNodeName`: a string literal's value, else the source text.
fn member_name(source: &str, id: &TSEnumMemberName<'_>) -> String {
    match id {
        TSEnumMemberName::String(literal) | TSEnumMemberName::ComputedString(literal) => literal.value.to_string(),
        TSEnumMemberName::Identifier(identifier) => identifier.name.to_string(),
        TSEnumMemberName::ComputedTemplateString(template) => {
            source.get(template.span.start as usize..template.span.end as usize).unwrap_or_default().to_owned()
        }
    }
}

/// `computeExpressionNumberValue`.
fn number(expression: &Expression<'_>) -> Option<f64> {
    match unparenthesized(expression) {
        Expression::NumericLiteral(literal) => Some(literal.value),
        Expression::UnaryExpression(unary) => {
            let argument = number(&unary.argument)?;

            match unary.operator {
                UnaryOperator::UnaryPlus => Some(argument),
                UnaryOperator::UnaryNegation => Some(-argument),
                UnaryOperator::BitwiseNot => Some(f64::from(!jsnum::int32(argument))),
                _ => None,
            }
        }
        Expression::BinaryExpression(binary) => {
            let (left, right) = (number(&binary.left)?, number(&binary.right)?);
            let shift = || jsnum::uint32(right) & 31;

            Some(match binary.operator {
                BinaryOperator::Exponential => jsnum::pow(left, right),
                BinaryOperator::ShiftRight => f64::from(jsnum::int32(left) >> shift()),
                BinaryOperator::ShiftLeft => f64::from(jsnum::int32(left).wrapping_shl(shift())),
                BinaryOperator::Addition => left + right,
                BinaryOperator::Subtraction => left - right,
                BinaryOperator::Multiplication => left * right,
                BinaryOperator::Division => left / right,
                BinaryOperator::Remainder => left % right,
                BinaryOperator::BitwiseOR => f64::from(jsnum::int32(left) | jsnum::int32(right)),
                BinaryOperator::BitwiseAnd => f64::from(jsnum::int32(left) & jsnum::int32(right)),
                BinaryOperator::BitwiseXOR => f64::from(jsnum::int32(left) ^ jsnum::int32(right)),
                _ => return None,
            })
        }
        _ => None,
    }
}

/// The property names of the member expressions chained onto the enum's
/// name; the chain ends before the member holding it.
fn member_expression_names(nodes: &AstNodes<'_>, identifier: NodeId) -> Vec<String> {
    let mut names = Vec::new();

    for ancestor in estree::ancestors(nodes, identifier) {
        match nodes.kind(ancestor) {
            AstKind::StaticMemberExpression(expression) => names.push(expression.property.name.to_string()),
            AstKind::ComputedMemberExpression(expression) => match unparenthesized(&expression.expression) {
                Expression::Identifier(identifier) => names.push(identifier.name.to_string()),
                Expression::StringLiteral(literal) => names.push(literal.value.to_string()),
                _ => {}
            },
            AstKind::PrivateFieldExpression(_) => {}
            _ => break,
        }
    }

    names
}

/// The legacy `computeDependencies`: identifiers and `Enum.member` names
/// reachable through ESTree's `alternate`, `argument(s)`, `callee`,
/// `consequent`, `elements`, `expressions`, `key`, `left`, `object`,
/// `properties`, `right`, `test` and node `value` keys.
fn legacy_dependencies(expression: &Expression<'_>, enum_name: &str, out: &mut Vec<String>) {
    let mut walk = |expression: &Expression<'_>| legacy_dependencies(expression, enum_name, out);

    match unparenthesized(expression) {
        Expression::Identifier(identifier) => out.push(identifier.name.to_string()),
        Expression::StaticMemberExpression(member) => {
            if matches!(unparenthesized(&member.object), Expression::Identifier(object) if object.name == enum_name) {
                out.push(member.property.name.to_string());
            }

            legacy_dependencies(&member.object, enum_name, out);
        }
        Expression::ComputedMemberExpression(member) => {
            if matches!(unparenthesized(&member.object), Expression::Identifier(object) if object.name == enum_name) {
                match unparenthesized(&member.expression) {
                    Expression::Identifier(identifier) => out.push(identifier.name.to_string()),
                    Expression::StringLiteral(literal) => out.push(literal.value.to_string()),
                    _ => {}
                }
            }

            legacy_dependencies(&member.object, enum_name, out);
        }
        Expression::PrivateFieldExpression(member) => walk(&member.object),
        Expression::ConditionalExpression(conditional) => {
            walk(&conditional.test);
            walk(&conditional.consequent);
            walk(&conditional.alternate);
        }
        Expression::UnaryExpression(unary) => walk(&unary.argument),
        Expression::AwaitExpression(expression) => walk(&expression.argument),
        Expression::YieldExpression(expression) => {
            if let Some(argument) = &expression.argument {
                walk(argument);
            }
        }
        Expression::UpdateExpression(update) => simple_target(&update.argument, enum_name, out),
        Expression::CallExpression(call) => {
            walk(&call.callee);
            arguments(&call.arguments, enum_name, out);
        }
        Expression::NewExpression(call) => {
            walk(&call.callee);
            arguments(&call.arguments, enum_name, out);
        }
        Expression::ArrayExpression(array) => {
            for element in &array.elements {
                match element {
                    ArrayExpressionElement::SpreadElement(spread) => walk(&spread.argument),
                    ArrayExpressionElement::Elision(_) => {}
                    element => {
                        if let Some(expression) = element.as_expression() {
                            walk(expression);
                        }
                    }
                }
            }
        }
        Expression::SequenceExpression(sequence) => sequence.expressions.iter().for_each(walk),
        Expression::TemplateLiteral(template) => template.expressions.iter().for_each(walk),
        Expression::BinaryExpression(binary) => {
            walk(&binary.left);
            walk(&binary.right);
        }
        Expression::LogicalExpression(logical) => {
            walk(&logical.left);
            walk(&logical.right);
        }
        Expression::PrivateInExpression(expression) => walk(&expression.right),
        Expression::AssignmentExpression(assignment) => {
            if let Some(target) = assignment.left.as_simple_assignment_target() {
                simple_target(target, enum_name, out);
            }

            legacy_dependencies(&assignment.right, enum_name, out);
        }
        Expression::ObjectExpression(object) => {
            for property in &object.properties {
                match property {
                    ObjectPropertyKind::SpreadProperty(spread) => legacy_dependencies(&spread.argument, enum_name, out),
                    ObjectPropertyKind::ObjectProperty(property) => {
                        match &property.key {
                            PropertyKey::StaticIdentifier(key) => out.push(key.name.to_string()),
                            PropertyKey::PrivateIdentifier(_) => {}
                            key => {
                                if let Some(expression) = key.as_expression() {
                                    legacy_dependencies(expression, enum_name, out);
                                }
                            }
                        }

                        legacy_dependencies(&property.value, enum_name, out);
                    }
                }
            }
        }
        _ => {}
    }
}

fn arguments(arguments: &[Argument<'_>], enum_name: &str, out: &mut Vec<String>) {
    for argument in arguments {
        match argument {
            Argument::SpreadElement(spread) => legacy_dependencies(&spread.argument, enum_name, out),
            argument => {
                if let Some(expression) = argument.as_expression() {
                    legacy_dependencies(expression, enum_name, out);
                }
            }
        }
    }
}

fn simple_target(target: &SimpleAssignmentTarget<'_>, enum_name: &str, out: &mut Vec<String>) {
    match target {
        SimpleAssignmentTarget::AssignmentTargetIdentifier(identifier) => out.push(identifier.name.to_string()),
        target => {
            if let Some(member) = target.as_member_expression() {
                let object = member.object();
                let is_enum = matches!(unparenthesized(object), Expression::Identifier(found) if found.name == enum_name);

                if is_enum && let Some(name) = member.static_property_name() {
                    out.push(name.to_string());
                }

                legacy_dependencies(object, enum_name, out);
            }
        }
    }
}
