//! `no-flow-mapping`: a mapping must be written in block style, one `key: value` per line.
//!
//! # Why
//!
//! Block mappings are easier to read and to diff than `{a: 1, b: 2}`: one entry per line,
//! one line per change.
//!
//! # Examples
//!
//! Bad:
//!
//! ```yaml
//! meta: {author: Jo, draft: true}
//! ```
//!
//! Good:
//!
//! ```yaml
//! meta:
//!   author: Jo
//!   draft: true
//! ```
//!
//! # Fix
//!
//! Rewrites the mapping as a block mapping, indented by the `indentation` size. The fix is
//! only offered when every key and value is a one-line scalar, the mapping is the value of a
//! block mapping key or an item of a block list, it has no anchor, tag or comment on it, and
//! it is not inside another `[...]` or `{...}`. A single `key: value` pair inside `[...]`
//! has no braces and is not flagged.

use super::flow_fix::flow_to_block;
use crate::lint::{Context, Finding};
use crate::tree::NodeKind;

/// The message for every finding.
const MESSAGE: &str = "flow mapping; use a block mapping";

/// Flags every non-empty `{...}` mapping.
pub(crate) fn check(ctx: &Context<'_>) -> Vec<Finding> {
    ctx.doc
        .nodes()
        .into_iter()
        .filter(|r| {
            matches!(&r.node.kind, NodeKind::Mapping { flow: true, entries } if !entries.is_empty())
                // A lone pair inside `[...]` is a flow mapping without braces.
                && ctx.source.get(r.node.range.start..).is_some_and(|s| s.starts_with('{'))
        })
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
    use crate::{Config, Level, Linter, RuleId};

    const RULE: RuleId = RuleId::NoFlowMapping;
    const MSG: &str = "flow mapping; use a block mapping";

    fn with_message(ranges: &[&str]) -> Vec<String> {
        ranges.iter().map(|r| format!("{r} {MSG}")).collect()
    }

    #[rstest]
    #[case::value("meta: {a: 1, b: 2}\n", &["2:7-2:19"], "meta:\n  a: 1\n  b: 2\n")]
    #[case::item("l:\n  - {a: 1, b: 2}\n", &["3:5-3:17"], "l:\n  - a: 1\n    b: 2\n")]
    #[case::multi_line("m: {\n  a: 1,\n  b: 2\n}\n", &["2:4-5:2"], "m:\n  a: 1\n  b: 2\n")]
    #[case::trailing_comma("m: {a: 1, b: 2,}\n", &["2:4-2:17"], "m:\n  a: 1\n  b: 2\n")]
    #[case::inner_spaces("m: { a: 1 , b: 2 }\n", &["2:4-2:19"], "m:\n  a: 1\n  b: 2\n")]
    #[case::null_value("m: {a: 1, b: }\n", &["2:4-2:15"], "m:\n  a: 1\n  b:\n")]
    // Quoted keys and values keep their quotes (these ones `no-quotes` leaves alone).
    #[case::quoted("m: {\"yes\": \"no\", 'on': 'off'}\n", &["2:4-2:30"], "m:\n  \"yes\": \"no\"\n  'on': 'off'\n")]
    #[case::non_ascii_key("é: {ü: ö}\n", &["2:4-2:10"], "é:\n  ü: ö\n")]
    #[case::nested_key("a:\n  b: {x: 1}\n", &["3:6-3:12"], "a:\n  b:\n    x: 1\n")]
    #[case::compact_key("l:\n  - k: {x: 1}\n", &["3:8-3:14"], "l:\n  - k:\n      x: 1\n")]
    #[case::wide_dash("l:\n  -   {a: 1, b: 2}\n", &["3:7-3:19"], "l:\n  -   a: 1\n      b: 2\n")]
    #[case::value_on_next_line("a:\n  {x: 1, y: 2}\n", &["3:3-3:15"], "a:\n  x: 1\n  y: 2\n")]
    fn flagged_and_fixed(#[case] fm: &str, #[case] want: &[&str], #[case] fixed: &str) {
        assert_eq!(lint_rule(RULE, fm, Config::default()), with_message(want));
        assert_eq!(fix_fm(fm, Config::default()), fixed);
    }

    #[rstest]
    #[case::nested_sequence("m: {a: [1]}\n", &["2:4-2:12"])]
    #[case::nested_mapping("m: {a: {b: 1}}\n", &["2:4-2:15", "2:8-2:14"])]
    #[case::key_without_colon("m: {a, b}\n", &["2:4-2:10"])]
    #[case::explicit_key("m: {? a : 1}\n", &["2:4-2:13"])]
    #[case::alias_value("b: &s x\nm: {a: *s}\n", &["3:4-3:11"])]
    #[case::comment_inside("m: {a: 1, # c\n  b: 2}\n", &["2:4-3:8"])]
    #[case::trailing_comment("m: {a: 1} # c\n", &["2:4-2:10"])]
    #[case::comment_before_value("m: # c\n  {a: 1}\n", &["3:3-3:9"])]
    #[case::anchor("m: &s {a: 1}\n", &["2:7-2:13"])]
    #[case::tag("m: !!map {a: 1}\n", &["2:10-2:16"])]
    #[case::value_tag("m: {a: !foo 1, b: 2}\n", &["2:4-2:21"])]
    #[case::key_anchor("m: {&x a: 1, b: 2}\n", &["2:4-2:19"])]
    #[case::multi_line_value("m: {a: \"x\n  y\", b: 2}\n", &["2:4-3:12"])]
    #[case::in_flow_sequence("a: [{b: 1}]\n", &["2:5-2:11"])]
    #[case::in_flow_mapping("a: {b: {c: 1}}\n", &["2:4-2:15", "2:8-2:14"])]
    fn flagged_without_fix(#[case] fm: &str, #[case] want: &[&str]) {
        let mut config = Config::default();
        // `no-flow-sequence` is not under test here.
        config.rules.insert(RuleId::NoFlowSequence, Level::Off);
        assert_eq!(lint_rule(RULE, fm, config.clone()), with_message(want));
        let report = Linter::new(config, None)
            .expect("valid config")
            .lint(&wrap(fm));
        let ours = report.diagnostics.iter().filter(|d| d.rule == RULE);
        assert!(ours.clone().count() > 0 && ours.into_iter().all(|d| d.fix.is_none()));
    }

    #[rstest]
    #[case::empty("m: {}\n")]
    #[case::empty_spaces("m: { }\n")]
    #[case::block("m:\n  a: 1\n")]
    // A lone pair in `[...]` has no braces.
    #[case::pair_in_sequence("m: [a: 1]\n")]
    fn not_flagged(#[case] fm: &str) {
        assert_eq!(lint_rule(RULE, fm, Config::default()), Vec::<String>::new());
    }

    #[test]
    fn indent_size_four() {
        let config = Config {
            indent_size: 4,
            ..Config::default()
        };
        assert_eq!(
            lint_rule(RULE, "m: {a: 1}\n", config.clone()),
            with_message(&["2:4-2:10"])
        );
        assert_eq!(fix_fm("m: {a: 1}\n", config), "m:\n    a: 1\n");
    }

    #[test]
    fn crlf() {
        let source = "---\r\nmeta: {a: 1, b: 2}\r\n---\r\n";
        assert_eq!(
            lint_src(RULE, source, Config::default()),
            with_message(&["2:7-2:19"])
        );
        assert_eq!(
            fix_src(source, Config::default()),
            "---\r\nmeta:\r\n  a: 1\r\n  b: 2\r\n---\r\n"
        );
    }

    #[test]
    fn crlf_item_and_multi_line_value() {
        let source = "---\r\nk:\r\n  - {a: 1, b: 2}\r\nz: {\r\n  p: 1,\r\n  q: 2\r\n}\r\n---\r\n";
        assert_eq!(
            lint_src(RULE, source, Config::default()),
            with_message(&["3:5-3:17", "4:4-7:2"])
        );
        assert_eq!(
            fix_src(source, Config::default()),
            "---\r\nk:\r\n  - a: 1\r\n    b: 2\r\nz:\r\n  p: 1\r\n  q: 2\r\n---\r\n"
        );
    }
}
