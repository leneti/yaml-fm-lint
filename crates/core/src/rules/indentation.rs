//! `indentation`: a nested block collection is indented by the configured size.
//!
//! # Why
//!
//! Uniform indentation makes the nesting readable and the diffs small. The size is the
//! `indent_size` setting (2 by default).
//!
//! # Examples
//!
//! Bad:
//!
//! ```yaml
//! a:
//!    b: 1
//! ```
//!
//! Good:
//!
//! ```yaml
//! a:
//!   b: 1
//! ```
//!
//! The top-level mapping starts at column 0. A block list under a key may also be written
//! level with the key (`a:` then `- x`). A collection that starts on the same line as its
//! parent's `- ` (`- a: 1`) is not checked, and neither are flow collections and scalars.
//!
//! # Fix
//!
//! Moves the collection and everything inside it (block text and continuation lines
//! included) left or right by the difference. A comment line with fewer leading spaces than
//! it would lose moves to column 0. No fix when any other line has fewer leading spaces
//! than it would lose; the fixer also refuses a shift that would change the data.

use crate::document::{LineKind, Role};
use crate::lint::{Context, Finding};
use crate::tree::{Node, NodeKind};
use crate::{Applicability, DataEffect, Fix, TextEdit, TextRange};

use super::flow_fix::column;

/// Flags every outermost block collection that is not at its expected column.
pub(crate) fn check(ctx: &Context<'_>) -> Vec<Finding> {
    let size = usize::from(ctx.config.indent_size);
    let mut findings = Vec::new();
    // Everything before this offset is inside a collection that is already reported.
    let mut reported_end = 0;
    for r in ctx.doc.nodes() {
        let node = r.node;
        if r.in_flow || node.range.start < reported_end {
            continue;
        }
        let is_sequence = match node.kind {
            NodeKind::Mapping { flow: false, .. } => false,
            NodeKind::Sequence { flow: false, .. } => true,
            _ => continue,
        };
        // (the expected column, and the other column a block list may use)
        let expected = match (r.role, r.entry, r.parent) {
            (Role::Root, ..) if !is_sequence => Some((0, None)),
            // An explicit `? key` entry, or a key with an anchor or tag, has no reliable column.
            (Role::Value, Some(entry), _)
                if !entry.explicit
                    && entry.key.anchor.is_none()
                    && !entry.key.has_tag
                    && entry.key.range.start < entry.key.range.end =>
            {
                column(ctx, entry.key.range.start).map(|key| (key + size, Some(key)))
            }
            (Role::Item, _, Some(seq)) => {
                column(ctx, seq.range.start).map(|dash| (dash + size, None))
            }
            _ => None,
        };
        let Some((expected, unindented)) = expected else {
            continue;
        };
        // Only a collection that is first on its line starts on a later line than its parent.
        let Some(line_index) = ctx.doc.line_index(node.range.start) else {
            continue;
        };
        let Some(line) = ctx.doc.lines.get(line_index) else {
            continue;
        };
        let Some(indent) = ctx.source.get(line.range.start..node.range.start) else {
            continue;
        };
        if !indent.bytes().all(|b| b == b' ') {
            continue;
        }
        let found = indent.len();
        if found == expected || (is_sequence && unindented == Some(found)) {
            continue;
        }
        reported_end = node.range.end;
        findings.push(Finding {
            range: TextRange {
                start: line.range.start,
                end: node.range.start,
            },
            message: format!(
                "expected indentation of {expected} space{}, found {found}",
                if expected == 1 { "" } else { "s" }
            ),
            fix: shift(ctx, node, line_index, expected, found),
        });
    }
    findings
}

/// A fix that moves the lines of `node` from `first_line` on from `found` to `expected` spaces.
///
/// Blank lines stay as they are (a blank line inside a scalar is data either way). An own-line
/// comment with fewer leading spaces than it would lose moves to column 0 (a comment is not
/// data); `None` if any other line has fewer.
fn shift(
    ctx: &Context<'_>,
    node: &Node,
    first_line: usize,
    expected: usize,
    found: usize,
) -> Option<Fix> {
    let last_line = ctx.doc.line_index(node.range.end.saturating_sub(1))?;
    let mut edits = Vec::new();
    for line in ctx.doc.lines.get(first_line..=last_line)? {
        let text = ctx.source.get(line.range.start..line.range.end)?;
        let leading = text.bytes().take_while(|&b| b == b' ').count();
        let blank = leading == text.len();
        if text.is_empty() || (blank && line.kind == LineKind::Structural) {
            continue;
        }
        edits.push(if expected > found {
            TextEdit {
                range: TextRange {
                    start: line.range.start,
                    end: line.range.start,
                },
                new_text: " ".repeat(expected - found),
            }
        } else {
            let lost = found - expected;
            let own_line_comment =
                line.kind == LineKind::Structural && text.get(leading..)?.starts_with('#');
            if leading < lost && !own_line_comment {
                return None;
            }
            if leading == 0 {
                continue;
            }
            TextEdit {
                range: TextRange {
                    start: line.range.start,
                    end: line.range.start + lost.min(leading),
                },
                new_text: String::new(),
            }
        });
    }
    Some(Fix {
        applicability: Applicability::Automatic,
        data_effect: DataEffect::Preserving,
        edits,
    })
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::testing::{fix_fm, fix_src, lint_rule, lint_src, wrap};
    use crate::{Config, Linter, RuleId};

    const RULE: RuleId = RuleId::Indentation;

    fn with_size(indent_size: u8) -> Config {
        Config {
            indent_size,
            ..Config::default()
        }
    }

    #[rstest]
    #[case::three("a:\n   b: 1\n", &["3:1-3:4 expected indentation of 2 spaces, found 3"], "a:\n  b: 1\n")]
    #[case::one("a:\n b: 1\n", &["3:1-3:2 expected indentation of 2 spaces, found 1"], "a:\n  b: 1\n")]
    #[case::subtree(
        "a:\n    b:\n      c: 1\n",
        &["3:1-3:5 expected indentation of 2 spaces, found 4"],
        "a:\n  b:\n    c: 1\n"
    )]
    #[case::seq_too_deep("a:\n    - x\n", &["3:1-3:5 expected indentation of 2 spaces, found 4"], "a:\n  - x\n")]
    #[case::root(
        "  a: 1\n  b: 2\n",
        &["2:1-2:3 expected indentation of 0 spaces, found 2"],
        "a: 1\nb: 2\n"
    )]
    #[case::item_too_deep(
        "l:\n  -\n      a: 1\n",
        &["4:1-4:7 expected indentation of 4 spaces, found 6"],
        "l:\n  -\n    a: 1\n"
    )]
    #[case::nested_in_item(
        "l:\n  - a:\n        b: 1\n",
        &["4:1-4:9 expected indentation of 6 spaces, found 8"],
        "l:\n  - a:\n      b: 1\n"
    )]
    #[case::non_ascii_key(
        "é:\n   b: 1\n",
        &["3:1-3:4 expected indentation of 2 spaces, found 3"],
        "é:\n  b: 1\n"
    )]
    #[case::block_text_moves(
        "a:\n   b: |\n      text\n\n      more\n   c: 1\n",
        &["3:1-3:4 expected indentation of 2 spaces, found 3"],
        "a:\n  b: |\n     text\n\n     more\n  c: 1\n"
    )]
    #[case::multi_line_quoted_moves(
        "a:\n   b: \"x #\n      y\"\n",
        &["3:1-3:4 expected indentation of 2 spaces, found 3"],
        "a:\n  b: \"x #\n     y\"\n"
    )]
    #[case::comment_line_moves(
        "a:\n   b: 1\n   # c\n   d: 2\n",
        &["3:1-3:4 expected indentation of 2 spaces, found 3"],
        "a:\n  b: 1\n  # c\n  d: 2\n"
    )]
    #[case::sequence_of_mappings(
        "l:\n    - a: 1\n      b: 2\n    - c: 3\n",
        &["3:1-3:5 expected indentation of 2 spaces, found 4"],
        "l:\n  - a: 1\n    b: 2\n  - c: 3\n"
    )]
    #[case::explicit_block_indent(
        "a:\n   b: |2\n       text\n   c: 1\n",
        &["3:1-3:4 expected indentation of 2 spaces, found 3"],
        "a:\n  b: |2\n      text\n  c: 1\n"
    )]
    #[case::anchored_value(
        "a: &x\n   - b\n",
        &["3:1-3:4 expected indentation of 2 spaces, found 3"],
        "a: &x\n  - b\n"
    )]
    #[case::sibling_blocks(
        "a:\n   x: 1\nb:\n    y: 2\n",
        &[
            "3:1-3:4 expected indentation of 2 spaces, found 3",
            "5:1-5:5 expected indentation of 2 spaces, found 4",
        ],
        "a:\n  x: 1\nb:\n  y: 2\n"
    )]
    fn flagged_and_fixed(#[case] fm: &str, #[case] want: &[&str], #[case] fixed: &str) {
        assert_eq!(lint_rule(RULE, fm, Config::default()), want);
        assert_eq!(fix_fm(fm, Config::default()), fixed);
    }

    #[rstest]
    #[case::size_4(
        "a:\n  b: 1\n",
        4,
        "3:1-3:3 expected indentation of 4 spaces, found 2",
        "a:\n    b: 1\n"
    )]
    #[case::size_1(
        "a:\n  b: 1\n",
        1,
        "3:1-3:3 expected indentation of 1 space, found 2",
        "a:\n b: 1\n"
    )]
    fn indent_size(#[case] fm: &str, #[case] size: u8, #[case] want: &str, #[case] fixed: &str) {
        assert_eq!(lint_rule(RULE, fm, with_size(size)), [want]);
        assert_eq!(fix_fm(fm, with_size(size)), fixed);
    }

    // Own-line comments are clamped to column 0 instead of blocking the fix.
    #[rstest]
    #[case::comment_at_column_0(
        "a:\n    b: 1\n# c\n    d: 2\n",
        &["3:1-3:5 expected indentation of 2 spaces, found 4"],
        "a:\n  b: 1\n# c\n  d: 2\n"
    )]
    #[case::comment_at_column_1(
        "a:\n    b: 1\n # c\n    d: 2\n",
        &["3:1-3:5 expected indentation of 2 spaces, found 4"],
        "a:\n  b: 1\n# c\n  d: 2\n"
    )]
    #[case::comment_at_column_0_root(
        "  a: 1\n# c\n  b: 2\n",
        &["2:1-2:3 expected indentation of 0 spaces, found 2"],
        "a: 1\n# c\nb: 2\n"
    )]
    fn comments_clamp(#[case] fm: &str, #[case] want: &[&str], #[case] fixed: &str) {
        assert_eq!(lint_rule(RULE, fm, Config::default()), want);
        assert_eq!(fix_fm(fm, Config::default()), fixed);
    }

    // The diagnostic is there, but no fix: a data line would lose more spaces than it has.
    #[rstest]
    #[case::short_line_in_block_text(
        "a:\n    b: |\n        t\n \n        u\n",
        &["3:1-3:5 expected indentation of 2 spaces, found 4"]
    )]
    fn flagged_without_fix(#[case] fm: &str, #[case] want: &[&str]) {
        assert_eq!(lint_rule(RULE, fm, Config::default()), want);
        let report = Linter::new(Config::default(), None)
            .expect("valid config")
            .lint(&wrap(fm));
        let ours = report.diagnostics.iter().filter(|d| d.rule == RULE);
        assert!(ours.clone().count() > 0 && ours.into_iter().all(|d| d.fix.is_none()));
    }

    #[rstest]
    #[case::unindented_seq("a:\n- x\n")]
    #[case::unindented_seq_then_key("a:\n- x\nb: 1\n")]
    #[case::block_text("a: |\n     text\n")]
    #[case::compact("l:\n  - a: 1\n    b: 2\n")]
    #[case::compact_wide_dash("l:\n  -   a: 1\n      b: 2\n")]
    #[case::nested_seq_compact("l:\n  - - a\n    - b\n")]
    #[case::flow("a: {b:    1}\nc: [\n      x]\n")]
    #[case::plain_scalar("a:\n      x\n")]
    #[case::anchored_key("&k a:\n    b: 1\n")]
    #[case::explicit_key("? a\n:\n   b: 1\n")]
    #[case::item_in_key_seq("a:\n  - x\n  - y\n")]
    #[case::blank_lines("a:\n\n  b: 1\n")]
    #[case::empty("")]
    #[case::scalar_root("x\n")]
    fn not_flagged(#[case] fm: &str) {
        assert_eq!(lint_rule(RULE, fm, Config::default()), Vec::<String>::new());
    }

    #[test]
    fn crlf() {
        let source = "---\r\na:\r\n   b: |\r\n      t\r\n   c: 1\r\n---\r\n";
        assert_eq!(
            lint_src(RULE, source, Config::default()),
            ["3:1-3:4 expected indentation of 2 spaces, found 3"]
        );
        assert_eq!(
            fix_src(source, Config::default()),
            "---\r\na:\r\n  b: |\r\n     t\r\n  c: 1\r\n---\r\n"
        );
    }

    #[test]
    fn fixed_file_is_clean_and_deep_nesting_converges() {
        let fm = "a:\n     b:\n          c:\n                d: 1\n";
        let fixed = fix_fm(fm, Config::default());
        assert_eq!(fixed, "a:\n  b:\n    c:\n      d: 1\n");
        assert!(lint_rule(RULE, &fixed, Config::default()).is_empty());
    }
}
