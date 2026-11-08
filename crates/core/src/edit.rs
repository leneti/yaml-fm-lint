//! Applying text edits.

use crate::TextEdit;

/// Applies sorted, non-overlapping edits to `text`.
///
/// Returns `None` if the edits are unsorted, overlap, go out of bounds or split a character.
pub(crate) fn apply_edits(text: &str, edits: &[TextEdit]) -> Option<String> {
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0;
    for edit in edits {
        let (start, end) = (edit.range.start, edit.range.end);
        if start < cursor || end < start {
            return None;
        }
        out.push_str(text.get(cursor..start)?);
        text.get(start..end)?;
        out.push_str(&edit.new_text);
        cursor = end;
    }
    out.push_str(text.get(cursor..)?);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TextEdit, TextRange};

    fn edit(start: usize, end: usize, new_text: &str) -> TextEdit {
        TextEdit {
            range: TextRange { start, end },
            new_text: new_text.into(),
        }
    }

    #[test]
    fn apply_edits_cases() {
        assert_eq!(
            apply_edits("abcdef", &[edit(1, 3, "X")]).as_deref(),
            Some("aXdef")
        );
        assert_eq!(
            apply_edits("abcdef", &[edit(0, 0, ">"), edit(5, 6, "")]).as_deref(),
            Some(">abcde")
        );
        assert_eq!(
            apply_edits("abcdef", &[edit(1, 4, "X"), edit(3, 5, "Y")]),
            None
        );
        assert_eq!(
            apply_edits("abcdef", &[edit(4, 5, "X"), edit(0, 1, "Y")]),
            None
        );
        assert_eq!(apply_edits("abcdef", &[edit(5, 9, "X")]), None);
        assert_eq!(apply_edits("😀", &[edit(1, 2, "X")]), None);
        assert_eq!(apply_edits("😀", &[edit(0, 4, "X")]).as_deref(), Some("X"));
    }
}
