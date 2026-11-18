//! `anti-slop/max-doc-comment-lines`: doc comments that say more than the
//! code can't, and doc comments on what only the class itself calls.
//!
//! A doc comment is the run of own-line comments directly above a
//! declaration, statement, class member, interface or enum member, object
//! property or constructor parameter property, with no blank line between
//! them. One longer than `max` content lines is reported; the `/**` and `*/`
//! lines and blank lines don't count. A doc comment of any length on a
//! private member (`private` or `#name`) or a constructor is reported too.
//!
//! Comments a tool or another rule requires are never counted: directives
//! ([`is_directive`]: suppressions, `@ts-*`, formatter ignores, coverage
//! hints, `/// <reference`, `/*!` and `@license`), `// observability: …`
//! escapes, and whole comments that state a `SAFETY:` invariant, open the
//! file with a license or copyright notice, or match `ignorePatterns`.
//!
//! Options: `{ max?: integer >= 1 = 2, ignoreTests?: boolean = false,
//! ignorePatterns?: string[] = [] }`. `ignorePatterns` are regular
//! expressions tested against each comment's text.

use std::collections::BTreeMap;

use oxc_ast::AstKind;
use oxc_ast::ast::{Comment, MethodDefinitionKind, PropertyKey, TSAccessibility};
use oxc_span::{GetSpan, Span};
use regress::Regex;
use serde_json::Value;

use super::super::anti_slop_martinfowler::shared::{Options, is_test_file};
use super::super::{Context, Rule};
use super::shared::{comment_value, comments_before, is_directive, is_js_space};

pub const NAME: &str = "anti-slop/max-doc-comment-lines";

pub fn build(options: &[Value]) -> Result<Box<dyn Rule>, String> {
    let options = Options::parse(options, &["ignorePatterns", "ignoreTests", "max"])?;
    let ignore_patterns = options
        .strings("ignorePatterns", &[])?
        .iter()
        .map(|pattern| Regex::with_flags(pattern, "u").map_err(|error| format!("\"ignorePatterns\" has an invalid pattern {pattern:?}: {error}")))
        .collect::<Result<_, _>>()?;

    Ok(Box::new(MaxDocCommentLines {
        max: usize::try_from(options.integer("max", 1, 2)?).unwrap_or(usize::MAX),
        ignore_tests: options.boolean("ignoreTests", false)?,
        ignore_patterns,
    }))
}

struct MaxDocCommentLines {
    max: usize,
    ignore_tests: bool,
    ignore_patterns: Vec<Regex>,
}

/// What a doc comment sits on, by how strictly it is judged.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
enum Target {
    #[default]
    Public,
    Private,
    Constructor,
}

impl Rule for MaxDocCommentLines {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run_once(&self, ctx: &mut Context<'_, '_>) {
        if self.ignore_tests && is_test_file(ctx.rel) {
            return;
        }

        let semantic = ctx.semantic;
        let source = semantic.source_text();
        let lines = Lines::new(source);
        // Nodes that share a start share a doc comment; the strictest one judges it.
        let mut targets: BTreeMap<u32, Target> = BTreeMap::new();

        for node in semantic.nodes().iter() {
            if let Some(target) = target(node.kind()) {
                let entry = targets.entry(node.kind().span().start).or_default();

                *entry = (*entry).max(target);
            }
        }

        for (start, target) in targets {
            let doc = doc_before(ctx, &lines, start);
            let counted: Vec<&Comment> = units(source, &lines, &doc).into_iter().filter(|unit| !self.exempt(source, unit)).flatten().collect();
            let (Some(first), Some(last)) = (counted.first(), counted.last()) else {
                continue;
            };

            let span = Span::new(first.span.start, last.span.end);
            let length: usize = counted.iter().map(|comment| content_lines(comment_value(source, comment))).sum();

            match target {
                Target::Constructor => {
                    ctx.report(span, "Drop the doc comment on this constructor. Say what callers need on the class, or on the factory they call.");
                }
                Target::Private => {
                    ctx.report(span, "Drop the doc comment on this private member. Only its class calls it, so let its name and body say what it does.");
                }
                Target::Public if length > self.max => {
                    let max = self.max;
                    let noun = if max == 1 { "line" } else { "lines" };

                    ctx.report(
                        span,
                        format!("This doc comment runs {length} lines. Keep a doc comment to what the code can't say, in at most {max} {noun}; the code, its tests and the commit message carry the rest."),
                    );
                }
                Target::Public => {}
            }
        }
    }
}

impl MaxDocCommentLines {
    /// Whether another rule or a tool needs this comment as written.
    fn exempt(&self, source: &str, unit: &[&Comment]) -> bool {
        let texts = || unit.iter().map(|comment| comment_value(source, comment));

        texts().any(|text| mentions_safety(text) || self.ignore_patterns.iter().any(|pattern| pattern.find(text).is_some()))
            || (opens_file(source, unit[0]) && texts().any(is_license))
    }
}

/// How strictly a doc comment above `kind` is judged, if it can have one.
fn target(kind: AstKind<'_>) -> Option<Target> {
    let member = |key: &PropertyKey<'_>, accessibility: Option<TSAccessibility>| {
        if key.is_private_identifier() || accessibility == Some(TSAccessibility::Private) { Target::Private } else { Target::Public }
    };

    match kind {
        AstKind::MethodDefinition(method) if method.kind == MethodDefinitionKind::Constructor => Some(Target::Constructor),
        AstKind::MethodDefinition(method) => Some(member(&method.key, method.accessibility)),
        AstKind::PropertyDefinition(property) => Some(member(&property.key, property.accessibility)),
        AstKind::AccessorProperty(property) => Some(member(&property.key, property.accessibility)),
        AstKind::FormalParameter(parameter) if parameter.accessibility == Some(TSAccessibility::Private) => Some(Target::Private),
        AstKind::FormalParameter(parameter) if parameter.accessibility.is_some() || parameter.readonly => Some(Target::Public),
        AstKind::ObjectProperty(_)
        | AstKind::StaticBlock(_)
        | AstKind::TSPropertySignature(_)
        | AstKind::TSMethodSignature(_)
        | AstKind::TSIndexSignature(_)
        | AstKind::TSCallSignatureDeclaration(_)
        | AstKind::TSConstructSignatureDeclaration(_)
        | AstKind::TSEnumMember(_) => Some(Target::Public),
        _ if kind.is_statement() || kind.is_declaration() => Some(Target::Public),
        _ => None,
    }
}

/// The own-line comments directly above `start`, with no blank line between
/// them or before `start`, without the ones that instruct a tool.
fn doc_before<'c>(ctx: &Context<'c, '_>, lines: &Lines, start: u32) -> Vec<&'c Comment> {
    let source = ctx.semantic.source_text();
    let mut doc = Vec::new();
    let mut line = lines.line(start);

    for comment in comments_before(ctx.semantic, start).iter().rev() {
        let end = lines.line(comment.span.end);

        if (end != line && end + 1 != line) || !lines.starts_line(source, comment.span.start) {
            break;
        }

        line = lines.line(comment.span.start);
        doc.push(comment);
    }

    doc.reverse();
    doc.retain(|comment| {
        let text = comment_value(source, comment);

        !is_directive(text) && !text.trim_start_matches(is_js_space).starts_with("observability:")
    });
    doc
}

/// The doc's comments as a reader sees them: each block comment alone, and
/// each run of `//` comments on consecutive lines together.
fn units<'c>(source: &str, lines: &Lines, doc: &[&'c Comment]) -> Vec<Vec<&'c Comment>> {
    let mut units: Vec<Vec<&Comment>> = Vec::new();

    for &comment in doc {
        let continues = units.last().and_then(|unit| unit.last()).is_some_and(|previous| {
            comment.is_line()
                && previous.is_line()
                && lines.line(previous.span.end) + 1 == lines.line(comment.span.start)
                && source[previous.span.end as usize..comment.span.start as usize].trim().is_empty()
        });

        match units.last_mut() {
            Some(unit) if continues => unit.push(comment),
            _ => units.push(vec![comment]),
        }
    }

    units
}

/// The lines of a comment's text that say something: not blank, and not just
/// the `*` gutter or the `/**` and `*/` delimiters.
fn content_lines(text: &str) -> usize {
    text.split('\n').filter(|line| !line.trim_matches(is_js_space).trim_start_matches('*').trim_matches(is_js_space).is_empty()).count()
}

/// Whether only whitespace or a hashbang comes before `comment`.
fn opens_file(source: &str, comment: &Comment) -> bool {
    let head = &source[..comment.span.start as usize];
    let head = if head.starts_with("#!") { head.split_once('\n').map_or("", |(_, rest)| rest) } else { head };

    head.chars().all(is_js_space)
}

/// Whether a comment's text is a license or copyright notice.
fn is_license(text: &str) -> bool {
    let text = text.to_lowercase();

    ["copyright", "license", "licence", "spdx-license-identifier", "©"].iter().any(|word| text.contains(word))
}

/// `/\bSAFETY\s*:/u`, as `require-safety-comment-for-type-assertion` reads it.
fn mentions_safety(text: &str) -> bool {
    text.match_indices("SAFETY").any(|(index, word)| {
        let bounded = !text[..index].chars().next_back().is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');

        bounded && text[index + word.len()..].trim_start_matches(is_js_space).starts_with(':')
    })
}

/// Line starts, for line numbers by byte offset.
struct Lines(Vec<usize>);

impl Lines {
    fn new(source: &str) -> Self {
        Self(std::iter::once(0).chain(source.match_indices('\n').map(|(at, _)| at + 1)).collect())
    }

    /// The zero-based line holding `offset`.
    fn line(&self, offset: u32) -> usize {
        self.0.partition_point(|&start| start <= offset as usize) - 1
    }

    /// Whether only whitespace comes before `offset` on its line.
    fn starts_line(&self, source: &str, offset: u32) -> bool {
        source[self.0[self.line(offset)]..offset as usize].chars().all(is_js_space)
    }
}
