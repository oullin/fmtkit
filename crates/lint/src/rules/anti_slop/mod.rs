//! fmtkit's own `anti-slop` rules, ported from the v1 oxlint JS plugin.
//!
//! The rules read oxc's AST through [`shared`], which hides the nodes ESTree
//! does not have, so they report what v1 reported at the same spans.

pub mod shared;

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

#[cfg(test)]
mod tests;

use super::Factory;

pub const RULES: &[(&str, Factory)] = &[
    (no_ambient_nondeterminism::NAME, no_ambient_nondeterminism::build),
    (no_chained_type_assertions::NAME, no_chained_type_assertions::build),
    (no_conditional_empty_object_spread::NAME, no_conditional_empty_object_spread::build),
    (no_known_value_widening::NAME, no_known_value_widening::build),
    (no_module_mocking::NAME, no_module_mocking::build),
    (no_object_parameters::NAME, no_object_parameters::build),
    (no_reflect_apply::NAME, no_reflect_apply::build),
    (no_reflect_get::NAME, no_reflect_get::build),
    (no_runtime_typeof::NAME, no_runtime_typeof::build),
    (no_shape_in_symbol_names::NAME, no_shape_in_symbol_names::build),
    (no_unknown_parameters::NAME, no_unknown_parameters::build),
    (no_unknown_returns::NAME, no_unknown_returns::build),
    (no_unknown_type_aliases::NAME, no_unknown_type_aliases::build),
    (no_unsafe_dictionary_type::NAME, no_unsafe_dictionary_type::build),
    (no_widen_then_assert::NAME, no_widen_then_assert::build),
    (require_safety_comment_for_type_assertion::NAME, require_safety_comment_for_type_assertion::build),
    (require_suppression_reason::NAME, require_suppression_reason::build),
];
