//! Helpers shared by the tests.

use crate::{Config, LineIndex, Linter, TextRange};

/// A range as `L:C-L:C`: 1-based line, 1-based character column, end exclusive.
pub(crate) fn show(source: &str, range: TextRange) -> String {
    let index = LineIndex::new(source);
    let (start, end) = (index.position(range.start), index.position(range.end));
    format!(
        "{}:{}-{}:{}",
        start.line + 1,
        start.chars + 1,
        end.line + 1,
        end.chars + 1
    )
}

/// Every diagnostic of `source` as `L:C-L:C rule message`.
pub(crate) fn lint_all(source: &str, config: Config) -> Vec<String> {
    let linter = Linter::new(config, None).expect("valid config");
    linter
        .lint(source)
        .diagnostics
        .iter()
        .map(|d| format!("{} {} {}", show(source, d.range), d.rule, d.message))
        .collect()
}

/// Front-matter text (without delimiters) as a whole file: `---\n{fm}---\nBody.\n`.
///
/// A missing final newline is added, so the closing `---` is always on its own line.
pub(crate) fn wrap(fm: &str) -> String {
    let newline = if fm.is_empty() || fm.ends_with('\n') {
        ""
    } else {
        "\n"
    };
    format!("---\n{fm}{newline}---\nBody.\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn show_counts_characters() {
        let source = "a😀b\nc";
        assert_eq!(show(source, TextRange { start: 1, end: 5 }), "1:2-1:3");
        assert_eq!(show(source, TextRange { start: 5, end: 7 }), "1:3-2:1");
    }

    #[test]
    fn wrap_adds_missing_newline() {
        assert_eq!(wrap("a: 1\n"), "---\na: 1\n---\nBody.\n");
        assert_eq!(wrap("a: 1"), "---\na: 1\n---\nBody.\n");
        assert_eq!(wrap(""), "---\n---\nBody.\n");
    }
}
