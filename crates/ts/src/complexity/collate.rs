//! The tie-break v1 used when two keys start on the same line:
//! `String.prototype.localeCompare` under the ICU root collation.
//!
//! ASCII follows ICU exactly: punctuation and symbols in ICU's order, then
//! digits, then letters compared case-insensitively first, with lower case
//! before upper case only when the letters tie. Characters outside printable
//! ASCII sort after it by code point, an approximation that only matters for
//! two same-line functions whose names differ there.

use std::cmp::Ordering;

/// The ICU root order of the printable ASCII characters that are not letters.
const ASCII_ORDER: &[u8] = b" _-,;:!?.'\"()[]{}@*/\\&#%`^+<=>|~$0123456789";

/// Compare two names the way v1's `localeCompare` did.
pub fn compare(left: &str, right: &str) -> Ordering {
    left.chars()
        .map(primary)
        .cmp(right.chars().map(primary))
        .then_with(|| left.chars().map(char::is_uppercase).cmp(right.chars().map(char::is_uppercase)))
        .then_with(|| left.cmp(right))
}

/// The primary weight: case-folded, with ASCII placed in ICU order.
fn primary(c: char) -> u32 {
    let lowered = c.to_ascii_lowercase();

    if lowered.is_ascii_lowercase() {
        return 128 + u32::from(lowered);
    }

    match ASCII_ORDER.iter().position(|&b| char::from(b) == lowered) {
        Some(rank) => u32::try_from(rank).unwrap_or(0),
        None => 256 + u32::from(c),
    }
}
