//! Output formats.

use yaml_fm_lint_core::LineIndex;

use crate::cli::Format;
use crate::run::{FileResult, Summary};

mod compact;
mod human;

/// A place in a file as users see it: 1-based line and column (counted in characters), and the
/// byte offset from the start of the file.
#[derive(Debug, Clone, Copy)]
pub struct Pos {
    pub line: usize,
    pub column: usize,
    #[expect(dead_code, reason = "read by the json format")]
    pub offset: usize,
}

/// The start and exclusive end of each diagnostic of `result`, in order. Every format gets its
/// positions here, from one `LineIndex` per file.
pub fn positions(result: &FileResult) -> Vec<(Pos, Pos)> {
    let index = LineIndex::new(result.text.as_deref().unwrap_or_default());
    let pos = |offset| {
        let at = index.position(offset);
        Pos {
            line: at.line + 1,
            column: at.chars + 1,
            offset,
        }
    };
    result
        .diagnostics
        .iter()
        .map(|d| (pos(d.range.start), pos(d.range.end)))
        .collect()
}

/// The whole output of the run in `format`; `results` are sorted by `display`.
// The other formats render as compact until they have their own renderers.
pub fn render(format: Format, results: &[FileResult], summary: &Summary) -> String {
    match format {
        Format::Human => human::render(results, summary),
        Format::Compact | Format::Json | Format::Sarif | Format::Github => compact::render(results),
    }
}

/// `s` with every control character (U+0000 to U+001F, U+007F to U+009F) written as `\u{..}`.
/// File names and front-matter text are untrusted: a raw escape sequence or line break could
/// spoof the terminal or split a one-line-per-problem report.
pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        push_escaped(&mut out, c);
    }
    out
}

/// Appends `c` to `out`, escaped like [`escape`].
pub fn push_escaped(out: &mut String, c: char) {
    if c.is_control() {
        out.extend(c.escape_unicode());
    } else {
        out.push(c);
    }
}

#[cfg(test)]
mod tests {
    use super::escape;

    #[test]
    fn escapes_control_characters_only() {
        assert_eq!(
            escape("a\u{1b}[0m\n\t\u{7f}\u{9b}b"),
            r"a\u{1b}[0m\u{a}\u{9}\u{7f}\u{9b}b"
        );
        assert_eq!(escape("plain é — ✘ \u{a0}"), "plain é — ✘ \u{a0}");
    }
}
