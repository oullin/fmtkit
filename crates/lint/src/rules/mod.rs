//! fmtkit's native rules and the small framework they share.
//!
//! A rule sees every AST node once through [`Rule::run`], and the whole file
//! once through [`Rule::run_once`]. All enabled rules share one walk over one
//! semantic model, built once per file alongside oxc_linter's.

use std::any::{Any, TypeId};
use std::rc::Rc;

use oxc_semantic::{AstNode, Semantic};
use oxc_span::Span;
use rustc_hash::FxHashMap;

use fmtkit_core::Edit;

pub mod anti_slop;
pub mod nkzw;
pub mod no_only_tests;
pub mod perfectionist;

/// One finding from a native rule, positioned by byte span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub rule: &'static str,
    pub span: Span,
    pub message: String,
    /// A safe fix: edits against the source the rule saw. Must not overlap.
    pub fix: Vec<Edit>,
}

/// What a rule can see and report into.
pub struct Context<'s, 'a> {
    pub semantic: &'s Semantic<'a>,
    /// Repository-relative path, forward slashes.
    pub rel: &'s str,
    rule: &'static str,
    findings: &'s mut Vec<Finding>,
    /// Per-file state shared between rules; see [`Context::shared`].
    shared: FxHashMap<TypeId, Rc<dyn Any>>,
}

impl<'s, 'a> Context<'s, 'a> {
    pub fn new(semantic: &'s Semantic<'a>, rel: &'s str, findings: &'s mut Vec<Finding>) -> Self {
        Self { semantic, rel, rule: "", findings, shared: FxHashMap::default() }
    }

    /// Per-file state shared by every rule that asks for a `T`: built by `init`
    /// on first use, then reused for the rest of the file.
    pub fn shared<T: Any>(&mut self, init: impl FnOnce(&Semantic<'a>) -> T) -> Rc<T> {
        let semantic = self.semantic;
        let state = self.shared.entry(TypeId::of::<T>()).or_insert_with(|| Rc::new(init(semantic)));

        Rc::clone(state).downcast::<T>().unwrap_or_else(|_| unreachable!("shared state is keyed by its type"))
    }

    pub fn source(&self) -> &'a str {
        self.semantic.source_text()
    }

    /// Point the context at the rule about to run.
    pub fn enter(&mut self, rule: &'static str) {
        self.rule = rule;
    }

    pub fn report(&mut self, span: Span, message: impl Into<String>) {
        self.report_with_fix(span, message, Vec::new());
    }

    pub fn report_with_fix(&mut self, span: Span, message: impl Into<String>, fix: Vec<Edit>) {
        self.findings.push(Finding { rule: self.rule, span, message: message.into(), fix });
    }
}

pub trait Rule: Send + Sync {
    /// The full name, `plugin/rule`, as written in `[lint.rules]`.
    fn name(&self) -> &'static str;

    /// Called for every node in source order.
    fn run<'a>(&self, node: &AstNode<'a>, ctx: &mut Context<'_, 'a>) {
        let _ = (node, ctx);
    }

    /// Called once per file, before the node walk.
    fn run_once(&self, ctx: &mut Context<'_, '_>) {
        let _ = ctx;
    }
}

/// Builds a rule from its options: the elements after the severity in
/// `[severity, options...]`, or an empty slice.
pub type Factory = fn(&[serde_json::Value]) -> Result<Box<dyn Rule>, String>;

/// Every native rule, by full name.
pub fn registry() -> impl Iterator<Item = (&'static str, Factory)> {
    anti_slop::RULES.iter().chain(nkzw::RULES).chain(no_only_tests::RULES).chain(perfectionist::RULES).copied()
}

/// Run `rules` over one file.
pub fn run_all(rules: &[Box<dyn Rule>], semantic: &Semantic<'_>, rel: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut ctx = Context::new(semantic, rel, &mut findings);

    for rule in rules {
        ctx.enter(rule.name());
        rule.run_once(&mut ctx);
    }

    for node in semantic.nodes().iter() {
        for rule in rules {
            ctx.enter(rule.name());
            rule.run(node, &mut ctx);
        }
    }

    findings
}
