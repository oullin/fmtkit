use std::fmt;

/// One replacement of the byte range `[start, end)` of a source text with `text`.
///
/// Offsets are UTF-8 byte offsets into the text the edit was computed against.
/// A zero-width edit (`start == end`) is an insertion.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Edit {
    pub start: u32,
    pub end: u32,
    pub text: String,
}

impl Edit {
    pub fn new(start: u32, end: u32, text: impl Into<String>) -> Self {
        debug_assert!(start <= end, "edit start {start} is after end {end}");

        Self { start, end, text: text.into() }
    }

    pub fn insert(at: u32, text: impl Into<String>) -> Self {
        Self::new(at, at, text)
    }

    /// Whether the two edits touch overlapping bytes. Two insertions at the same
    /// offset, or an insertion at the boundary of a replacement, do not overlap.
    pub fn overlaps(&self, other: &Self) -> bool {
        self.start < other.end && other.start < self.end
    }
}

/// Two edits in one set claimed the same bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditConflict {
    pub first: Edit,
    pub second: Edit,
}

impl fmt::Display for EditConflict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "overlapping edits [{}, {}) and [{}, {})", self.first.start, self.first.end, self.second.start, self.second.end)
    }
}

impl std::error::Error for EditConflict {}

/// An unordered collection of edits against one source text.
///
/// Applying a set never silently drops an edit: overlapping edits are an
/// [`EditConflict`]. Passes that legitimately produce competing candidates
/// resolve them up front with [`EditSet::retain_non_overlapping`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EditSet {
    edits: Vec<Edit>,
}

impl EditSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, edit: Edit) {
        self.edits.push(edit);
    }

    pub fn extend(&mut self, edits: impl IntoIterator<Item = Edit>) {
        self.edits.extend(edits);
    }

    pub fn is_empty(&self) -> bool {
        self.edits.is_empty()
    }

    pub fn len(&self) -> usize {
        self.edits.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Edit> {
        self.edits.iter()
    }

    /// Drop edits that leave the text unchanged and later exact duplicates,
    /// keeping push order so same-offset insertions still apply in sequence.
    pub fn normalize(&mut self, source: &str) {
        let mut seen = rustc_hash::FxHashSet::default();

        self.edits.retain(|edit| source.get(edit.start as usize..edit.end as usize) != Some(edit.text.as_str()) && seen.insert(edit.clone()));
    }

    /// Keep a greedy maximal set of non-overlapping edits: earlier starts win,
    /// and at the same start the longer edit wins.
    pub fn retain_non_overlapping(&mut self) {
        self.edits.sort_by(|a, b| a.start.cmp(&b.start).then(b.end.cmp(&a.end)));

        let mut kept: Vec<Edit> = Vec::with_capacity(self.edits.len());

        for edit in self.edits.drain(..) {
            if kept.iter().all(|other| !other.overlaps(&edit)) {
                kept.push(edit);
            }
        }

        self.edits = kept;
    }

    /// Apply every edit to `source`, producing the new text.
    ///
    /// Insertions at the same offset are applied in the order they were pushed.
    pub fn apply(&self, source: &str) -> Result<String, EditConflict> {
        let mut ordered: Vec<(usize, &Edit)> = self.edits.iter().enumerate().collect();

        ordered.sort_by(|(ia, a), (ib, b)| a.start.cmp(&b.start).then(a.end.cmp(&b.end)).then(ia.cmp(ib)));

        for pair in ordered.windows(2) {
            let (_, a) = pair[0];
            let (_, b) = pair[1];

            if a.overlaps(b) {
                return Err(EditConflict { first: a.clone(), second: b.clone() });
            }
        }

        let growth: usize = self.edits.iter().map(|e| e.text.len()).sum();
        let mut out = String::with_capacity(source.len() + growth);
        let mut cursor = 0usize;

        for (_, edit) in ordered {
            let start = edit.start as usize;
            let end = edit.end as usize;

            out.push_str(&source[cursor..start]);
            out.push_str(&edit.text);
            cursor = end;
        }

        out.push_str(&source[cursor..]);

        Ok(out)
    }
}

impl FromIterator<Edit> for EditSet {
    fn from_iter<T: IntoIterator<Item = Edit>>(iter: T) -> Self {
        Self { edits: iter.into_iter().collect() }
    }
}

impl IntoIterator for EditSet {
    type Item = Edit;
    type IntoIter = std::vec::IntoIter<Edit>;

    fn into_iter(self) -> Self::IntoIter {
        self.edits.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applies_replacements_and_insertions() {
        let set: EditSet = [Edit::new(0, 3, "let"), Edit::insert(9, "\n")].into_iter().collect();

        assert_eq!(set.apply("var a = 1;b").unwrap(), "let a = 1\n;b");
    }

    #[test]
    fn insertions_at_one_offset_keep_push_order() {
        let set: EditSet = [Edit::insert(1, "a"), Edit::insert(1, "b")].into_iter().collect();

        assert_eq!(set.apply("xy").unwrap(), "xaby");
    }

    #[test]
    fn rejects_overlap() {
        let set: EditSet = [Edit::new(0, 4, "a"), Edit::new(2, 6, "b")].into_iter().collect();

        assert!(set.apply("0123456789").is_err());
    }

    #[test]
    fn retain_non_overlapping_prefers_earlier_then_longer() {
        let mut set: EditSet = [Edit::new(2, 6, "b"), Edit::new(0, 4, "a"), Edit::new(0, 8, "c"), Edit::new(8, 9, "d")].into_iter().collect();

        set.retain_non_overlapping();

        assert_eq!(set.iter().map(|e| e.text.as_str()).collect::<Vec<_>>(), ["c", "d"]);
    }

    #[test]
    fn normalize_drops_no_ops_and_duplicates() {
        let mut set: EditSet = [Edit::new(0, 1, "a"), Edit::new(1, 2, "x"), Edit::new(1, 2, "x")].into_iter().collect();

        set.normalize("ab");

        assert_eq!(set.len(), 1);
    }
}
