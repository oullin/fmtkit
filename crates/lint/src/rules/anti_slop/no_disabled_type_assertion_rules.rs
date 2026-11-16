//! `anti-slop/no-disabled-type-assertion-rules`: no inline disabling of the
//! rules that keep type assertions out, and no disable comment that names no
//! rule.
//!
//! Reads every `oxlint-disable`, `eslint-disable`, `-line` and `-next-line`
//! comment: reports, at the comment, each listed rule that is a
//! type-assertion rule (`@typescript-eslint/` reads as `typescript/`), and a
//! directive that lists none, since that disables them all. A `-- reason`
//! is not part of the list; prose that merely mentions a directive is not
//! one. Disables in the lint config are out of scope.
//!
//! Options: `[{ additionalRules?: string[] }]`, more rule names to protect,
//! unique and non-empty.

use std::borrow::Cow;

use serde_json::Value;

use super::super::{Context, Rule};
use super::shared::{comment_value, is_js_space};

pub const NAME: &str = "anti-slop/no-disabled-type-assertion-rules";

/// The rules that keep type assertions out. An inline disable of any of them
/// lets a cast through.
const DEFAULT_RULES: [&str; 6] = [
    "anti-slop/no-chained-type-assertions",
    "anti-slop/no-widen-then-assert",
    "anti-slop/require-safety-comment-for-type-assertion",
    "effecttsgo/unsafe-effect-type-assertion",
    "typescript/consistent-type-assertions",
    "typescript/no-unsafe-type-assertion",
];

/// `[{ additionalRules?: string[] }]`, validated like upstream's schema.
pub fn build(options: &[Value]) -> Result<Box<dyn Rule>, String> {
    let mut rules: Vec<String> = DEFAULT_RULES.iter().map(|&rule| rule.to_owned()).collect();

    match options {
        [] => {}
        [Value::Object(object)] => {
            for (key, value) in object {
                match key.as_str() {
                    "additionalRules" => {
                        let Value::Array(additional) = value else {
                            return Err("\"additionalRules\" must be an array of rule names".into());
                        };

                        for (index, rule) in additional.iter().enumerate() {
                            let rule = rule.as_str().filter(|rule| !rule.is_empty()).ok_or("\"additionalRules\" must hold non-empty strings")?;

                            if additional.iter().take(index).any(|earlier| earlier.as_str() == Some(rule)) {
                                return Err(format!("\"additionalRules\" lists \"{rule}\" twice"));
                            }

                            rules.push(normalize(rule).into_owned());
                        }
                    }
                    other => return Err(format!("unknown option \"{other}\"")),
                }
            }
        }
        [_] => return Err("expected an options object".into()),
        _ => return Err("expected at most one options object".into()),
    }

    Ok(Box::new(NoDisabledTypeAssertionRules { rules }))
}

struct NoDisabledTypeAssertionRules {
    rules: Vec<String>,
}

impl Rule for NoDisabledTypeAssertionRules {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run_once(&self, ctx: &mut Context<'_, '_>) {
        let semantic = ctx.semantic;
        let source = semantic.source_text();

        for comment in semantic.comments() {
            let Some(listed) = directive_rules(comment_value(source, comment)) else {
                continue;
            };

            let mut names = listed.peekable();

            if names.peek().is_none() {
                ctx.report(comment.span, "This disable comment names no rule, so it turns off the type-assertion rules too. Name the rules it disables.");

                continue;
            }

            for rule in names {
                if self.rules.iter().any(|protected| *protected == rule) {
                    ctx.report(
                        comment.span,
                        format!("Don't disable `{rule}` inline. Remove the cast, or add a file override with its reason in the lint config."),
                    );
                }
            }
        }
    }
}

/// The rule names a disable directive lists, normalized, or `None` when the
/// comment is not one: upstream's
/// `/^\s*(?:oxlint|eslint)-disable(?:-next-line|-line)?(?=\s|$)(.*)$/su`, cut
/// at the first `\s--\s` or trailing `\s--`, split on commas.
fn directive_rules(value: &str) -> Option<impl Iterator<Item = Cow<'_, str>>> {
    let text = value.trim_start_matches(is_js_space);
    let rest = text.strip_prefix("oxlint-disable").or_else(|| text.strip_prefix("eslint-disable"))?;
    let at_boundary = |rest: &str| rest.is_empty() || rest.starts_with(is_js_space);
    let rest = ["-next-line", "-line"]
        .iter()
        .find_map(|suffix| rest.strip_prefix(suffix).filter(|after| at_boundary(after)))
        .or_else(|| at_boundary(rest).then_some(rest))?;
    let list = rest
        .char_indices()
        .find(|&(at, c)| is_js_space(c) && rest[at + c.len_utf8()..].strip_prefix("--").is_some_and(at_boundary))
        .map_or(rest, |(at, _)| &rest[..at]);

    Some(list.split(',').map(|rule| normalize(rule.trim_matches(is_js_space))).filter(|rule| !rule.is_empty()))
}

/// `rule` with `@typescript-eslint/` or `typescript-eslint/` read as
/// `typescript/`, as upstream compares names.
fn normalize(rule: &str) -> Cow<'_, str> {
    rule.strip_prefix('@').unwrap_or(rule).strip_prefix("typescript-eslint/").map_or(Cow::Borrowed(rule), |name| Cow::Owned(format!("typescript/{name}")))
}
