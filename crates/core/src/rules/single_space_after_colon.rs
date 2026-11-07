//! `single-space-after-colon`: at most one space between a `:` and its value.
//!
//! # Why
//!
//! Extra spaces after the colon misalign values for no benefit and show up as
//! noise in diffs.
//!
//! # Examples
//!
//! Bad:
//!
//! ```yaml
//! title:  Hello
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
//! Replaces the run of spaces and tabs after the `:` with one space. A colon
//! followed by a line break, a comment or nothing is left alone.

use crate::lint::{Context, Finding};
use crate::tree::{NodeKind, walk};
use crate::{Applicability, DataEffect, Fix, TextEdit, TextRange};

/// The message for every finding.
const MESSAGE: &str = "more than one space after \":\"";

/// Flags a run of two or more spaces and tabs between a `:` and a value on the same line.
pub(crate) fn check(ctx: &Context<'_>) -> Vec<Finding> {
    let mut findings = Vec::new();
    walk(&ctx.doc.root, &mut |node, _| {
        let NodeKind::Mapping { entries, .. } = &node.kind else {
            return;
        };
        for entry in entries {
            let Some(colon) = entry.colon else {
                continue;
            };
            let value = entry.value.range;
            // A null value is empty; a value on a later line or after a comment has a line
            // break between the colon and its start.
            if value.start >= value.end {
                continue;
            }
            let Some(between) = ctx.source.get(colon + 1..value.start) else {
                continue;
            };
            let run_len = between.len() - between.trim_start_matches([' ', '\t']).len();
            let rest = between.get(run_len..).unwrap_or("");
            // An anchor or tag may sit between the run and the value, but not a line break.
            if run_len < 2 || rest.contains(['\n', '#']) {
                continue;
            }
            let run = TextRange {
                start: colon + 1,
                end: colon + 1 + run_len,
            };
            findings.push(Finding {
                range: run,
                message: MESSAGE.to_string(),
                fix: Some(Fix {
                    applicability: Applicability::Automatic,
                    data_effect: DataEffect::Preserving,
                    edits: vec![TextEdit {
                        range: run,
                        new_text: " ".to_string(),
                    }],
                }),
            });
        }
    });
    findings
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::testing::{fix_fm, lint_rule};
    use crate::{Config, RuleId};

    const RULE: RuleId = RuleId::SingleSpaceAfterColon;

    // A tab right after the colon is a yaml-syntax error in the parser (ruling T4b), so there
    // are no tab rows.
    #[rstest]
    #[case::two_spaces("a:  1\n", &["2:3-2:5 more than one space after \":\""], "a: 1\n")]
    #[case::comment_only("a:   # c\n  - x\n", &[], "a:   # c\n  - x\n")]
    #[case::flow("m: {a:  1}\n", &["2:7-2:9 more than one space after \":\""], "m: {a: 1}\n")]
    #[case::block_header("a:  |\n  x\n", &["2:3-2:5 more than one space after \":\""], "a: |\n  x\n")]
    #[case::single_space("a: 1\n", &[], "a: 1\n")]
    // Trailing spaces after a null value are no-trailing-spaces' business, so they go in the fix.
    #[case::null_value("a:   \nb: 1\n", &[], "a:\nb: 1\n")]
    fn front_matter(#[case] fm: &str, #[case] want: &[&str], #[case] fixed: &str) {
        assert_eq!(lint_rule(RULE, fm, Config::default()), want);
        assert_eq!(fix_fm(fm, Config::default()), fixed);
    }
}
