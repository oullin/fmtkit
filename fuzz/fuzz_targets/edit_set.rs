//! `EditSet` over arbitrary sources and edits clamped to char boundaries.
//!
//! - `apply` returns `Ok` exactly when no two edits overlap, and a conflict
//!   names two edits that really do overlap.
//! - After `retain_non_overlapping`, `apply` always succeeds.
//! - `normalize` is idempotent.

#![no_main]

use arbitrary::Arbitrary;
use fmtkit_core::{Edit, EditSet};
use libfuzzer_sys::fuzz_target;

#[derive(Arbitrary, Debug)]
struct RawEdit {
    start: u32,
    len: u16,
    text: String,
}

#[derive(Arbitrary, Debug)]
struct Input {
    source: String,
    edits: Vec<RawEdit>,
}

fn clamp(source: &str, raw: RawEdit) -> Edit {
    let len = source.len();
    let start = source.floor_char_boundary(raw.start as usize % (len + 1));
    let end = source.floor_char_boundary((start + usize::from(raw.len)).min(len));

    Edit::new(to_u32(start), to_u32(end), raw.text)
}

fn to_u32(offset: usize) -> u32 {
    u32::try_from(offset).expect("fuzz inputs are far below 4 GiB")
}

fn any_overlap(set: &EditSet) -> bool {
    let edits: Vec<&Edit> = set.iter().collect();

    edits.iter().enumerate().any(|(i, a)| edits[i + 1..].iter().any(|b| a.overlaps(b)))
}

fuzz_target!(|input: Input| {
    let Input { source, edits } = input;
    let set: EditSet = edits.into_iter().map(|raw| clamp(&source, raw)).collect();

    match set.apply(&source) {
        Ok(_) => assert!(!any_overlap(&set), "apply accepted overlapping edits: {set:?}"),
        Err(conflict) => {
            assert!(conflict.first.overlaps(&conflict.second), "conflict between edits that do not overlap: {conflict:?}");
            assert!(any_overlap(&set));
        }
    }

    let mut normalized = set.clone();

    normalized.normalize(&source);

    let mut twice = normalized.clone();

    twice.normalize(&source);
    assert_eq!(twice, normalized, "normalize is not idempotent");

    let mut retained = set;

    retained.retain_non_overlapping();
    assert!(!any_overlap(&retained), "retain_non_overlapping kept overlapping edits: {retained:?}");

    if let Err(conflict) = retained.apply(&source) {
        panic!("apply failed after retain_non_overlapping: {conflict}");
    }
});
