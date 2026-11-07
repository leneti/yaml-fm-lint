//! `no-trailing-commas`: a plain value or list item must not end with a comma.
//!
//! # Why
//!
//! A comma at the end of a block-style value is usually a leftover from JSON or
//! from a flow list, and it becomes part of the text.
//!
//! # Examples
//!
//! Bad:
//!
//! ```yaml
//! title: Hello,
//! tags:
//!   - news,
//! ```
//!
//! Good:
//!
//! ```yaml
//! title: Hello
//! tags:
//!   - news
//! ```
//!
//! # Fix
//!
//! Deletes the trailing commas (and the spaces among them). The value is then read
//! again as plain text, so `5,` becomes the number `5`. When nothing would be left,
//! there is no fix.

use crate::document::Role;
use crate::lint::{Context, Finding};
use crate::tree::{NodeKind, Style, walk};
use crate::{Applicability, DataEffect, Fix, TextEdit, TextRange};

/// The message for every finding.
const MESSAGE: &str = "trailing comma";

/// Flags a trailing run of commas and whitespace on single-line plain values and items.
pub(crate) fn check(ctx: &Context<'_>) -> Vec<Finding> {
    let mut findings = Vec::new();
    // `nodes()` and `walk` both go in pre-order, keys before values.
    let mut refs = ctx.doc.nodes().into_iter();
    walk(&ctx.doc.root, &mut |node, path| {
        let Some(r) = refs.next() else {
            return;
        };
        if !std::ptr::eq(r.node, node) || r.in_flow || !matches!(r.role, Role::Value | Role::Item) {
            return;
        }
        let NodeKind::Scalar {
            style: Style::Plain,
            ..
        } = &node.kind
        else {
            return;
        };
        let Some(text) = ctx.source.get(node.range.start..node.range.end) else {
            return;
        };
        if text.contains('\n') || !text.ends_with(',') {
            return;
        }
        let kept = text.trim_end_matches(|c: char| c == ',' || c.is_whitespace());
        let run = TextRange {
            start: node.range.start + kept.len(),
            end: node.range.end,
        };
        let fix = (!kept.is_empty()).then(|| Fix {
            applicability: Applicability::Automatic,
            data_effect: DataEffect::ChangesScalar {
                path: path.to_vec(),
                new_plain: kept.to_string(),
            },
            edits: vec![TextEdit {
                range: run,
                new_text: String::new(),
            }],
        });
        findings.push(Finding {
            range: run,
            message: MESSAGE.to_string(),
            fix,
        });
    });
    findings
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::testing::{fix_fm, fix_src, lint_rule, lint_src};
    use crate::{Config, RuleId};

    const RULE: RuleId = RuleId::NoTrailingCommas;

    #[rstest]
    #[case::value("a: b,\n", &["2:5-2:6 trailing comma"], Some("a: b\n"))]
    #[case::item("l:\n  - x,\n", &["3:6-3:7 trailing comma"], Some("l:\n  - x\n"))]
    #[case::several("a: b, ,\n", &["2:5-2:8 trailing comma"], Some("a: b\n"))]
    #[case::becomes_int("w: 5,\n", &["2:5-2:6 trailing comma"], Some("w: 5\n"))]
    #[case::flow_separator("a: [x,]\n", &[], None)]
    #[case::inner_comma("a: b, c\n", &[], None)]
    fn front_matter(#[case] fm: &str, #[case] want: &[&str], #[case] fixed: Option<&str>) {
        assert_eq!(lint_rule(RULE, fm, Config::default()), want);
        if let Some(fixed) = fixed {
            assert_eq!(fix_fm(fm, Config::default()), fixed);
        }
    }

    #[test]
    fn crlf() {
        let source = "---\r\na: b,\r\n---\r\n";
        assert_eq!(
            lint_src(RULE, source, Config::default()),
            ["2:5-2:6 trailing comma"]
        );
        assert_eq!(fix_src(source, Config::default()), "---\r\na: b\r\n---\r\n");
    }
}
