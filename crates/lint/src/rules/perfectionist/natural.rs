//! `natural-orderby` 5.0.0's `compare()` for strings, which perfectionist's
//! `natural` sort type uses.
//!
//! Not ported: date detection (`Date.parse`), so date-like strings compare as
//! plain chunks.

use std::cmp::Ordering;
use std::sync::LazyLock;

use icu_collator::CollatorBorrowed;
use regress::Regex;

use super::jsnum;
use super::source::is_js_space;

static NUMBERS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(^0x[\da-fA-F]+$|^([+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?(?!\.\d+)(?=\D|\s|$))|\d+)").expect("a valid pattern"));
static INT_OR_FLOAT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?$").expect("a valid pattern"));
static LEADING_ZERO: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^0+[1-9]{1}[0-9]*$").expect("a valid pattern"));

struct Chunk {
    number: Option<f64>,
    text: String,
}

struct Record {
    number: Option<f64>,
    chunks: Vec<Chunk>,
}

pub fn compare(a: &str, b: &str, collator: &CollatorBorrowed<'static>) -> Ordering {
    if a == b {
        return Ordering::Equal;
    }

    let (a, b) = (record(a), record(b));

    if let (Some(x), Some(y)) = (a.number, b.number) {
        return compare_numbers(x, y);
    }

    compare_chunks(&a.chunks, &b.chunks, collator)
}

fn record(value: &str) -> Record {
    let text: String = value.to_lowercase().trim_matches(is_js_space).to_owned();
    let number = parse_number(&text);
    let source = match number {
        Some(n) if n != 0.0 => jsnum::format(n),
        _ => text,
    };

    Record { number, chunks: chunks(&source) }
}

fn parse_number(value: &str) -> Option<f64> {
    if value.is_empty() {
        return None;
    }

    jsnum::parse(&value.replace('_', ""))
}

fn chunks(value: &str) -> Vec<Chunk> {
    let mut marked = String::with_capacity(value.len() + 8);
    let mut last = 0;

    for found in NUMBERS.find_iter(value) {
        marked.push_str(&value[last..found.start()]);
        marked.push('\0');
        marked.push_str(&value[found.range()]);
        marked.push('\0');
        last = found.end();
    }

    marked.push_str(&value[last..]);

    let marked = marked.strip_suffix('\0').unwrap_or(&marked);
    let marked = marked.strip_prefix('\0').unwrap_or(marked);
    let parts: Vec<&str> = marked.split('\0').collect();

    parts
        .iter()
        .enumerate()
        .map(|(index, part)| {
            let numeric = INT_OR_FLOAT.find(part).is_some() && (LEADING_ZERO.find(part).is_none() || index == 0 || parts[index - 1] != ".");
            let number = numeric.then(|| parse_number(part).filter(|&n| n != 0.0).unwrap_or(0.0));

            Chunk { number, text: part.split(is_js_space).filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" ") }
        })
        .collect()
}

fn compare_chunks(a: &[Chunk], b: &[Chunk], collator: &CollatorBorrowed<'static>) -> Ordering {
    for (x, y) in a.iter().zip(b) {
        if x.text == y.text {
            continue;
        }

        if x.text.is_empty() != y.text.is_empty() {
            return if x.text.is_empty() { Ordering::Less } else { Ordering::Greater };
        }

        return match (x.number, y.number) {
            (Some(m), Some(n)) => match compare_numbers(m, n) {
                Ordering::Equal => compare_units(&x.text, &y.text),
                other => other,
            },
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) if x.text.chars().chain(y.text.chars()).any(|c| u32::from(c) > 0x80) => collator.compare(&x.text, &y.text),
            (None, None) => compare_units(&x.text, &y.text),
        };
    }

    a.len().cmp(&b.len())
}

fn compare_numbers(a: f64, b: f64) -> Ordering {
    a.partial_cmp(&b).unwrap_or(Ordering::Equal)
}

/// JavaScript's `<` on strings: UTF-16 code unit order.
pub fn compare_units(a: &str, b: &str) -> Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
}

#[cfg(test)]
mod tests {
    use icu_collator::options::CollatorOptions;
    use icu_locale_core::locale;

    use super::*;

    #[test]
    fn orders_naturally() {
        let collator = CollatorBorrowed::try_new((&locale!("en-US")).into(), CollatorOptions::default()).expect("collation data");
        let mut values = vec!["item10", "item2", "Item1", "item1a", "1", "_", "a"];

        values.sort_by(|a, b| compare(a, b, &collator));

        assert_eq!(values, ["_", "1", "a", "Item1", "item1a", "item2", "item10"]);
    }
}
