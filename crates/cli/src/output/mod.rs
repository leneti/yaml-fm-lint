//! Output formats.

use yaml_fm_lint_core::LineIndex;

use crate::cli::Format;
use crate::run::{FileResult, Summary};

mod compact;

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
// The human format renders as compact until it has its own renderer.
pub fn render(format: Format, results: &[FileResult], _summary: &Summary, _styled: bool) -> String {
    match format {
        Format::Human | Format::Compact | Format::Json | Format::Sarif | Format::Github => {
            compact::render(results)
        }
    }
}
