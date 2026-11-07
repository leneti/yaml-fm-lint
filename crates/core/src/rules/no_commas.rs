//! `no-commas`: a plain value must not look like a comma-separated list.
//!
//! # Why
//!
//! `tags: a, b` is one string `a, b`, not a list. Authors who write it usually
//! meant a sequence.
//!
//! # Examples
//!
//! Bad:
//!
//! ```yaml
//! tags: news, sport
//! ```
//!
//! Good:
//!
//! ```yaml
//! tags:
//!   - news
//!   - sport
//! ```
//!
//! # Fix
//!
//! No fix.

use crate::document::Role;
use crate::lint::{Context, Finding};
use crate::tree::{NodeKind, Style};

/// The message for every finding.
const MESSAGE: &str = "comma in a plain value; use a sequence for lists";

/// Flags single-line plain values and items with a comma that is followed by a character.
pub(crate) fn check(ctx: &Context<'_>) -> Vec<Finding> {
    let mut findings = Vec::new();
    for r in ctx.doc.nodes() {
        if r.in_flow || !matches!(r.role, Role::Value | Role::Item) {
            continue;
        }
        let NodeKind::Scalar {
            style: Style::Plain,
            ..
        } = &r.node.kind
        else {
            continue;
        };
        let Some(text) = ctx.source.get(r.node.range.start..r.node.range.end) else {
            continue;
        };
        if text.contains('\n')
            || !text
                .split_once(',')
                .is_some_and(|(_, rest)| !rest.is_empty())
        {
            continue;
        }
        findings.push(Finding {
            range: r.node.range,
            message: MESSAGE.to_string(),
            fix: None,
        });
    }
    findings
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::testing::{fix_fm, lint_rule};
    use crate::{Config, RuleId};

    const RULE: RuleId = RuleId::NoCommas;
    const MSG: &str = "comma in a plain value; use a sequence for lists";

    #[rstest]
    #[case::list_like("a: x, y\n", &["2:4-2:8"])]
    #[case::no_space("a: x,y\n", &["2:4-2:7"])]
    #[case::item("l:\n  - x, y\n", &["3:5-3:9"])]
    #[case::trailing_only("a: x,\n", &[])]
    #[case::quoted("a: \"x, y\"\n", &[])]
    #[case::flow("a: [x, y]\n", &[])]
    #[case::key("x, y: 1\n", &[])]
    fn front_matter(#[case] fm: &str, #[case] want: &[&str]) {
        let want: Vec<String> = want.iter().map(|r| format!("{r} {MSG}")).collect();
        assert_eq!(lint_rule(RULE, fm, Config::default()), want);
    }

    #[test]
    fn no_fix() {
        let fm = "a: x, y\nb: [x, y]\n";
        assert_eq!(fix_fm(fm, Config::default()), fm);
    }
}
