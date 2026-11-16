//! `anti-slop-martinfowler/*`: code smells from Martin Fowler's *Refactoring*
//! that a syntax tree can show: long functions and parameter lists, shared
//! mutable state, collection-building loops, message chains, mysterious names
//! and repeated switches. Ported from the k-mistele/anti-slop fork
//! (<https://github.com/k-mistele/anti-slop>). Every rule is off unless
//! `[lint.rules]` enables it.

mod global_data;
mod long_function;
mod long_parameter_list;
mod loops;
mod message_chains;
mod mutable_data;
mod mysterious_name;
mod repeated_switches;
mod shared;
#[cfg(test)]
#[allow(clippy::too_many_lines, reason = "the generated tables keep one test per upstream option set")]
mod tests;

use super::Factory;

/// Every rule, by full name. Each takes upstream's options object.
pub const RULES: &[(&str, Factory)] = &[
    (global_data::NAME, global_data::build),
    (long_function::NAME, long_function::build),
    (long_parameter_list::NAME, long_parameter_list::build),
    (loops::NAME, loops::build),
    (message_chains::NAME, message_chains::build),
    (mutable_data::NAME, mutable_data::build),
    (mysterious_name::NAME, mysterious_name::build),
    (repeated_switches::NAME, repeated_switches::build),
];
