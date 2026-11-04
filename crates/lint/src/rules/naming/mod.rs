//! fmtkit's `naming/*` rules: names that repeat what their path already says.
//! Every rule is off unless `[lint.rules]` enables it.

mod no_redundant_name_prefix;
mod words;

use super::Factory;

/// Every rule, by full name.
pub const RULES: &[(&str, Factory)] = &[(no_redundant_name_prefix::NAME, no_redundant_name_prefix::build)];
