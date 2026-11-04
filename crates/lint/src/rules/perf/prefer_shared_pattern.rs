//! `perf/prefer-shared-pattern`: a regular expression that a shared module
//! already exports is imported from there, not written again.
//!
//! The rule has no default list: `patterns` maps each exported name to its
//! literal exactly as written, flags included. The declaration that defines a
//! name (`const WHITESPACE = /\s+/u`) is not reported.

use oxc_ast::AstKind;
use oxc_semantic::AstNode;
use rustc_hash::FxHashMap;
use serde_json::Value;

use super::super::anti_slop::shared::parent;
use super::super::{Context, Rule};

pub const NAME: &str = "perf/prefer-shared-pattern";

/// `[{ module: string, patterns: { [name]: "/source/flags" } }]`, both keys
/// required.
pub fn build(options: &[Value]) -> Result<Box<dyn Rule>, String> {
    let [Value::Object(object)] = options else {
        return Err("expected one options object with \"module\" and \"patterns\"".into());
    };

    let mut module = None;
    let mut by_literal = FxHashMap::default();

    for (key, value) in object {
        match key.as_str() {
            "module" => module = Some(value.as_str().filter(|module| !module.is_empty()).ok_or("\"module\" must be a non-empty string")?.to_owned()),
            "patterns" => by_literal = patterns(value)?,
            other => return Err(format!("unknown option \"{other}\"")),
        }
    }

    let module = module.ok_or("\"module\" is required")?;

    if by_literal.is_empty() {
        return Err("\"patterns\" must name at least one pattern".into());
    }

    Ok(Box::new(PreferSharedPattern { module, by_literal }))
}

/// The configured patterns keyed by their literal, so a literal finds its name.
fn patterns(value: &Value) -> Result<FxHashMap<String, String>, String> {
    let object = value.as_object().ok_or("\"patterns\" must be an object of name to literal")?;
    let mut by_literal = FxHashMap::default();

    for (name, literal) in object {
        let literal = literal
            .as_str()
            .filter(|literal| literal.len() > 2 && literal.starts_with('/'))
            .ok_or_else(|| format!("pattern \"{name}\" must be a literal such as \"/\\\\s+/u\""))?;

        if let Some(other) = by_literal.insert(literal.to_owned(), name.clone()) {
            return Err(format!("patterns \"{other}\" and \"{name}\" share the literal {literal}"));
        }
    }

    Ok(by_literal)
}

struct PreferSharedPattern {
    module: String,
    by_literal: FxHashMap<String, String>,
}

impl Rule for PreferSharedPattern {
    fn name(&self) -> &'static str {
        NAME
    }

    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let AstKind::RegExpLiteral(literal) = node.kind() else {
            return;
        };

        let text = &ctx.source()[literal.span.start as usize..literal.span.end as usize];

        let Some(name) = self.by_literal.get(text) else {
            return;
        };

        if defines(ctx, node, name) {
            return;
        }

        ctx.report(literal.span, format!("`{text}` is `{name}` in `{}`. Import it from there instead of writing the pattern again.", self.module));
    }
}

/// Whether `node` is the value of the declaration that defines `name`.
fn defines(ctx: &Context<'_, '_>, node: &AstNode<'_>, name: &str) -> bool {
    matches!(parent(ctx.semantic.nodes(), node.id()).kind(), AstKind::VariableDeclarator(declarator) if declarator.id.get_identifier_name().is_some_and(|declared| declared == name))
}
