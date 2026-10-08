//! Table tests for the anti-slop rules.
//!
//! Every table holds v1's own test cases plus edge cases, with the findings
//! the v1 plugin produced for them under oxlint 1.86: byte spans and exact
//! messages. Like v1's rule tester, each case runs one rule alone.

mod no_ambient_nondeterminism;
mod no_chained_type_assertions;
mod no_conditional_empty_object_spread;
mod no_known_value_widening;
mod no_module_mocking;
mod no_object_parameters;
mod no_reflect_apply;
mod no_reflect_get;
mod no_runtime_typeof;
mod no_shape_in_symbol_names;
mod no_unknown_parameters;
mod no_unknown_returns;
mod no_unknown_type_aliases;
mod no_unsafe_dictionary_type;
mod no_widen_then_assert;
mod require_safety_comment_for_type_assertion;
mod require_suppression_reason;

use std::fmt::Write as _;
use std::path::Path;

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use serde_json::Value;

use super::super::run_all;
use super::RULES;

/// One snippet and the findings v1 reports for it: `(start, end, message)`.
pub struct Case {
    pub name: &'static str,
    pub path: &'static str,
    pub code: &'static str,
    pub expected: &'static [(u32, u32, &'static str)],
}

/// Run `rule`, configured with the JSON array `options`, over every case and
/// fail with every mismatch at once.
pub fn check(rule: &str, options: &str, cases: &[Case]) {
    let options: Vec<Value> = serde_json::from_str(options).expect("options are a JSON array");
    let factory = RULES.iter().find(|(name, _)| *name == rule).map(|(_, factory)| *factory).unwrap_or_else(|| panic!("{rule} is not registered"));
    let mut failures = String::new();

    for case in cases {
        let rules = vec![factory(&options).unwrap_or_else(|error| panic!("{rule} rejects {options:?}: {error}"))];
        let allocator = Allocator::default();
        let source_type = SourceType::from_path(case.path).expect("a TypeScript path");
        let parsed = Parser::new(&allocator, case.code, source_type).parse();

        assert!(parsed.diagnostics.is_empty(), "{}: parse errors {:?}", case.name, parsed.diagnostics);

        let built = SemanticBuilder::new_linter().build(&parsed.program);

        assert!(built.diagnostics.is_empty(), "{}: semantic errors {:?}", case.name, built.diagnostics);

        let mut actual: Vec<(u32, u32, String)> =
            run_all(&rules, &built.semantic, case.path).into_iter().map(|finding| (finding.span.start, finding.span.end, finding.message)).collect();

        actual.sort();

        let expected: Vec<(u32, u32, String)> = case.expected.iter().map(|&(start, end, message)| (start, end, message.to_owned())).collect();

        if actual != expected {
            let _ = write!(failures, "\n--- {}\n{}\nexpected: {expected:#?}\nactual: {actual:#?}\n", case.name, case.code);
        }
    }

    assert!(failures.is_empty(), "{rule}:{failures}");
}

#[test]
fn every_v1_rule_is_registered() {
    let names: Vec<&str> = RULES.iter().map(|(name, _)| *name).collect();

    assert_eq!(
        names,
        [
            "anti-slop/no-ambient-nondeterminism",
            "anti-slop/no-chained-type-assertions",
            "anti-slop/no-conditional-empty-object-spread",
            "anti-slop/no-known-value-widening",
            "anti-slop/no-module-mocking",
            "anti-slop/no-object-parameters",
            "anti-slop/no-reflect-apply",
            "anti-slop/no-reflect-get",
            "anti-slop/no-runtime-typeof",
            "anti-slop/no-shape-in-symbol-names",
            "anti-slop/no-unknown-parameters",
            "anti-slop/no-unknown-returns",
            "anti-slop/no-unknown-type-aliases",
            "anti-slop/no-unsafe-dictionary-type",
            "anti-slop/no-widen-then-assert",
            "anti-slop/require-safety-comment-for-type-assertion",
            "anti-slop/require-suppression-reason",
        ]
    );

    for (name, factory) in RULES {
        assert!(factory(&[]).is_ok(), "{name} rejects no options");
    }
}
