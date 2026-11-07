//! Helpers shared by the tests.

use crate::data::build;
use crate::document::Document;
use crate::edit::apply_edits;
use crate::extract::{Extraction, FrontMatter, extract};
use crate::parse::parse;
use crate::{Applicability, Config, LineIndex, Linter, RuleId, TextEdit, TextRange};

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

/// The diagnostics of `rule` for `source` as `L:C-L:C message`.
pub(crate) fn lint_src(rule: RuleId, source: &str, config: Config) -> Vec<String> {
    let linter = Linter::new(config, None).expect("valid config");
    linter
        .lint(source)
        .diagnostics
        .iter()
        .filter(|d| d.rule == rule)
        .map(|d| format!("{} {}", show(source, d.range), d.message))
        .collect()
}

/// The diagnostics of `rule` for front-matter text, wrapped by [`wrap`].
pub(crate) fn lint_rule(rule: RuleId, fm: &str, config: Config) -> Vec<String> {
    lint_src(rule, &wrap(fm), config)
}

/// `source` with every `Automatic` fix applied once.
///
/// Stand-in for `Linter::fix` until Task 11: a fix that overlaps an earlier one is skipped.
pub(crate) fn fix_src(source: &str, config: Config) -> String {
    let linter = Linter::new(config, None).expect("valid config");
    let mut fixes: Vec<Vec<TextEdit>> = linter
        .lint(source)
        .diagnostics
        .into_iter()
        .filter_map(|d| d.fix)
        .filter(|fix| fix.applicability == Applicability::Automatic)
        .map(|fix| fix.edits)
        .filter(|edits| !edits.is_empty())
        .collect();
    fixes.sort_by_key(|edits| edits.first().map(|e| e.range.start));
    let mut cursor = 0;
    let mut edits = Vec::new();
    for fix in fixes {
        let start = fix.first().map_or(0, |e| e.range.start);
        if start >= cursor {
            cursor = fix.last().map_or(cursor, |e| e.range.end);
            edits.extend(fix);
        }
    }
    apply_edits(source, &edits).expect("fix edits apply")
}

/// The front-matter text (without delimiters) after the fixes; the body must be unchanged.
pub(crate) fn fix_fm(fm: &str, config: Config) -> String {
    let fixed = fix_src(&wrap(fm), config);
    fixed
        .strip_prefix("---\n")
        .and_then(|rest| rest.strip_suffix("---\nBody.\n"))
        .expect("delimiters and body unchanged")
        .to_string()
}

/// Parses `source` into its front matter and a [`Document`].
pub(crate) fn document(source: &str, ignore_keys: &[String]) -> (FrontMatter, Document) {
    let Extraction::Found(fm) = extract(source) else {
        panic!("no front matter in {source:?}");
    };
    let root = parse(source, &fm).expect("parses");
    let data = build(&root).expect("builds");
    let doc = Document::new(source, &fm, root, data, ignore_keys);
    (fm, doc)
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
