//! Table tests for the `anti-slop-effect/*` rules, run through the anti-slop
//! [`check`](super::super::anti_slop::tests::check) harness.

mod no_manual_effect_error_tag;
mod no_manual_tag_comparison;
mod no_manual_tagged_construction;
mod no_manual_tagged_type;
mod no_ordie_in_service_construction;
mod no_service_constructor_imports;
mod no_service_parameters;
mod no_test_code_in_src;
mod prefer_effect_match;
mod prefer_tagged_error_handling;
mod prefer_yield_service;
mod require_cause_when_narrowing;
mod require_operation_span;
mod require_service_make_layer;

use super::RULES;

#[test]
fn every_rule_is_registered() {
    let names: Vec<&str> = RULES.iter().map(|(name, _)| *name).collect();

    assert_eq!(
        names,
        [
            "anti-slop-effect/no-manual-effect-error-tag",
            "anti-slop-effect/no-manual-tag-comparison",
            "anti-slop-effect/no-manual-tagged-construction",
            "anti-slop-effect/no-manual-tagged-type",
            "anti-slop-effect/no-ordie-in-service-construction",
            "anti-slop-effect/no-service-constructor-imports",
            "anti-slop-effect/no-service-parameters",
            "anti-slop-effect/no-test-code-in-src",
            "anti-slop-effect/prefer-effect-match",
            "anti-slop-effect/prefer-tagged-error-handling",
            "anti-slop-effect/prefer-yield-service",
            "anti-slop-effect/require-cause-when-narrowing",
            "anti-slop-effect/require-operation-span",
            "anti-slop-effect/require-service-make-layer",
        ]
    );

    for (name, factory) in RULES {
        assert!(factory(&[]).is_ok(), "{name} rejects no options");
    }
}
