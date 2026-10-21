//! fmtkit's own `anti-slop` rules: the v1 oxlint JS plugin, ported, and
//! rules adapted from t3code's oxlint plugin
//! (<https://github.com/pingdotgg/t3code/tree/main/oxlint-plugin-t3code>).
//!
//! The rules read oxc's AST through [`shared`], which hides the nodes ESTree
//! does not have, so the v1 rules report what v1 reported at the same spans.

pub mod shared;

mod no_ambient_nondeterminism;
mod no_chained_type_assertions;
mod no_conditional_empty_object_spread;
mod no_hookless_use_function;
mod no_known_value_widening;
mod no_low_value_comments;
mod no_module_mocking;
mod no_object_parameters;
mod no_polling_loop_in_tests;
mod no_real_time_wait_in_tests;
mod no_reflect_apply;
mod no_reflect_get;
mod no_runtime_typeof;
mod no_shape_in_symbol_names;
mod no_test_in_loop;
mod no_unknown_parameters;
mod no_unknown_returns;
mod no_unknown_type_aliases;
mod no_unreasoned_test_skip;
mod no_unsafe_dictionary_type;
mod no_widen_then_assert;
mod require_safety_comment_for_type_assertion;
mod require_suppression_reason;

#[cfg(test)]
#[allow(clippy::too_many_lines, reason = "the generated tables keep one test per v1 group")]
mod tests;

use super::Factory;

/// Every rule, by full name. Only `no-runtime-typeof` takes options; the
/// others ignore any they are given, as v1 did.
pub const RULES: &[(&str, Factory)] = &[
    (no_ambient_nondeterminism::NAME, |_| Ok(Box::new(no_ambient_nondeterminism::NoAmbientNondeterminism))),
    (no_chained_type_assertions::NAME, |_| Ok(Box::new(no_chained_type_assertions::NoChainedTypeAssertions))),
    (no_conditional_empty_object_spread::NAME, |_| Ok(Box::new(no_conditional_empty_object_spread::NoConditionalEmptyObjectSpread))),
    (no_hookless_use_function::NAME, |_| Ok(Box::new(no_hookless_use_function::NoHooklessUseFunction))),
    (no_known_value_widening::NAME, |_| Ok(Box::new(no_known_value_widening::NoKnownValueWidening))),
    (no_low_value_comments::NAME, |_| Ok(Box::new(no_low_value_comments::NoLowValueComments))),
    (no_module_mocking::NAME, |_| Ok(Box::new(no_module_mocking::NoModuleMocking))),
    (no_object_parameters::NAME, |_| Ok(Box::new(no_object_parameters::NoObjectParameters))),
    (no_polling_loop_in_tests::NAME, |_| Ok(Box::new(no_polling_loop_in_tests::NoPollingLoopInTests))),
    (no_real_time_wait_in_tests::NAME, |_| Ok(Box::new(no_real_time_wait_in_tests::NoRealTimeWaitInTests))),
    (no_reflect_apply::NAME, |_| Ok(Box::new(no_reflect_apply::NoReflectApply))),
    (no_reflect_get::NAME, |_| Ok(Box::new(no_reflect_get::NoReflectGet))),
    (no_runtime_typeof::NAME, no_runtime_typeof::build),
    (no_shape_in_symbol_names::NAME, |_| Ok(Box::new(no_shape_in_symbol_names::NoShapeInSymbolNames))),
    (no_test_in_loop::NAME, |_| Ok(Box::new(no_test_in_loop::NoTestInLoop))),
    (no_unknown_parameters::NAME, |_| Ok(Box::new(no_unknown_parameters::NoUnknownParameters))),
    (no_unknown_returns::NAME, |_| Ok(Box::new(no_unknown_returns::NoUnknownReturns))),
    (no_unknown_type_aliases::NAME, |_| Ok(Box::new(no_unknown_type_aliases::NoUnknownTypeAliases))),
    (no_unreasoned_test_skip::NAME, |_| Ok(Box::new(no_unreasoned_test_skip::NoUnreasonedTestSkip))),
    (no_unsafe_dictionary_type::NAME, |_| Ok(Box::new(no_unsafe_dictionary_type::NoUnsafeDictionaryType))),
    (no_widen_then_assert::NAME, |_| Ok(Box::new(no_widen_then_assert::NoWidenThenAssert))),
    (require_safety_comment_for_type_assertion::NAME, |_| Ok(Box::new(require_safety_comment_for_type_assertion::RequireSafetyCommentForTypeAssertion))),
    (require_suppression_reason::NAME, |_| Ok(Box::new(require_suppression_reason::RequireSuppressionReason))),
];
