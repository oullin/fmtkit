//! One-based line numbers for the function starts the walk meets.

/// Resolves byte offsets to one-based lines (only `\n` ends a line, as in v1)
/// without indexing the file up front.
///
/// Function starts arrive almost in source order, so each lookup counts the
/// newlines between the previous offset and this one; the walk is linear in
/// the file and allocates nothing.
pub struct LineCursor<'s> {
    bytes: &'s [u8],
    offset: usize,
    line: u32,
}

impl<'s> LineCursor<'s> {
    pub const fn new(source: &'s str) -> Self {
        Self { bytes: source.as_bytes(), offset: 0, line: 1 }
    }

    /// The line `offset` falls on. Offsets past the end clamp to the last line.
    pub fn line_at(&mut self, offset: u32) -> u32 {
        let target = usize::try_from(offset).unwrap_or(usize::MAX).min(self.bytes.len());

        if target >= self.offset {
            self.line += newlines(&self.bytes[self.offset..target]);
        } else {
            self.line -= newlines(&self.bytes[target..self.offset]);
        }

        self.offset = target;

        self.line
    }
}

fn newlines(bytes: &[u8]) -> u32 {
    let count: usize = bytes.iter().map(|&b| usize::from(b == b'\n')).sum();

    u32::try_from(count).unwrap_or(u32::MAX)
}
