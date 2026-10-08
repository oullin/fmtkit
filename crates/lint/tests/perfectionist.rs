//! perfectionist's own `RuleTester` cases, converted to JSON under
//! `tests/fixtures/perfectionist`, run against the native port.

use std::sync::OnceLock;

use icu_collator::CollatorBorrowed;
use icu_collator::options::CollatorOptions;
use icu_locale_core::locale;
use oxc_allocator::Allocator;
use oxc_parser::{ParseOptions, Parser};
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use serde_json::{Map, Value};

use fmtkit_lint::rules::{self, Finding};

/// ESLint's `RuleTester` (via `eslint-vitest-rule-tester`) fixes until stable.
const ESLINT_PASSES: usize = 10;

/// Cases oxc cannot parse at all, by rule and fixture line: index
/// signatures without a type and spread or mapped members in interfaces and
/// type literals. typescript-eslint recovers from these; fmtkit reports them
/// as syntax errors and never lints them.
const UNPARSEABLE: &[(&str, u64)] = &[
    ("sort-interfaces", 240),
    ("sort-interfaces", 253),
    ("sort-interfaces", 3126),
    ("sort-interfaces", 3422),
    ("sort-interfaces", 3435),
    ("sort-interfaces", 5540),
    ("sort-interfaces", 5826),
    ("sort-interfaces", 5839),
    ("sort-object-types", 156),
    ("sort-object-types", 170),
    ("sort-object-types", 3852),
    ("sort-object-types", 4064),
    ("sort-object-types", 4078),
    ("sort-object-types", 6213),
    ("sort-object-types", 6422),
    ("sort-object-types", 6436),
];

const DOES_NOT_PARSE: &str = "does not parse";

#[test]
fn sort_enums() {
    check(include_str!("fixtures/perfectionist/sort-enums.json"));
}

#[test]
fn sort_heritage_clauses() {
    check(include_str!("fixtures/perfectionist/sort-heritage-clauses.json"));
}

#[test]
fn sort_interfaces() {
    check(include_str!("fixtures/perfectionist/sort-interfaces.json"));
}

#[test]
fn sort_jsx_props() {
    check(include_str!("fixtures/perfectionist/sort-jsx-props.json"));
}

#[test]
fn sort_object_types() {
    check(include_str!("fixtures/perfectionist/sort-object-types.json"));
}

#[test]
fn sort_objects() {
    check(include_str!("fixtures/perfectionist/sort-objects.json"));
}

fn check(fixture: &str) {
    let fixture: Value = serde_json::from_str(fixture).expect("a JSON fixture");
    let rule = fixture["rule"].as_str().expect("a rule name");
    let cases = fixture["cases"].as_array().expect("cases");
    let mut skipped = 0;
    let mut failures = Vec::new();

    for case in cases {
        let listed = UNPARSEABLE.contains(&(rule, case["line"].as_u64().unwrap_or_default()));
        let failure = match run(rule, case) {
            Err(why) if listed && why.starts_with(DOES_NOT_PARSE) => {
                skipped += 1;
                continue;
            }
            Err(why) => why,
            Ok(()) if listed => "parses now: remove it from UNPARSEABLE".to_owned(),
            Ok(()) => continue,
        };

        failures.push(format!("line {} {}\n  {failure}", case["line"], case["name"]));
    }

    println!("{rule}: {} pass, {skipped} skipped as unparseable, {} fail, of {}", cases.len() - skipped - failures.len(), failures.len(), cases.len());

    assert!(failures.is_empty(), "{rule}: {} of {} cases fail:\n{}", failures.len(), cases.len(), failures.join("\n"));
}

/// One case: its messages, then its output.
fn run(rule: &str, case: &Value) -> Result<(), String> {
    let code = case["code"].as_str().ok_or("no code")?;
    let options: Vec<Value> = case["options"].as_array().map(|options| options.iter().map(substitute).collect()).unwrap_or_default();
    let directive = format!("rule-to-test/{rule}");
    let built = rules::perfectionist::build(rule, &options, case.get("settings").filter(|settings| !settings.is_null()), &directive)
        .map_err(|e| format!("options rejected: {e}"))?;
    let rules = [built];
    let source_types = source_types(case);
    let passes = if case["tester"] == "oxlint" { 1 } else { ESLINT_PASSES };
    let first = lint(&rules, code, &source_types, &directive)?;
    let actual: Vec<&str> = first.iter().map(|finding| finding.message.as_str()).collect();
    let expected = case["errors"].as_array().map(Vec::as_slice).unwrap_or_default();

    if actual.len() != expected.len() || !actual.iter().zip(expected).all(|(actual, expected)| matches(actual, expected)) {
        let expected: Vec<String> = expected.iter().map(|error| render(error).unwrap_or_else(|| format!("<{}>", error["messageId"]))).collect();

        return Err(format!("messages differ\n  expected: {expected:#?}\n  actual:   {actual:#?}\n  code:\n{code}"));
    }

    let mut fixed = code.to_owned();
    let mut findings = first;

    for pass in 0..passes {
        if pass > 0 {
            findings = lint(&rules, &fixed, &source_types, &directive).map_err(|why| format!("fix output {why}"))?;
        }

        match apply(&fixed, &findings) {
            Some(next) if next != fixed => fixed = next,
            _ => break,
        }
    }

    let wanted = case["output"].as_str().unwrap_or(code);

    if fixed == wanted { Ok(()) } else { Err(format!("output differs\n  code:\n{code}\n  expected:\n{wanted}\n  actual:\n{fixed}")) }
}

/// The source types to try, in order: typescript-eslint parses `.tsx`-like
/// sources only with JSX enabled, so fall back to it.
fn source_types(case: &Value) -> Vec<SourceType> {
    match (case["parser"].as_str(), case["lang"].as_str()) {
        (Some("default"), _) => vec![SourceType::jsx()],
        (Some("typescript"), Some("jsx")) => vec![SourceType::tsx()],
        _ => vec![SourceType::ts(), SourceType::tsx()],
    }
}

/// The unsuppressed findings for `code`, sorted the way ESLint sorts them.
/// A clean parse wins; failing that, the first AST oxc recovered, as
/// typescript-eslint accepts shorthand initializers in object literals and
/// destructuring declarations without initializers.
fn lint(rules: &[Box<dyn rules::Rule>], code: &str, source_types: &[SourceType], directive: &str) -> Result<Vec<Finding>, String> {
    let attempts = source_types.iter().map(|&source_type| (source_type, false)).chain(source_types.iter().map(|&source_type| (source_type, true)));

    for (source_type, recovered) in attempts {
        let allocator = Allocator::default();
        let parsed =
            Parser::new(&allocator, code, source_type).with_options(ParseOptions { parse_regular_expression: true, ..ParseOptions::default() }).parse();

        if parsed.fatal_error || (!recovered && !parsed.diagnostics.is_empty()) {
            continue;
        }

        let program = allocator.alloc(parsed.program);
        let built = SemanticBuilder::new_linter().build(program);
        let comments: Vec<(u32, u32)> = program.comments.iter().map(|comment| (comment.span.start, comment.span.end)).collect();
        let suppressions = Suppressions::new(code, &comments, directive);
        let mut findings: Vec<Finding> =
            rules::run_all(rules, &built.semantic, "file.ts").into_iter().filter(|finding| !suppressions.suppressed(finding.span.start)).collect();

        findings.sort_by_key(|finding| finding.span.start);

        return Ok(findings);
    }

    Err(format!("{DOES_NOT_PARSE}:\n{code}"))
}

/// ESLint's `SourceCodeFixer`: each problem's edits merge into one range fix,
/// and a fix is skipped when it starts at or before the previous one's end.
fn apply(code: &str, findings: &[Finding]) -> Option<String> {
    let mut fixes: Vec<(u32, u32, String)> = findings
        .iter()
        .filter(|finding| !finding.fix.is_empty())
        .map(|finding| {
            let mut edits = finding.fix.clone();

            edits.sort_by_key(|edit| (edit.start, edit.end));

            let start = edits.iter().map(|edit| edit.start).min().unwrap_or_default();
            let end = edits.iter().map(|edit| edit.end).max().unwrap_or_default();
            let mut text = String::new();
            let mut at = start;

            for edit in &edits {
                text.push_str(&code[at as usize..edit.start as usize]);
                text.push_str(&edit.text);
                at = edit.end;
            }

            text.push_str(&code[at as usize..end as usize]);

            (start, end, text)
        })
        .collect();

    if fixes.is_empty() {
        return None;
    }

    fixes.sort_by_key(|&(start, end, _)| (start, end));

    let mut out = String::new();
    let mut last: Option<u32> = None;

    for (start, end, text) in fixes {
        if last.is_some_and(|last| last >= start) {
            continue;
        }

        out.push_str(&code[last.unwrap_or_default() as usize..start as usize]);
        out.push_str(&text);
        last = Some(end);
    }

    out.push_str(&code[last.unwrap_or_default() as usize..]);

    Some(out)
}

/// `eslint-disable` comments, as ESLint applies them to one rule.
struct Suppressions {
    /// Block directives by offset: `(offset, disable, all rules)`.
    blocks: Vec<(u32, bool, bool)>,
    /// Lines (0-based) a `-line` or `-next-line` directive covers.
    lines: Vec<usize>,
    line_starts: Vec<u32>,
}

impl Suppressions {
    fn new(code: &str, comments: &[(u32, u32)], rule: &str) -> Self {
        let line_starts: Vec<u32> = std::iter::once(0).chain(code.match_indices('\n').map(|(i, _)| u32::try_from(i + 1).expect("a small source"))).collect();
        let line_of = |offset: u32| line_starts.partition_point(|&start| start <= offset) - 1;
        let mut blocks = Vec::new();
        let mut lines = Vec::new();

        for &(start, end) in comments {
            let text = &code[start as usize..end as usize];
            let body = text.strip_prefix("//").or_else(|| text.strip_prefix("/*").and_then(|text| text.strip_suffix("*/"))).unwrap_or(text);
            let body = body.split(" -- ").next().unwrap_or_default().trim();
            let Some((kind, list)) = ["eslint-disable-next-line", "eslint-disable-line", "eslint-disable", "eslint-enable"].iter().find_map(|kind| {
                body.strip_prefix(kind).filter(|rest| rest.is_empty() || rest.starts_with(char::is_whitespace)).map(|rest| (*kind, rest.trim()))
            }) else {
                continue;
            };
            let all = list.is_empty();

            if !all && !list.split(',').any(|name| name.trim() == rule) {
                continue;
            }

            match kind {
                "eslint-disable-next-line" => lines.push(line_of(end) + 1),
                "eslint-disable-line" => lines.push(line_of(start)),
                "eslint-disable" if text.starts_with("/*") => blocks.push((start, true, all)),
                "eslint-enable" if text.starts_with("/*") => blocks.push((start, false, all)),
                _ => {}
            }
        }

        Self { blocks, lines, line_starts }
    }

    fn suppressed(&self, offset: u32) -> bool {
        let line = self.line_starts.partition_point(|&start| start <= offset) - 1;

        self.lines.contains(&line) || self.blocks.iter().take_while(|&&(at, _, _)| at <= offset).last().is_some_and(|&(_, disable, _)| disable)
    }
}

/// Whether `actual` is the message `expected` describes. Without `data`,
/// ESLint checks the message id alone.
fn matches(actual: &str, expected: &Value) -> bool {
    match render(expected) {
        Some(message) => actual == message,
        None => kind(expected["messageId"].as_str().unwrap_or_default()) == kind_of(actual),
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Kind {
    Order,
    GroupOrder,
    DependencyOrder,
    ExtraSpacing,
    MissedSpacing,
}

fn kind(message_id: &str) -> Kind {
    if message_id.ends_with("GroupOrder") {
        Kind::GroupOrder
    } else if message_id.ends_with("DependencyOrder") {
        Kind::DependencyOrder
    } else if message_id.starts_with("extraSpacing") {
        Kind::ExtraSpacing
    } else if message_id.starts_with("missedSpacing") {
        Kind::MissedSpacing
    } else {
        Kind::Order
    }
}

fn kind_of(message: &str) -> Kind {
    if message.starts_with("Expected dependency") {
        Kind::DependencyOrder
    } else if message.starts_with("Extra spacing") {
        Kind::ExtraSpacing
    } else if message.starts_with("Missed spacing") {
        Kind::MissedSpacing
    } else if message.ends_with(").") {
        Kind::GroupOrder
    } else {
        Kind::Order
    }
}

/// The message perfectionist's template renders for an expected error.
fn render(error: &Value) -> Option<String> {
    let data = error.get("data")?.as_object()?;
    let template = match kind(error["messageId"].as_str()?) {
        Kind::Order => "Expected \"{{right}}\" to come before \"{{left}}\".",
        Kind::GroupOrder => "Expected \"{{right}}\" ({{rightGroup}}) to come before \"{{left}}\" ({{leftGroup}}).",
        Kind::DependencyOrder => "Expected dependency \"{{right}}\" to come before \"{{nodeDependentOnRight}}\".",
        Kind::ExtraSpacing => "Extra spacing between \"{{left}}\" and \"{{right}}\".",
        Kind::MissedSpacing => "Missed spacing between \"{{left}}\" and \"{{right}}\".",
    };

    Some(interpolate(template, data))
}

/// ESLint's `interpolate`: unknown keys stay as they are.
fn interpolate(template: &str, data: &Map<String, Value>) -> String {
    let mut out = String::new();
    let mut rest = template;

    while let Some(open) = rest.find("{{") {
        let Some(close) = rest[open..].find("}}") else { break };
        let key = rest[open + 2..open + close].trim();

        out.push_str(&rest[..open]);

        match data.get(key) {
            Some(Value::String(value)) => out.push_str(value),
            Some(value) => out.push_str(&value.to_string()),
            None => out.push_str(&rest[open..open + close + 2]),
        }

        rest = &rest[open + close + 2..];
    }

    out.push_str(rest);
    out
}

/// Replaces `{"$alphabet": ...}` placeholders with the alphabet they name.
fn substitute(value: &Value) -> Value {
    match value {
        Value::Object(object) if object.contains_key("$alphabet") => Value::String(recommended_alphabet().to_owned()),
        Value::Object(object) => Value::Object(object.iter().map(|(key, value)| (key.clone(), substitute(value))).collect()),
        Value::Array(array) => Value::Array(array.iter().map(substitute).collect()),
        other => other.clone(),
    }
}

/// `Alphabet.generateRecommendedAlphabet().sortByLocaleCompare('en-US')`:
/// planes 0 and 1, stably sorted by an `en-US` collator. Lone surrogates
/// cannot appear in a Rust string and are left out.
fn recommended_alphabet() -> &'static str {
    static ALPHABET: OnceLock<String> = OnceLock::new();

    ALPHABET.get_or_init(|| {
        let collator = CollatorBorrowed::try_new((&locale!("en-US")).into(), CollatorOptions::default()).expect("collation data");
        let mut characters: Vec<String> = (0..=0x1_ffff).filter_map(char::from_u32).map(String::from).collect();

        characters.sort_by(|a, b| collator.compare(a, b));
        characters.concat()
    })
}
