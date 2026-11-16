//! `anti-slop-effect/*`: rules for Effect programs: tagged errors and values,
//! services and their layers, and the spans and causes that keep failures
//! traceable. Ported from the k-mistele/anti-slop fork
//! (<https://github.com/k-mistele/anti-slop>). Every rule is off unless
//! `[lint.rules]` enables it.

mod shared;

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

#[cfg(test)]
#[allow(clippy::too_many_lines, reason = "the generated tables keep one test per upstream option set")]
mod tests;

use super::Factory;

/// Every rule, by full name. `no-test-code-in-src` and
/// `require-service-make-layer` take options; the others ignore any they are
/// given.
pub const RULES: &[(&str, Factory)] = &[
    (no_manual_effect_error_tag::NAME, |_| Ok(Box::new(no_manual_effect_error_tag::NoManualEffectErrorTag))),
    (no_manual_tag_comparison::NAME, |_| Ok(Box::new(no_manual_tag_comparison::NoManualTagComparison))),
    (no_manual_tagged_construction::NAME, |_| Ok(Box::new(no_manual_tagged_construction::NoManualTaggedConstruction))),
    (no_manual_tagged_type::NAME, |_| Ok(Box::new(no_manual_tagged_type::NoManualTaggedType))),
    (no_ordie_in_service_construction::NAME, |_| Ok(Box::new(no_ordie_in_service_construction::NoOrDieInServiceConstruction))),
    (no_service_constructor_imports::NAME, |_| Ok(Box::new(no_service_constructor_imports::NoServiceConstructorImports))),
    (no_service_parameters::NAME, |_| Ok(Box::new(no_service_parameters::NoServiceParameters))),
    (no_test_code_in_src::NAME, no_test_code_in_src::build),
    (prefer_effect_match::NAME, |_| Ok(Box::new(prefer_effect_match::PreferEffectMatch))),
    (prefer_tagged_error_handling::NAME, |_| Ok(Box::new(prefer_tagged_error_handling::PreferTaggedErrorHandling))),
    (prefer_yield_service::NAME, |_| Ok(Box::new(prefer_yield_service::PreferYieldService))),
    (require_cause_when_narrowing::NAME, |_| Ok(Box::new(require_cause_when_narrowing::RequireCauseWhenNarrowing))),
    (require_operation_span::NAME, |_| Ok(Box::new(require_operation_span::RequireOperationSpan))),
    (require_service_make_layer::NAME, require_service_make_layer::build),
];
