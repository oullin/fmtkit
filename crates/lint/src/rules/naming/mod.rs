//! fmtkit's `naming/*` rules: names that repeat what their path already says,
//! and names with more words than they need. Every rule is off unless
//! `[lint.rules]` enables it.

mod max_name_words;
mod no_redundant_name_prefix;
mod words;

use super::Factory;

/// Every rule, by full name.
pub const RULES: &[(&str, Factory)] = &[(max_name_words::NAME, max_name_words::build), (no_redundant_name_prefix::NAME, no_redundant_name_prefix::build)];
