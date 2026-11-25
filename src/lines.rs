//! Byte-offset to line/column conversion.

/// Maps byte offsets in a source string to 1-based line and column numbers.
#[derive(Debug, Clone)]
pub struct LineIndex {
    starts: Vec<usize>,
    len: usize,
}

impl LineIndex {
    pub fn new(source: &str) -> Self {
        let mut starts = vec![0];
        for (i, b) in source.bytes().enumerate() {
            if b == b'\n' {
                starts.push(i + 1);
            }
        }
        // A trailing newline does not start a real line.
        if starts.len() > 1 && *starts.last().unwrap() == source.len() {
            starts.pop();
        }
        LineIndex {
            starts,
            len: source.len(),
        }
    }

    /// 1-based line containing `offset`.
    pub fn line(&self, offset: usize) -> usize {
        match self.starts.binary_search(&offset) {
            Ok(i) => i + 1,
            Err(i) => i,
        }
    }

    /// 1-based (line, column) of `offset`; the column counts bytes.
    pub fn line_col(&self, offset: usize) -> (usize, usize) {
        let line = self.line(offset);
        (line, offset - self.starts[line - 1] + 1)
    }

    pub fn line_count(&self) -> usize {
        self.starts.len()
    }

    /// Text of lines `start..=end` (1-based, inclusive), without a trailing newline.
    pub fn slice<'a>(&self, source: &'a str, start: usize, end: usize) -> &'a str {
        let from = self.starts[start - 1];
        let to = if end < self.starts.len() {
            self.starts[end]
        } else {
            self.len
        };
        source[from..to].trim_end_matches('\n')
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_and_slices() {
        let src = "a\nbb\nccc\n";
        let idx = LineIndex::new(src);
        assert_eq!(idx.line_count(), 3);
        assert_eq!(idx.line_col(0), (1, 1));
        assert_eq!(idx.line_col(3), (2, 2));
        assert_eq!(idx.slice(src, 2, 3), "bb\nccc");
    }
}
