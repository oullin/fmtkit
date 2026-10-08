//! `eslint-plugin-no-only-tests` 3.4.0: `no-only-tests/no-only-tests`.

use oxc_ast::AstKind;
use oxc_ast::ast::{Expression, StaticMemberExpression};
use oxc_semantic::AstNode;
use serde_json::Value;

use fmtkit_core::Edit;

use super::{Context, Factory, Rule};

pub const RULES: &[(&str, Factory)] = &[(NoOnlyTests::NAME, |options| Ok(Box::new(NoOnlyTests::from_options(options)?)))];

const BLOCK: &[&str] = &["describe", "it", "context", "test", "tape", "fixture", "serial", "Feature", "Scenario", "Given", "And", "When", "Then"];

/// Flags focused tests (`describe.only`) and, optionally, banned test functions (`fit`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoOnlyTests {
    block: Vec<String>,
    focus: Vec<String>,
    functions: Vec<String>,
    fix: bool,
}

impl Default for NoOnlyTests {
    fn default() -> Self {
        Self { block: BLOCK.iter().map(|&b| b.to_owned()).collect(), focus: vec!["only".into()], functions: Vec::new(), fix: false }
    }
}

impl NoOnlyTests {
    pub const NAME: &str = "no-only-tests/no-only-tests";

    /// `[{ block?, focus?, functions?, fix? }]`, validated like the plugin's schema.
    pub fn from_options(options: &[Value]) -> Result<Self, String> {
        let mut rule = Self::default();
        let object = match options {
            [] => return Ok(rule),
            [Value::Object(object)] => object,
            [_] => return Err("expected an options object".into()),
            _ => return Err("expected at most one options object".into()),
        };

        for (key, value) in object {
            match key.as_str() {
                "block" => rule.block = strings(key, value)?,
                "focus" => rule.focus = strings(key, value)?,
                "functions" => rule.functions = strings(key, value)?,
                "fix" => rule.fix = value.as_bool().ok_or("\"fix\" must be a boolean")?,
                other => return Err(format!("unknown option \"{other}\"")),
            }
        }

        Ok(rule)
    }

    fn blocked(&self, call_path: &str) -> bool {
        self.block.iter().any(|block| match block.strip_suffix('*') {
            Some(prefix) => call_path.starts_with(prefix),
            None => call_path.strip_prefix(block.as_str()).is_some_and(|rest| rest.starts_with('.')),
        })
    }
}

fn strings(key: &str, value: &Value) -> Result<Vec<String>, String> {
    let invalid = || format!("\"{key}\" must be an array of unique strings");
    let items = value.as_array().ok_or_else(invalid)?;
    let mut out: Vec<String> = Vec::with_capacity(items.len());

    for item in items {
        let item = item.as_str().ok_or_else(invalid)?;

        if out.iter().any(|seen| seen == item) {
            return Err(invalid());
        }

        out.push(item.to_owned());
    }

    Ok(out)
}

impl Rule for NoOnlyTests {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let (name, span) = match node.kind() {
            AstKind::IdentifierName(id) => (id.name.as_str(), id.span),
            AstKind::IdentifierReference(id) => (id.name.as_str(), id.span),
            AstKind::BindingIdentifier(id) => (id.name.as_str(), id.span),
            AstKind::LabelIdentifier(id) => (id.name.as_str(), id.span),
            _ => return,
        };

        if self.functions.iter().any(|f| f == name) {
            ctx.report(span, format!("{name} not permitted"));
        }

        if !self.focus.iter().any(|f| f == name) {
            return;
        }

        // ESTree has no parenthesis nodes, so the member is the first ancestor that is not one.
        let nodes = ctx.semantic.nodes();
        let Some(parent) = nodes.ancestor_kinds(node.id()).find(|kind| !matches!(kind, AstKind::ParenthesizedExpression(_))) else {
            return;
        };

        let call_path = match parent {
            AstKind::StaticMemberExpression(member) => call_path(member),
            AstKind::ComputedMemberExpression(_) | AstKind::PrivateFieldExpression(_) => String::new(),
            _ => return,
        };

        if !self.blocked(&call_path) {
            return;
        }

        let message = format!("{call_path} not permitted");

        if self.fix {
            ctx.report_with_fix(span, message, vec![Edit::new(span.start.saturating_sub(1), span.end, "")]);
        } else {
            ctx.report(span, message);
        }
    }
}

/// The plugin's `getCallPath`: property names down a chain of static members
/// and calls, ending at an identifier (`it.default.before(x).only` reads
/// `it.default.before.only`). Anything else ends the path where it is.
fn call_path(member: &StaticMemberExpression<'_>) -> String {
    let mut names = vec![member.property.name.as_str()];
    let mut expr = &member.object;

    loop {
        match expr {
            Expression::StaticMemberExpression(member) => {
                names.push(member.property.name.as_str());
                expr = &member.object;
            }
            Expression::CallExpression(call) => expr = &call.callee,
            Expression::ParenthesizedExpression(paren) => expr = &paren.expression,
            Expression::Identifier(id) => {
                names.push(id.name.as_str());
                break;
            }
            _ => break,
        }
    }

    names.reverse();
    names.join(".")
}
