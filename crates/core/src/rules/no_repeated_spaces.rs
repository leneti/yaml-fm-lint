//! `no-repeated-spaces`: a plain value must not contain runs of two or more spaces.
//!
//! # Why
//!
//! Repeated spaces inside a plain value are usually a typo or leftover alignment, and
//! they are easy to miss because they look like one space in most fonts.
//!
//! # Examples
//!
//! Bad:
//!
//! ```yaml
//! title: Hello  world
//! ```
//!
//! Good:
//!
//! ```yaml
//! title: Hello world
//! ```
//!
//! # Fix
//!
//! A suggestion, never applied by `fix`: it collapses every run in the value to one
//! space, which changes the value's text. Keys get no fix, since renaming a key changes
//! the data's shape.

use crate::document::Role;
use crate::lint::{Context, Finding};
use crate::tree::{NodeKind, Style, walk};
use crate::{Applicability, DataEffect, Fix, TextEdit, TextRange};

/// The message for every finding.
const MESSAGE: &str = "repeated spaces in a value";

/// Flags each run of two or more spaces in single-line plain scalars.
pub(crate) fn check(ctx: &Context<'_>) -> Vec<Finding> {
    let mut findings = Vec::new();
    // `nodes()` and `walk` both go in pre-order, keys before values.
    let mut refs = ctx.doc.nodes().into_iter();
    walk(&ctx.doc.root, &mut |node, path| {
        let Some(r) = refs.next() else {
            return;
        };
        if !std::ptr::eq(r.node, node) || r.in_flow {
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
        if text.contains('\n') {
            return;
        }
        let runs = runs(text, node.range.start);
        if runs.is_empty() {
            return;
        }
        // Every finding of a scalar carries the whole-scalar fix, so its edits always
        // produce `new_plain`.
        let fix = (r.role != Role::Key).then(|| Fix {
            applicability: Applicability::Suggestion,
            data_effect: DataEffect::ChangesScalar {
                path: path.to_vec(),
                new_plain: collapse(text),
            },
            edits: runs
                .iter()
                .map(|&range| TextEdit {
                    range,
                    new_text: " ".to_string(),
                })
                .collect(),
        });
        for range in runs {
            findings.push(Finding {
                range,
                message: MESSAGE.to_string(),
                fix: fix.clone(),
            });
        }
    });
    findings
}

/// The byte ranges (offset by `base`) of every run of two or more spaces in `text`.
fn runs(text: &str, base: usize) -> Vec<TextRange> {
    let mut runs = Vec::new();
    let mut start = None;
    for (i, c) in text.char_indices() {
        match (c == ' ', start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                if i - s >= 2 {
                    runs.push(TextRange {
                        start: base + s,
                        end: base + i,
                    });
                }
                start = None;
            }
            _ => {}
        }
    }
    // Plain scalars never end in a space, so a run cannot be open here.
    runs
}

/// `text` with every run of spaces replaced by one space.
fn collapse(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut prev_space = false;
    for c in text.chars() {
        if c != ' ' || !prev_space {
            out.push(c);
        }
        prev_space = c == ' ';
    }
    out
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::edit::apply_edits;
    use crate::testing::{fix_fm, lint_rule, wrap};
    use crate::{Applicability, Config, DataEffect, Level, Linter, RuleId};

    const RULE: RuleId = RuleId::NoRepeatedSpaces;

    fn ignoring_a() -> Config {
        Config {
            ignore_keys: vec!["a".to_string()],
            ..Config::default()
        }
    }

    #[rstest]
    #[case::run("a: x  y\n", Config::default(), &["2:5-2:7 repeated spaces in a value"])]
    #[case::two_runs(
        "a: x   y  z\n",
        Config::default(),
        &["2:5-2:8 repeated spaces in a value", "2:9-2:11 repeated spaces in a value"]
    )]
    #[case::item("l:\n  - x  y\n", Config::default(), &["3:6-3:8 repeated spaces in a value"])]
    #[case::key("a  b: 1\n", Config::default(), &["2:2-2:4 repeated spaces in a value"])]
    #[case::after_emoji("a: 😀  y\n", Config::default(), &["2:5-2:7 repeated spaces in a value"])]
    #[case::quoted("a: \"x  y\"\n", Config::default(), &[])]
    #[case::flow("a: [x  y]\n", Config::default(), &[])]
    #[case::single("a: x y\n", Config::default(), &[])]
    #[case::ignore_keys("a: x  y\n", ignoring_a(), &[])]
    fn front_matter(#[case] fm: &str, #[case] config: Config, #[case] want: &[&str]) {
        assert_eq!(lint_rule(RULE, fm, config.clone()), want);
        // A suggestion is never applied; other rules' fixes are switched off for the rows
        // they would rewrite.
        let mut config = config;
        config.rules.insert(RuleId::NoQuotes, Level::Off);
        config.rules.insert(RuleId::NoFlowSequence, Level::Off);
        assert_eq!(fix_fm(fm, config), fm);
    }

    fn fixes(fm: &str) -> Vec<Option<crate::Fix>> {
        let source = wrap(fm);
        Linter::new(Config::default(), None)
            .expect("valid config")
            .lint(&source)
            .diagnostics
            .into_iter()
            .filter(|d| d.rule == RULE)
            .map(|d| d.fix)
            .collect()
    }

    #[test]
    fn value_fix_is_a_suggestion_collapsing_every_run() {
        let fm = "a: x   y  z\n";
        let source = wrap(fm);
        let fixes = fixes(fm);
        assert_eq!(fixes.len(), 2);
        for fix in fixes {
            let fix = fix.expect("a fix");
            assert_eq!(fix.applicability, Applicability::Suggestion);
            let DataEffect::ChangesScalar { new_plain, .. } = &fix.data_effect else {
                panic!("expected ChangesScalar");
            };
            assert_eq!(new_plain, "x y z");
            assert_eq!(
                apply_edits(&source, &fix.edits).as_deref(),
                Some("---\na: x y z\n---\nBody.\n")
            );
        }
    }

    #[test]
    fn key_has_no_fix() {
        assert_eq!(fixes("a  b: 1\n"), [None]);
    }
}
