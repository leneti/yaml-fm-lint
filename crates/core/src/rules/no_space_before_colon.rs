//! `no-space-before-colon`: no whitespace between a key and its `:`.
//!
//! # Why
//!
//! `key : value` is unusual style, and some tools treat the key as `key ` with
//! the space attached.
//!
//! # Examples
//!
//! Bad:
//!
//! ```yaml
//! title : Hello
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
//! Deletes the spaces and tabs before the `:`. Explicit keys (`? key`) are skipped.

use crate::lint::{Context, Finding};
use crate::tree::{NodeKind, walk};
use crate::{Applicability, DataEffect, Fix, TextEdit, TextRange};

/// The message for every finding.
const MESSAGE: &str = "whitespace before \":\"";

/// Flags the gap between a key's end and its `:`.
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
            if entry.explicit || colon <= entry.key.range.end {
                continue;
            }
            let gap = TextRange {
                start: entry.key.range.end,
                end: colon,
            };
            // Only spaces and tabs; anything else (a comment, a line break) is not ours.
            let blank = ctx
                .source
                .get(gap.start..gap.end)
                .is_some_and(|t| t.chars().all(|c| c == ' ' || c == '\t'));
            if !blank {
                continue;
            }
            findings.push(Finding {
                range: gap,
                message: MESSAGE.to_string(),
                fix: Some(Fix {
                    applicability: Applicability::Automatic,
                    data_effect: DataEffect::Preserving,
                    edits: vec![TextEdit {
                        range: gap,
                        new_text: String::new(),
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

    const RULE: RuleId = RuleId::NoSpaceBeforeColon;

    #[rstest]
    #[case::one_space("a : 1\n", &["2:2-2:3 whitespace before \":\""], "a: 1\n")]
    #[case::two("a  : 1\n", &["2:2-2:4 whitespace before \":\""], "a: 1\n")]
    #[case::flow("m: {a : 1}\n", &["2:6-2:7 whitespace before \":\""], "m: {a: 1}\n")]
    #[case::quoted_key("\"a\" : 1\n", &["2:4-2:5 whitespace before \":\""], "\"a\": 1\n")]
    #[case::explicit_key("? a\n: 1\n", &[], "? a\n: 1\n")]
    fn front_matter(#[case] fm: &str, #[case] want: &[&str], #[case] fixed: &str) {
        assert_eq!(lint_rule(RULE, fm, Config::default()), want);
        assert_eq!(fix_fm(fm, Config::default()), fixed);
    }
}
