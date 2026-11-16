//! Table tests for the anti-slop rules, and the [`check`] harness other rule
//! groups' tables share.
//!
//! Every table for a v1 rule holds v1's own test cases plus edge cases, with
//! the findings the v1 plugin produced for them under oxlint 1.86: byte spans
//! and exact messages. Tables named `upstream` hold the test cases of the
//! k-mistele/anti-slop fork the same way, with the findings that fork's plugin
//! produced. The rules adapted from t3code have no recorded findings; their
//! tables are fmtkit's own. Like v1's rule tester, each case runs one rule
//! alone, and the snippet's own `oxlint-disable` directives apply.

mod max_ternary_depth;
mod no_ambient_nondeterminism;
mod no_array_filter_map;
mod no_chained_type_assertions;
mod no_conditional_empty_object_spread;
mod no_conditional_spread;
mod no_disabled_type_assertion_rules;
mod no_hookless_use_function;
mod no_known_value_widening;
mod no_low_value_comments;
mod no_module_mocking;
mod no_object_parameters;
mod no_polling_loop_in_tests;
mod no_real_time_wait_in_tests;
mod no_reduce_accumulator_copy;
mod no_reflect_apply;
mod no_reflect_get;
mod no_reprovide_ambient_service;
mod no_runtime_typeof;
mod no_shape_in_symbol_names;
mod no_test_in_loop;
mod no_unknown_parameters;
mod no_unknown_returns;
mod no_unknown_type_aliases;
mod no_unreasoned_test_skip;
mod no_unsafe_dictionary_type;
mod no_widen_then_assert;
mod require_readable_spacing;
mod require_safety_comment_for_type_assertion;
mod require_suppression_reason;

use std::fmt::Write as _;
use std::path::Path;
use std::sync::Arc;

use oxc_allocator::Allocator;
use oxc_linter::{ContextSubHost, ContextSubHostOptions, ModuleRecord};
use oxc_parser::{ParseOptions, Parser};
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use serde_json::Value;

use super::super::{registry, run_all};
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
    let rule_options: Vec<Value> = serde_json::from_str(options).expect("options are a JSON array");
    let factory = registry().find(|(name, _)| *name == rule).map_or_else(|| panic!("{rule} is not registered"), |(_, factory)| factory);
    let mut failures = String::new();

    for case in cases {
        let rules = vec![factory(&rule_options).unwrap_or_else(|error| panic!("{rule} rejects {rule_options:?}: {error}"))];
        let allocator = Allocator::default();
        let source_type = SourceType::from_path(case.path).expect("a TypeScript path");
        // The options oxlint parses with.
        let options = ParseOptions { parse_regular_expression: true, allow_return_outside_function: true, ..ParseOptions::default() };
        let parsed = Parser::new(&allocator, case.code, source_type).with_options(options).parse();

        assert!(parsed.diagnostics.is_empty(), "{}: parse errors {:?}", case.name, parsed.diagnostics);

        let built = SemanticBuilder::new_linter().build(&parsed.program);

        assert!(built.diagnostics.is_empty(), "{}: semantic errors {:?}", case.name, built.diagnostics);

        // Like the bridge, and like oxlint for v1, honor the file's own suppression directives.
        let record = Arc::new(ModuleRecord::new(Path::new(case.path), &parsed.module_record, &built.semantic));
        let host = ContextSubHost::new(built.semantic, record, 0, ContextSubHostOptions::default());
        let mut actual: Vec<(u32, u32, String)> = run_all(&rules, host.semantic(), case.path)
            .into_iter()
            .filter(|finding| !host.disable_directives().contains(finding.rule, finding.span))
            .map(|finding| (finding.span.start, finding.span.end, finding.message))
            .collect();

        actual.sort();

        let mut expected: Vec<(u32, u32, String)> = case.expected.iter().map(|&(start, end, message)| (start, end, message.to_owned())).collect();

        expected.sort();

        if actual != expected {
            let _ = write!(failures, "\n--- {}\n{}\nexpected: {expected:#?}\nactual: {actual:#?}\n", case.name, case.code);
        }
    }

    assert!(failures.is_empty(), "{rule}:{failures}");
}

#[test]
fn every_rule_is_registered() {
    let names: Vec<&str> = RULES.iter().map(|(name, _)| *name).collect();

    assert_eq!(
        names,
        [
            "anti-slop/max-ternary-depth",
            "anti-slop/no-ambient-nondeterminism",
            "anti-slop/no-array-filter-map",
            "anti-slop/no-chained-type-assertions",
            "anti-slop/no-conditional-empty-object-spread",
            "anti-slop/no-conditional-spread",
            "anti-slop/no-disabled-type-assertion-rules",
            "anti-slop/no-hookless-use-function",
            "anti-slop/no-known-value-widening",
            "anti-slop/no-low-value-comments",
            "anti-slop/no-module-mocking",
            "anti-slop/no-object-parameters",
            "anti-slop/no-polling-loop-in-tests",
            "anti-slop/no-real-time-wait-in-tests",
            "anti-slop/no-reduce-accumulator-copy",
            "anti-slop/no-reflect-apply",
            "anti-slop/no-reflect-get",
            "anti-slop/no-reprovide-ambient-service",
            "anti-slop/no-runtime-typeof",
            "anti-slop/no-shape-in-symbol-names",
            "anti-slop/no-test-in-loop",
            "anti-slop/no-unknown-parameters",
            "anti-slop/no-unknown-returns",
            "anti-slop/no-unknown-type-aliases",
            "anti-slop/no-unreasoned-test-skip",
            "anti-slop/no-unsafe-dictionary-type",
            "anti-slop/no-widen-then-assert",
            "anti-slop/require-readable-spacing",
            "anti-slop/require-safety-comment-for-type-assertion",
            "anti-slop/require-suppression-reason",
        ]
    );

    for (name, factory) in RULES {
        assert!(factory(&[]).is_ok(), "{name} rejects no options");
    }
}
