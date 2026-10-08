/// Maps byte offsets to 1-based line and column numbers.
#[derive(Debug, Clone)]
pub struct LineIndex {
    starts: Vec<u32>,
}

impl LineIndex {
    pub fn new(text: &str) -> Self {
        let mut starts = vec![0];

        starts.extend(memchr_newlines(text).map(|i| u32::try_from(i + 1).unwrap_or(u32::MAX)));

        Self { starts }
    }

    /// The 1-based line containing `offset`.
    pub fn line(&self, offset: u32) -> u32 {
        let index = self.starts.partition_point(|&start| start <= offset);

        u32::try_from(index).unwrap_or(u32::MAX).max(1)
    }

    /// The 1-based line and 1-based byte column of `offset`.
    pub fn line_col(&self, offset: u32) -> (u32, u32) {
        let line = self.line(offset);
        let start = self.starts[(line - 1) as usize];

        (line, offset - start + 1)
    }

    /// The byte offset where the 1-based `line` starts, if it exists.
    pub fn line_start(&self, line: u32) -> Option<u32> {
        line.checked_sub(1).and_then(|i| self.starts.get(i as usize).copied())
    }
}

fn memchr_newlines(text: &str) -> impl Iterator<Item = usize> + '_ {
    text.bytes().enumerate().filter_map(|(i, b)| (b == b'\n').then_some(i))
}

#[cfg(test)]
mod tests {
    use super::LineIndex;

    #[test]
    fn maps_offsets() {
        let index = LineIndex::new("ab\ncd\n\nx");

        assert_eq!(index.line(0), 1);
        assert_eq!(index.line(2), 1);
        assert_eq!(index.line(3), 2);
        assert_eq!(index.line_col(4), (2, 2));
        assert_eq!(index.line(7), 4);
        assert_eq!(index.line_start(3), Some(6));
    }
}
