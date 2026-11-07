//! `no-blank-lines`: the front matter must not contain blank lines.
//!
//! # Why
//!
//! Blank lines in a short block of metadata are noise, and they are easy to leave
//! behind when keys are moved around.
//!
//! # Examples
//!
//! Bad (the second line is blank):
//!
//! ```yaml
//! title: Hello
//!
//! draft: false
//! ```
//!
//! Good:
//!
//! ```yaml
//! title: Hello
//! draft: false
//! ```
//!
//! # Fix
//!
//! Deletes the blank line together with its line ending. Blank lines inside a
//! block scalar or a multi-line quoted string are text and are left alone.

use crate::document::LineKind;
use crate::lint::{Context, Finding};
use crate::{Applicability, DataEffect, Fix, TextEdit, TextRange};

/// The message for every finding.
const MESSAGE: &str = "blank line";

/// Flags each structural line that is empty or only spaces and tabs.
pub(crate) fn check(ctx: &Context<'_>) -> Vec<Finding> {
    let mut findings = Vec::new();
    for line in &ctx.doc.lines {
        if line.kind != LineKind::Structural {
            continue;
        }
        let Some(text) = ctx.source.get(line.range.start..line.range.end) else {
            continue;
        };
        if !text.chars().all(|c| c == ' ' || c == '\t') {
            continue;
        }
        let rest = ctx.source.get(line.range.end..).unwrap_or("");
        let ending = if rest.starts_with("\r\n") {
            2
        } else {
            usize::from(rest.starts_with('\n'))
        };
        findings.push(Finding {
            range: line.range,
            message: MESSAGE.to_string(),
            fix: Some(Fix {
                applicability: Applicability::Automatic,
                data_effect: DataEffect::Preserving,
                edits: vec![TextEdit {
                    range: TextRange {
                        start: line.range.start,
                        end: line.range.end + ending,
                    },
                    new_text: String::new(),
                }],
            }),
        });
    }
    findings
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::testing::{fix_fm, fix_src, lint_rule, lint_src};
    use crate::{Config, RuleId};

    const RULE: RuleId = RuleId::NoBlankLines;

    #[rstest]
    #[case::empty("a: 1\n\nb: 2\n", &["3:1-3:1 blank line"], "a: 1\nb: 2\n")]
    #[case::whitespace("a: 1\n  \nb: 2\n", &["3:1-3:3 blank line"], "a: 1\nb: 2\n")]
    #[case::first_line("\na: 1\n", &["2:1-2:1 blank line"], "a: 1\n")]
    #[case::block_text("a: |\n  x\n\n  y\nb: 2\n", &[], "a: |\n  x\n\n  y\nb: 2\n")]
    #[case::keep_chomping("a: >+\n  x\n\nb: 1\n", &[], "a: >+\n  x\n\nb: 1\n")]
    #[case::multi_line_quoted("a: \"x\n\n  y\"\n", &[], "a: \"x\n\n  y\"\n")]
    fn front_matter(#[case] fm: &str, #[case] want: &[&str], #[case] fixed: &str) {
        assert_eq!(lint_rule(RULE, fm, Config::default()), want);
        assert_eq!(fix_fm(fm, Config::default()), fixed);
    }

    #[rstest]
    #[case::crlf(
        "---\r\na: 1\r\n\r\nb: 2\r\n---\r\n",
        &["3:1-3:1 blank line"],
        "---\r\na: 1\r\nb: 2\r\n---\r\n"
    )]
    fn whole_source(#[case] source: &str, #[case] want: &[&str], #[case] fixed: &str) {
        assert_eq!(lint_src(RULE, source, Config::default()), want);
        assert_eq!(fix_src(source, Config::default()), fixed);
    }
}
