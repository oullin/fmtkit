//! A port of `eslint-plugin-perfectionist@5.12.1`'s sort-enums,
//! sort-heritage-clauses, sort-interfaces, sort-jsx-props, sort-object-types
//! and sort-objects, autofixes included.

use std::rc::Rc;

use oxc_ast::ast::{Expression, PropertyKey};
use oxc_semantic::{AstNodes, NodeId};
use oxc_span::Span;
use rustc_hash::FxHashSet;
use serde_json::Value;

use super::{Context, Factory, Rule};

mod deps;
mod enums;
mod estree;
mod heritage;
mod jsnum;
mod jsx_props;
mod natural;
mod object_types;
mod objects;
mod options;
mod regex;
mod report;
mod sort;
mod source;

use options::{Options, PartitionComment, Schema};
use report::Problem;
use source::{Index, Src};

pub const RULES: &[(&str, Factory)] = &[
    ("perfectionist/sort-enums", sort_enums),
    ("perfectionist/sort-heritage-clauses", sort_heritage_clauses),
    ("perfectionist/sort-interfaces", sort_interfaces),
    ("perfectionist/sort-jsx-props", sort_jsx_props),
    ("perfectionist/sort-object-types", sort_object_types),
    ("perfectionist/sort-objects", sort_objects),
];

fn sort_enums(options: &[Value]) -> Result<Box<dyn Rule>, String> {
    build("sort-enums", options, None, "perfectionist/sort-enums")
}

fn sort_heritage_clauses(options: &[Value]) -> Result<Box<dyn Rule>, String> {
    build("sort-heritage-clauses", options, None, "perfectionist/sort-heritage-clauses")
}

fn sort_interfaces(options: &[Value]) -> Result<Box<dyn Rule>, String> {
    build("sort-interfaces", options, None, "perfectionist/sort-interfaces")
}

fn sort_jsx_props(options: &[Value]) -> Result<Box<dyn Rule>, String> {
    build("sort-jsx-props", options, None, "perfectionist/sort-jsx-props")
}

fn sort_object_types(options: &[Value]) -> Result<Box<dyn Rule>, String> {
    build("sort-object-types", options, None, "perfectionist/sort-object-types")
}

fn sort_objects(options: &[Value]) -> Result<Box<dyn Rule>, String> {
    build("sort-objects", options, None, "perfectionist/sort-objects")
}

/// Builds `rule` (`sort-enums`, ...) from its options and ESLint's
/// `settings` object, matching `eslint-disable` comments against `directive`.
pub fn build(rule: &str, options: &[Value], settings: Option<&Value>, directive: &str) -> Result<Box<dyn Rule>, String> {
    let settings = settings.and_then(|settings| settings.get("perfectionist"));

    Ok(match rule {
        "sort-enums" => Box::new(enums::SortEnums { base: Base::new(&enums::SCHEMA, options, settings, directive)? }),
        "sort-heritage-clauses" => Box::new(heritage::SortHeritageClauses { base: Base::new(&heritage::SCHEMA, options, settings, directive)? }),
        "sort-interfaces" => {
            Box::new(object_types::SortObjectTypes { interfaces: true, base: Base::new(&object_types::SCHEMA, options, settings, directive)? })
        }
        "sort-jsx-props" => Box::new(jsx_props::SortJsxProps { base: Base::new(&jsx_props::SCHEMA, options, settings, directive)? }),
        "sort-object-types" => {
            Box::new(object_types::SortObjectTypes { interfaces: false, base: Base::new(&object_types::SCHEMA, options, settings, directive)? })
        }
        "sort-objects" => Box::new(objects::SortObjects { base: Base::new(&objects::SCHEMA, options, settings, directive)? }),
        other => return Err(format!("unknown perfectionist rule \"{other}\"")),
    })
}

/// What every rule keeps: its option candidates and its directive name.
struct Base {
    directive: String,
    /// One per `context.options` entry, in order.
    candidates: Vec<Options>,
    /// Defaults and settings, for when no candidate matches.
    fallback: Options,
}

impl Base {
    fn new(schema: &Schema, options: &[Value], settings: Option<&Value>, directive: &str) -> Result<Self, String> {
        for (i, option) in options.iter().enumerate() {
            if options[..i].contains(option) {
                return Err("options must be unique".into());
            }
        }

        let candidates = options.iter().map(|option| options::build(schema, Some(option), settings)).collect::<Result<_, _>>()?;
        let fallback = options::build(schema, None, settings)?;

        Ok(Self { directive: directive.to_owned(), candidates, fallback })
    }

    /// The first candidate `matches` accepts, else the defaults.
    fn select(&self, matches: impl Fn(&Options) -> bool) -> &Options {
        self.candidates.iter().find(|options| matches(options)).unwrap_or(&self.fallback)
    }
}

/// The per-file view every rule run needs.
struct File {
    index: Rc<Index>,
    disabled: FxHashSet<u32>,
}

impl File {
    fn new(ctx: &mut Context<'_, '_>, directive: &str) -> Self {
        let index = ctx.shared(Index::new);
        let disabled = source::disabled_lines(Src { text: ctx.source(), index: &index }, directive);

        Self { index, disabled }
    }

    fn src<'s>(&'s self, text: &'s str) -> Src<'s> {
        Src { text, index: &self.index }
    }

    /// `isNodeEslintDisabled`.
    fn is_disabled(&self, src: Src<'_>, span: Span) -> bool {
        self.disabled.contains(&src.line(span.start))
    }
}

/// `shouldPartition`.
fn should_partition(src: Src<'_>, comments: &PartitionComment, newline: bool, last: Option<Span>, span: Span) -> bool {
    if comments.is_on() && src.relevant_comments_before(span.start).into_iter().any(|comment| comments.matches(comment, src.comment_value(comment))) {
        return true;
    }

    newline && last.is_some_and(|last| src.blank_lines_between(last.end, span.start) > 0)
}

/// `rangeToDiff`: the node's length, minus a trailing `,` or `;`.
fn size(src: Src<'_>, span: Span) -> u32 {
    let text = src.text_of(span);

    span.size() - u32::from(text.ends_with(',') || text.ends_with(';'))
}

/// Every node below `root`; ids are pre-order, so they follow it.
fn subtree(nodes: &AstNodes<'_>, root: NodeId) -> Vec<NodeId> {
    let mut inside = FxHashSet::from_iter([root]);
    let mut found = Vec::new();

    for index in root.index() + 1..nodes.len() {
        let id = NodeId::new(index);

        if !inside.contains(&nodes.parent_id(id)) {
            break;
        }

        inside.insert(id);
        found.push(id);
    }

    found
}

/// `String(literal.value)` for what ESTree calls a `Literal`.
fn literal(expression: &Expression<'_>) -> Option<String> {
    Some(match expression {
        Expression::StringLiteral(literal) => literal.value.to_string(),
        Expression::NumericLiteral(literal) => jsnum::format(literal.value),
        Expression::BooleanLiteral(literal) => literal.value.to_string(),
        Expression::NullLiteral(_) => "null".to_owned(),
        Expression::BigIntLiteral(literal) => literal.value.to_string(),
        Expression::RegExpLiteral(literal) => format!("/{}/{}", literal.regex.pattern.text, literal.regex.flags),
        _ => return None,
    })
}

/// A property key's name when ESTree has it as an `Identifier` (computed or
/// not) or a `Literal`.
fn key_name(key: &PropertyKey<'_>) -> Option<String> {
    match key {
        PropertyKey::StaticIdentifier(identifier) => Some(identifier.name.to_string()),
        PropertyKey::PrivateIdentifier(_) => None,
        key => match key.as_expression() {
            Some(Expression::Identifier(identifier)) => Some(identifier.name.to_string()),
            Some(expression) => literal(expression),
            None => None,
        },
    }
}

fn emit(ctx: &mut Context<'_, '_>, problems: Vec<Problem>) {
    for problem in problems {
        ctx.report_with_fix(problem.span, problem.message, problem.fix);
    }
}
