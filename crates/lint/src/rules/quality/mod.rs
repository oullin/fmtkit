//! `@nkzw/oxlint-config` 3.0.0's `quality` plugin
//! (<https://github.com/nkzw-tech/oxlint-config/tree/v3.0.0/quality>), adapted
//! from dmmulroy/anti-slop at 13909c9.
//!
//! The k-mistele fork of anti-slop has the same twelve rules, and fmtkit
//! shipped them as `anti-slop/<name>`. Those names are now aliases of these
//! rules ([`alias_of`]). Seven rules report what the fork reports. The other
//! five resolve type names lexically ([`bindings`], [`dictionary`]), where the
//! fork resolves module-level aliases only, and differ in a few more ways
//! each file's docs describe. The plugin's rules declare no options.

mod bindings;
mod dictionary;
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
mod widening;

#[cfg(test)]
#[allow(clippy::too_many_lines, reason = "the generated tables keep one test per source")]
mod tests;

use super::{Factory, Rule};

/// Every rule, by full name.
pub const RULES: &[(&str, Factory)] = &[
    (no_array_filter_map::NAME, |options| no_options(options, no_array_filter_map::NoArrayFilterMap)),
    (no_chained_type_assertions::NAME, |options| no_options(options, no_chained_type_assertions::NoChainedTypeAssertions)),
    (no_conditional_empty_object_spread::NAME, |options| no_options(options, no_conditional_empty_object_spread::NoConditionalEmptyObjectSpread)),
    (no_known_value_widening::NAME, |options| no_options(options, no_known_value_widening::NoKnownValueWidening)),
    (no_object_parameters::NAME, |options| no_options(options, no_object_parameters::NoObjectParameters)),
    (no_reduce_accumulator_copy::NAME, |options| no_options(options, no_reduce_accumulator_copy::NoReduceAccumulatorCopy)),
    (no_reflect_apply::NAME, |options| no_options(options, no_reflect_apply::NoReflectApply)),
    (no_reflect_get::NAME, |options| no_options(options, no_reflect_get::NoReflectGet)),
    (no_unknown_returns::NAME, |options| no_options(options, no_unknown_returns::NoUnknownReturns)),
    (no_unknown_type_aliases::NAME, |options| no_options(options, no_unknown_type_aliases::NoUnknownTypeAliases)),
    (no_unsafe_dictionary_type::NAME, |options| no_options(options, no_unsafe_dictionary_type::NoUnsafeDictionaryType)),
    (no_widen_then_assert::NAME, |options| no_options(options, no_widen_then_assert::NoWidenThenAssert)),
];

/// The `quality/*` rule an `anti-slop/*` name stands for, if it is one of
/// the twelve that moved here.
pub fn alias_of(name: &str) -> Option<&'static str> {
    let rule = name.strip_prefix("anti-slop/")?;

    RULES.iter().map(|(quality, _)| *quality).find(|quality| quality.strip_prefix("quality/") == Some(rule))
}

/// The `anti-slop/*` alias of a `quality/*` rule.
pub fn alias(quality: &str) -> Option<String> {
    let rule = quality.strip_prefix("quality/")?;

    RULES.iter().any(|(name, _)| *name == quality).then(|| format!("anti-slop/{rule}"))
}

/// The plugin's rules declare no schema, and oxlint rejects options for them.
fn no_options(options: &[serde_json::Value], rule: impl Rule + 'static) -> Result<Box<dyn Rule>, String> {
    if options.is_empty() { Ok(Box::new(rule)) } else { Err("takes no options".into()) }
}
