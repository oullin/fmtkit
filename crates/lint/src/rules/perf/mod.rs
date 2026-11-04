//! fmtkit's `perf/*` rules: work a function repeats on every call that the
//! module could do once, and queries that read more than the caller uses or
//! run once per item. Every rule is off unless `[lint.rules]` enables it.

mod no_query_in_loop;
mod no_regex_in_function;
mod no_repeated_url_parse;
mod no_select_all;
mod no_stateful_regex_test;
mod prefer_shared_pattern;
mod shared;

use super::Factory;

/// Every rule, by full name. `no-query-in-loop` and `prefer-shared-pattern`
/// take options; the others ignore any they are given.
pub const RULES: &[(&str, Factory)] = &[
    (no_query_in_loop::NAME, no_query_in_loop::build),
    (no_regex_in_function::NAME, |_| Ok(Box::new(no_regex_in_function::NoRegexInFunction))),
    (no_repeated_url_parse::NAME, |_| Ok(Box::new(no_repeated_url_parse::NoRepeatedUrlParse))),
    (no_select_all::NAME, |_| Ok(Box::new(no_select_all::NoSelectAll))),
    (no_stateful_regex_test::NAME, |_| Ok(Box::new(no_stateful_regex_test::NoStatefulRegexTest))),
    (prefer_shared_pattern::NAME, prefer_shared_pattern::build),
];
