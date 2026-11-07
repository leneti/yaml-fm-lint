//! `no-trailing-spaces`: lines must not end with spaces or tabs.
//!
//! # Why
//!
//! Trailing whitespace is invisible, shows up as noise in diffs and is easy to
//! add by accident.
//!
//! # Examples
//!
//! Bad (the first line ends with a space):
//!
//! ```yaml
//! title: Hello
//! ```
//!
//! Good:
//!
//! ```yaml
//! title: Hello
//! ```
//!
//! # Fix
//!
//! Deletes the trailing spaces and tabs. Text inside a scalar, such as block
//! scalar content or a multi-line quoted string, is left alone.

use crate::document::LineKind;
use crate::lint::{Context, Finding};
use crate::{Applicability, DataEffect, Fix, TextEdit, TextRange};

/// The message for every finding.
const MESSAGE: &str = "trailing whitespace";

/// Flags the trailing run of spaces and tabs on each line that is not block-scalar text.
pub(crate) fn check(ctx: &Context<'_>) -> Vec<Finding> {
    let mut findings = Vec::new();
    for line in &ctx.doc.lines {
        if line.kind == LineKind::BlockText {
            continue;
        }
        let Some(text) = ctx.source.get(line.range.start..line.range.end) else {
            continue;
        };
        let kept = text.trim_end_matches([' ', '\t']).len();
        let run = TextRange {
            start: line.range.start + kept,
            end: line.range.end,
        };
        // A run inside a multi-line quoted scalar is part of its text.
        if run.start == run.end || ctx.doc.in_scalar(run.start) {
            continue;
        }
        findings.push(Finding {
            range: run,
            message: MESSAGE.to_string(),
            fix: Some(Fix {
                applicability: Applicability::Automatic,
                data_effect: DataEffect::Preserving,
                edits: vec![TextEdit {
                    range: run,
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

    const RULE: RuleId = RuleId::NoTrailingSpaces;

    #[rstest]
    #[case::space("a: 1 \n", &["2:5-2:6 trailing whitespace"], "a: 1\n")]
    #[case::tab("a: 1\t\n", &["2:5-2:6 trailing whitespace"], "a: 1\n")]
    #[case::comment_line("# c  \n", &["2:4-2:6 trailing whitespace"], "# c\n")]
    #[case::after_emoji("emoji: 😀 \n", &["2:9-2:10 trailing whitespace"], "emoji: 😀\n")]
    #[case::block_text("a: |\n  x  \n", &[], "a: |\n  x  \n")]
    #[case::multi_line_quoted("a: \"x:  \n  y\"\n", &[], "a: \"x:  \n  y\"\n")]
    #[case::after_closing_quote("a: \"x:\n  y\" \n", &["3:5-3:6 trailing whitespace"], "a: \"x:\n  y\"\n")]
    fn front_matter(#[case] fm: &str, #[case] want: &[&str], #[case] fixed: &str) {
        assert_eq!(lint_rule(RULE, fm, Config::default()), want);
        assert_eq!(fix_fm(fm, Config::default()), fixed);
    }

    #[rstest]
    #[case::delimiters(
        "--- \na: 1\n--- \nBody. \n",
        &["1:4-1:5 trailing whitespace", "3:4-3:5 trailing whitespace"],
        "---\na: 1\n---\nBody. \n"
    )]
    #[case::crlf(
        "---\r\na: 1 \r\n---\r\n",
        &["2:5-2:6 trailing whitespace"],
        "---\r\na: 1\r\n---\r\n"
    )]
    fn whole_source(#[case] source: &str, #[case] want: &[&str], #[case] fixed: &str) {
        assert_eq!(lint_src(RULE, source, Config::default()), want);
        assert_eq!(fix_src(source, Config::default()), fixed);
    }
}
