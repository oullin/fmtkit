//! fmtkit's own `anti-slop` rules: the v1 oxlint JS plugin, ported; the core
//! rules the k-mistele/anti-slop fork added since
//! (<https://github.com/k-mistele/anti-slop>), ported; and rules adapted from
//! t3code's oxlint plugin
//! (<https://github.com/pingdotgg/t3code/tree/main/oxlint-plugin-t3code>).
//!
//! The rules read oxc's AST through [`shared`], which hides the nodes ESTree
//! does not have, so the ported rules report what their plugin reported at
//! the same spans.
//!
//! Twelve rules the k-mistele fork shares with `@nkzw/oxlint-config` 3.0 live
//! in [`super::quality`] instead, and their `anti-slop/` names are aliases.

pub mod shared;

mod max_doc_comment_lines;
mod max_ternary_depth;
mod no_ambient_nondeterminism;
mod no_conditional_spread;
mod no_disabled_type_assertion_rules;
mod no_hookless_use_function;
mod no_low_value_comments;
mod no_module_mocking;
mod no_polling_loop_in_tests;
mod no_real_time_wait_in_tests;
mod no_reprovide_ambient_service;
mod no_runtime_typeof;
mod no_shape_in_symbol_names;
mod no_test_in_loop;
mod no_unknown_parameters;
mod no_unreasoned_test_skip;
mod require_readable_spacing;
mod require_safety_comment_for_type_assertion;
mod require_suppression_reason;

#[cfg(test)]
#[allow(clippy::too_many_lines, reason = "the generated tables keep one test per v1 group")]
pub(crate) mod tests;

use super::Factory;

/// Every rule, by full name. `max-doc-comment-lines`, `max-ternary-depth`,
/// `no-disabled-type-assertion-rules` and `no-runtime-typeof` take options,
/// and `require-readable-spacing` rejects any; the others ignore any they are
/// given, as v1 did.
pub const RULES: &[(&str, Factory)] = &[
    (max_doc_comment_lines::NAME, max_doc_comment_lines::build),
    (max_ternary_depth::NAME, max_ternary_depth::build),
    (no_ambient_nondeterminism::NAME, |_| Ok(Box::new(no_ambient_nondeterminism::NoAmbientNondeterminism))),
    (no_conditional_spread::NAME, |_| Ok(Box::new(no_conditional_spread::NoConditionalSpread))),
    (no_disabled_type_assertion_rules::NAME, no_disabled_type_assertion_rules::build),
    (no_hookless_use_function::NAME, |_| Ok(Box::new(no_hookless_use_function::NoHooklessUseFunction))),
    (no_low_value_comments::NAME, |_| Ok(Box::new(no_low_value_comments::NoLowValueComments))),
    (no_module_mocking::NAME, |_| Ok(Box::new(no_module_mocking::NoModuleMocking))),
    (no_polling_loop_in_tests::NAME, |_| Ok(Box::new(no_polling_loop_in_tests::NoPollingLoopInTests))),
    (no_real_time_wait_in_tests::NAME, |_| Ok(Box::new(no_real_time_wait_in_tests::NoRealTimeWaitInTests))),
    (no_reprovide_ambient_service::NAME, |_| Ok(Box::new(no_reprovide_ambient_service::NoReprovideAmbientService))),
    (no_runtime_typeof::NAME, no_runtime_typeof::build),
    (no_shape_in_symbol_names::NAME, |_| Ok(Box::new(no_shape_in_symbol_names::NoShapeInSymbolNames))),
    (no_test_in_loop::NAME, |_| Ok(Box::new(no_test_in_loop::NoTestInLoop))),
    (no_unknown_parameters::NAME, |_| Ok(Box::new(no_unknown_parameters::NoUnknownParameters))),
    (no_unreasoned_test_skip::NAME, |_| Ok(Box::new(no_unreasoned_test_skip::NoUnreasonedTestSkip))),
    (require_readable_spacing::NAME, require_readable_spacing::build),
    (require_safety_comment_for_type_assertion::NAME, |_| Ok(Box::new(require_safety_comment_for_type_assertion::RequireSafetyCommentForTypeAssertion))),
    (require_suppression_reason::NAME, |_| Ok(Box::new(require_suppression_reason::RequireSuppressionReason))),
];
