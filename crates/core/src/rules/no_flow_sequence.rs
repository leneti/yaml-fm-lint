//! `no-flow-sequence`: a list must be written in block style, one `- item` per line.
//!
//! # Why
//!
//! Block lists are easier to read and to diff than `[a, b]`: one item per line, one line
//! per change.
//!
//! # Examples
//!
//! Bad:
//!
//! ```yaml
//! tags: [news, update]
//! ```
//!
//! Good:
//!
//! ```yaml
//! tags:
//!   - news
//!   - update
//! ```
//!
//! # Fix
//!
//! Rewrites the list as a block list, indented by the `indentation` size. The fix is
//! only offered when every item is a one-line scalar, the list is the value of a block
//! mapping key or an item of a block list, it has no anchor, tag or comment on it, and it
//! is not inside another `[...]` or `{...}`.

use super::flow_fix::flow_to_block;
use crate::lint::{Context, Finding};
use crate::tree::NodeKind;

/// The message for every finding.
const MESSAGE: &str = "flow sequence; use a block sequence";

/// Flags every non-empty flow sequence.
pub(crate) fn check(ctx: &Context<'_>) -> Vec<Finding> {
    ctx.doc
        .nodes()
        .into_iter()
        .filter(|r| matches!(&r.node.kind, NodeKind::Sequence { flow: true, items } if !items.is_empty()))
        .map(|r| Finding {
            range: r.node.range,
            message: MESSAGE.to_string(),
            fix: flow_to_block(ctx, &r),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::testing::{fix_fm, fix_src, lint_rule, lint_src, wrap};
    use crate::{Config, Linter, RuleId};

    const RULE: RuleId = RuleId::NoFlowSequence;
    const MSG: &str = "flow sequence; use a block sequence";

    fn with_message(ranges: &[&str]) -> Vec<String> {
        ranges.iter().map(|r| format!("{r} {MSG}")).collect()
    }

    #[rstest]
    #[case::value("tags: [a, b]\n", &["2:7-2:13"], "tags:\n  - a\n  - b\n")]
    // `"yes"` keeps its quotes, so `no-quotes` leaves the fixed file alone.
    #[case::item("l:\n  - [a, \"yes\"]\n", &["3:5-3:15"], "l:\n  - - a\n    - \"yes\"\n")]
    #[case::multi_line("t: [\n  a,\n  b\n]\n", &["2:4-5:2"], "t:\n  - a\n  - b\n")]
    #[case::trailing_comma("t: [a, b,]\n", &["2:4-2:11"], "t:\n  - a\n  - b\n")]
    #[case::inner_spaces("t: [ a , b ]\n", &["2:4-2:13"], "t:\n  - a\n  - b\n")]
    #[case::non_ascii_key("é: [a]\n", &["2:4-2:7"], "é:\n  - a\n")]
    #[case::nested_key("a:\n  b: [x]\n", &["3:6-3:9"], "a:\n  b:\n    - x\n")]
    #[case::compact_key("l:\n  - k: [x]\n", &["3:8-3:11"], "l:\n  - k:\n      - x\n")]
    #[case::wide_dash("l:\n  -   [a, b]\n", &["3:7-3:13"], "l:\n  -   - a\n      - b\n")]
    #[case::value_on_next_line("a:\n  [x, y]\n", &["3:3-3:9"], "a:\n  - x\n  - y\n")]
    fn flagged_and_fixed(#[case] fm: &str, #[case] want: &[&str], #[case] fixed: &str) {
        assert_eq!(lint_rule(RULE, fm, Config::default()), with_message(want));
        assert_eq!(fix_fm(fm, Config::default()), fixed);
    }

    #[rstest]
    #[case::nested("a: [x, [y]]\n", &["2:4-2:12", "2:8-2:11"])]
    #[case::mapping_item("a: [x, {b: 1}]\n", &["2:4-2:15"])]
    #[case::pair_item("a: [x, b: 1]\n", &["2:4-2:13"])]
    #[case::alias_item("b: &s x\na: [*s]\n", &["3:4-3:8"])]
    #[case::comment_inside("a: [x, # c\n  y]\n", &["2:4-3:5"])]
    #[case::trailing_comment("a: [x] # c\n", &["2:4-2:7"])]
    #[case::comment_before_value("a: # c\n  [x]\n", &["3:3-3:6"])]
    #[case::anchor("a: &s [x]\n", &["2:7-2:10"])]
    #[case::tag("a: !!seq [x]\n", &["2:10-2:13"])]
    #[case::item_tag("k: [!foo a, b]\n", &["2:4-2:15"])]
    #[case::item_anchor("k: [&x a, b]\n", &["2:4-2:13"])]
    #[case::multi_line_item("a: [\"x\n  y\", z]\n", &["2:4-3:9"])]
    #[case::in_flow_mapping("a: {b: [x]}\n", &["2:8-2:11"])]
    fn flagged_without_fix(#[case] fm: &str, #[case] want: &[&str]) {
        assert_eq!(lint_rule(RULE, fm, Config::default()), with_message(want));
        let report = Linter::new(Config::default(), None)
            .expect("valid config")
            .lint(&wrap(fm));
        let ours = report.diagnostics.iter().filter(|d| d.rule == RULE);
        assert!(ours.clone().count() > 0 && ours.into_iter().all(|d| d.fix.is_none()));
    }

    #[rstest]
    #[case::empty("t: []\n")]
    #[case::empty_spaces("t: [ ]\n")]
    #[case::block("t:\n  - a\n")]
    fn not_flagged(#[case] fm: &str) {
        assert_eq!(lint_rule(RULE, fm, Config::default()), Vec::<String>::new());
    }

    #[test]
    fn indent_size_four() {
        let config = Config {
            indent_size: 4,
            ..Config::default()
        };
        assert_eq!(fix_fm("t: [a, b]\n", config), "t:\n    - a\n    - b\n");
    }

    #[test]
    fn crlf() {
        let source = "---\r\ntags: [a, b]\r\n---\r\n";
        assert_eq!(
            lint_src(RULE, source, Config::default()),
            with_message(&["2:7-2:13"])
        );
        assert_eq!(
            fix_src(source, Config::default()),
            "---\r\ntags:\r\n  - a\r\n  - b\r\n---\r\n"
        );
    }

    #[test]
    fn crlf_item_and_multi_line_value() {
        let source = "---\r\nk:\r\n  - [a, b]\r\nz: [\r\n  p,\r\n  q\r\n]\r\n---\r\n";
        assert_eq!(
            lint_src(RULE, source, Config::default()),
            with_message(&["3:5-3:11", "4:4-7:2"])
        );
        assert_eq!(
            fix_src(source, Config::default()),
            "---\r\nk:\r\n  - - a\r\n    - b\r\nz:\r\n  - p\r\n  - q\r\n---\r\n"
        );
    }
}
