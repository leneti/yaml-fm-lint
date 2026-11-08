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

/// How far apart, in bytes, the column checkpoints of a non-ASCII line are.
const CHECKPOINT_STRIDE: usize = 64;

/// The character and UTF-16 counts from a line's column origin up to `offset`.
#[derive(Debug, Clone, Copy)]
struct Checkpoint {
    offset: usize,
    chars: usize,
    utf16: usize,
}

/// What a column lookup needs to know about one line.
#[derive(Debug, Clone)]
struct LineCols {
    /// Where columns count from: the line start, or after a leading BOM on the first line.
    origin: usize,
    /// The content up to its end has only ASCII, so every column is the byte distance.
    ascii: bool,
    /// For a non-ASCII line, its slice of `LineIndex::checkpoints`.
    checkpoints: std::ops::Range<usize>,
}

/// Line start offsets of a text; a line ends at `\n`, and a `\r` before it is not content.
///
/// Column lookups cost O(log lines + 64 bytes) however long the line is.
#[derive(Debug, Clone)]
pub struct LineIndex<'a> {
    text: &'a str,
    starts: Vec<usize>,
    cols: Vec<LineCols>,
    checkpoints: Vec<Checkpoint>,
}

impl<'a> LineIndex<'a> {
    /// Indexes the lines of `text`.
    pub fn new(text: &'a str) -> Self {
        let starts = std::iter::once(0)
            .chain(text.match_indices('\n').map(|(i, _)| i + 1))
            .collect();
        let mut index = Self {
            text,
            starts,
            cols: Vec::new(),
            checkpoints: Vec::new(),
        };
        for line in 0..index.starts.len() {
            let cols = index.line_cols(line);
            index.cols.push(cols);
        }
        index
    }

    /// Scans `line` once, recording a checkpoint every [`CHECKPOINT_STRIDE`] bytes if it is
    /// not ASCII.
    fn line_cols(&mut self, line: usize) -> LineCols {
        let TextRange { mut start, end } = self
            .line_range(line)
            .unwrap_or(TextRange { start: 0, end: 0 });
        // A leading BOM is zero-width: columns on the first line count from after it.
        if line == 0 && self.text.starts_with('\u{FEFF}') {
            start = '\u{FEFF}'.len_utf8();
        }
        let content = self.text.get(start..end).unwrap_or_default();
        let first = self.checkpoints.len();
        if !content.is_ascii() {
            let (mut chars, mut utf16, mut next) = (0, 0, CHECKPOINT_STRIDE);
            for (i, c) in content.char_indices() {
                if i >= next {
                    self.checkpoints.push(Checkpoint {
                        offset: start + i,
                        chars,
                        utf16,
                    });
                    next = i + CHECKPOINT_STRIDE;
                }
                chars += 1;
                utf16 += c.len_utf16();
            }
        }
        LineCols {
            origin: start,
            ascii: content.is_ascii(),
            checkpoints: first..self.checkpoints.len(),
        }
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
        let end = self.line_range(line).map_or(0, |r| r.end);
        let (Some(cols), Some(utf8)) = (
            self.cols.get(line),
            offset
                .min(end)
                .checked_sub(self.cols.get(line).map_or(0, |c| c.origin)),
        ) else {
            return Position {
                line,
                utf8: 0,
                utf16: 0,
                chars: 0,
            };
        };
        if cols.ascii {
            return Position {
                line,
                utf8,
                utf16: utf8,
                chars: utf8,
            };
        }
        let target = cols.origin + utf8;
        let own = self
            .checkpoints
            .get(cols.checkpoints.clone())
            .unwrap_or_default();
        let before = own.partition_point(|c| c.offset <= target);
        let from = before
            .checked_sub(1)
            .and_then(|i| own.get(i))
            .copied()
            .unwrap_or(Checkpoint {
                offset: cols.origin,
                chars: 0,
                utf16: 0,
            });
        let rest = self.text.get(from.offset..target).unwrap_or_default();
        Position {
            line,
            utf8,
            utf16: from.utf16 + rest.encode_utf16().count(),
            chars: from.chars + rest.chars().count(),
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

    /// The columns by counting from the line start, as a direct reading of the definition.
    fn naive(text: &str, offset: usize) -> Position {
        let mut offset = offset.min(text.len());
        while !text.is_char_boundary(offset) {
            offset -= 1;
        }
        let line = text[..offset].matches('\n').count();
        let mut start = text[..offset].rfind('\n').map_or(0, |i| i + 1);
        let line_end = text[start..].find('\n').map_or(text.len(), |i| start + i);
        let end = if text[..line_end].ends_with('\r') && line_end < text.len() {
            line_end - 1
        } else {
            line_end
        };
        if line == 0 && text.starts_with('\u{FEFF}') {
            start = 3;
        }
        let before = text.get(start..offset.min(end)).unwrap_or_default();
        pos(
            line,
            before.len(),
            before.encode_utf16().count(),
            before.chars().count(),
        )
    }

    proptest::proptest! {
        #[test]
        fn matches_the_naive_count(parts in proptest::collection::vec(
            proptest::prop_oneof![
                proptest::strategy::Just("a".to_string()),
                proptest::strategy::Just("\r\n".to_string()),
                proptest::strategy::Just("\n".to_string()),
                proptest::strategy::Just("😀".to_string()),
                proptest::strategy::Just("é".to_string()),
                proptest::strategy::Just("\u{FEFF}".to_string()),
                proptest::strategy::Just("x".repeat(70)),
                proptest::strategy::Just("é".repeat(40)),
            ],
            0..30,
        )) {
            let text = parts.concat();
            let idx = LineIndex::new(&text);
            for offset in 0..=text.len() + 1 {
                proptest::prop_assert_eq!(idx.position(offset), naive(&text, offset), "offset {}", offset);
            }
        }
    }

    #[test]
    fn many_positions_on_one_long_non_ascii_line_are_fast() {
        // 500 KB line; 150k lookups. Scanning the line per lookup took tens of seconds.
        let text = format!("é😀{}\n", "ab😀".repeat(100_000));
        let idx = LineIndex::new(&text);
        let started = std::time::Instant::now();
        let mut last = pos(0, 0, 0, 0);
        for offset in (0..text.len()).step_by(3) {
            last = idx.position(offset);
        }
        assert_eq!(last.line, 0);
        assert_eq!(
            idx.position(text.len() - 1),
            pos(0, text.len() - 1, 3 + 4 * 100_000, 2 + 3 * 100_000)
        );
        // Generous for slow runners; the O(line) version takes far longer.
        assert!(started.elapsed() < std::time::Duration::from_secs(20));
    }
}
