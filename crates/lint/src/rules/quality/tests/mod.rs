//! Table tests for the `quality` rules, on anti-slop's [`check`] harness.
//!
//! Every expectation was recorded from `@nkzw/oxlint-config` 3.0.0's `quality`
//! plugin under oxlint 1.94.0, running the one rule alone: byte spans and
//! exact messages. The tables replay `anti-slop/<same name>`'s cases, add
//! upstream's `test/type-bindings.test.js` scenario, and add differential
//! probes of the type-aware rules.

mod no_array_filter_map;
mod no_chained_type_assertions;
mod no_conditional_empty_object_spread;
mod no_known_value_widening;
mod no_object_parameters;
mod no_reduce_accumulator_copy;
mod no_reflect_apply;
mod no_reflect_get;
mod no_unknown_returns;
mod no_unknown_type_aliases;
mod no_unsafe_dictionary_type;
mod no_widen_then_assert;

use serde_json::json;

pub use super::super::anti_slop::tests::{Case, check};
use super::RULES;

#[test]
fn every_rule_is_registered() {
    let names: Vec<&str> = RULES.iter().map(|(name, _)| *name).collect();

    assert_eq!(
        names,
        [
            "quality/no-array-filter-map",
            "quality/no-chained-type-assertions",
            "quality/no-conditional-empty-object-spread",
            "quality/no-known-value-widening",
            "quality/no-object-parameters",
            "quality/no-reduce-accumulator-copy",
            "quality/no-reflect-apply",
            "quality/no-reflect-get",
            "quality/no-unknown-returns",
            "quality/no-unknown-type-aliases",
            "quality/no-unsafe-dictionary-type",
            "quality/no-widen-then-assert",
        ]
    );

    for (name, factory) in RULES {
        let rule = factory(&[]).unwrap_or_else(|error| panic!("{name} rejects no options: {error}"));

        assert_eq!(rule.name(), *name, "{name} reports under another name");
        assert!(factory(&[json!({})]).is_err(), "{name} accepts options");
    }
}
