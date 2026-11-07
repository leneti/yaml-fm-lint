//! Byte offset to line and column conversion.

use crate::TextRange;

/// A zero-based line and the column counted three ways from the line start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    /// Zero-based line number.
    pub line: usize,
    /// Column in UTF-8 bytes.
    pub utf8: usize,
    /// Column in UTF-16 code units.
    pub utf16: usize,
    /// Column in Unicode scalar values.
    pub chars: usize,
}

/// Line start offsets of a text; a line ends at `\n`, and a `\r` before it is not content.
#[derive(Debug, Clone)]
pub struct LineIndex<'a> {
    text: &'a str,
    starts: Vec<usize>,
}

impl<'a> LineIndex<'a> {
    /// Indexes the lines of `text`.
    pub fn new(text: &'a str) -> Self {
        let starts = std::iter::once(0)
            .chain(text.match_indices('\n').map(|(i, _)| i + 1))
            .collect();
        Self { text, starts }
    }

    /// The number of lines; an empty text has one.
    pub fn line_count(&self) -> usize {
        self.starts.len()
    }

    /// The content of `line` without its line ending, or `None` past the last line.
    pub fn line_range(&self, line: usize) -> Option<TextRange> {
        let start = *self.starts.get(line)?;
        let end = match self.starts.get(line + 1) {
            Some(&next) => {
                let newline = next.saturating_sub(1);
                let cr = newline
                    .checked_sub(1)
                    .and_then(|i| self.text.as_bytes().get(i));
                if cr == Some(&b'\r') {
                    newline - 1
                } else {
                    newline
                }
            }
            None => self.text.len(),
        };
        Some(TextRange { start, end })
    }

    /// The position of `offset`, clamped to the text and down to a character start.
    pub fn position(&self, offset: usize) -> Position {
        let mut offset = offset.min(self.text.len());
        while !self.text.is_char_boundary(offset) {
            offset -= 1;
        }
        let line = self.starts.partition_point(|&s| s <= offset) - 1;
        let TextRange { mut start, end } = self
            .line_range(line)
            .unwrap_or(TextRange { start: 0, end: 0 });
        // A leading BOM is zero-width: columns on the first line count from after it.
        if line == 0 && self.text.starts_with('\u{FEFF}') {
            start = '\u{FEFF}'.len_utf8();
        }
        let before = self.text.get(start..offset.min(end)).unwrap_or_default();
        Position {
            line,
            utf8: before.len(),
            utf16: before.encode_utf16().count(),
            chars: before.chars().count(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pos(line: usize, utf8: usize, utf16: usize, chars: usize) -> Position {
        Position {
            line,
            utf8,
            utf16,
            chars,
        }
    }

    #[test]
    fn lf_lines() {
        assert_eq!(LineIndex::new("ab\ncd").position(3), pos(1, 0, 0, 0));
    }

    #[test]
    fn crlf_cr_not_counted() {
        let idx = LineIndex::new("ab\r\ncd");
        assert_eq!(idx.position(2), pos(0, 2, 2, 2));
        assert_eq!(idx.position(4), pos(1, 0, 0, 0));
        assert_eq!(idx.line_range(0), Some(TextRange { start: 0, end: 2 }));
    }

    #[test]
    fn emoji_columns() {
        assert_eq!(LineIndex::new("😀x").position(4), pos(0, 4, 2, 1));
    }

    #[test]
    fn clamps() {
        let idx = LineIndex::new("😀x");
        assert_eq!(idx.position(2), pos(0, 0, 0, 0));
        assert_eq!(idx.position(99), pos(0, 5, 3, 2));
    }

    #[test]
    fn offset_on_newline_and_line_range_bounds() {
        let idx = LineIndex::new("ab\ncd");
        assert_eq!(idx.position(2), pos(0, 2, 2, 2));
        assert_eq!(idx.line_range(1), Some(TextRange { start: 3, end: 5 }));
        assert_eq!(idx.line_range(2), None);
    }

    #[test]
    fn bom_is_zero_width() {
        let idx = LineIndex::new("\u{FEFF}ab\ncd");
        assert_eq!(idx.position(0), pos(0, 0, 0, 0));
        assert_eq!(idx.position(3), pos(0, 0, 0, 0));
        assert_eq!(idx.position(4), pos(0, 1, 1, 1));
        assert_eq!(idx.position(7), pos(1, 1, 1, 1));
        assert_eq!(idx.line_range(0), Some(TextRange { start: 0, end: 5 }));
    }

    #[test]
    fn empty_text() {
        let idx = LineIndex::new("");
        assert_eq!(idx.position(0), pos(0, 0, 0, 0));
        assert_eq!(idx.line_count(), 1);
    }
}
