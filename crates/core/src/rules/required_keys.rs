//! `required-keys`: every key in `requiredKeys` must be a top-level key.
//!
//! # Why
//!
//! Tools that read the front matter often rely on a few keys, such as a title or
//! a date. A missing one is easier to catch here than downstream.
//!
//! # Examples
//!
//! Bad, with `requiredKeys: [title, date]`:
//!
//! ```yaml
//! title: Hello
//! ```
//!
//! Good:
//!
//! ```yaml
//! title: Hello
//! date: 2026-01-01
//! ```
//!
//! # Fix
//!
//! No fix.

use crate::data::Data;
use crate::lint::{Context, Finding};

/// One finding per configured key that the root mapping lacks, in configuration order.
pub(crate) fn check(ctx: &Context<'_>) -> Vec<Finding> {
    let present = |key: &str| match &ctx.doc.data {
        Data::Map(pairs) => pairs
            .iter()
            .any(|(k, _)| matches!(k, Data::Str(s) if s == key)),
        _ => false,
    };
    ctx.config
        .required_keys
        .iter()
        .filter(|key| !present(key))
        .map(|key| Finding {
            range: ctx.fm.open,
            message: format!("missing required key `{key}`"),
            fix: None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::testing::lint_rule;
    use crate::{Config, RuleId};

    #[rstest]
    #[case::one_missing("title: x\n", &["1:1-1:4 missing required key `date`"])]
    #[case::nested_does_not_count(
        "meta:\n  date: x\ntitle: y\n",
        &["1:1-1:4 missing required key `date`"]
    )]
    #[case::empty(
        "",
        &["1:1-1:4 missing required key `title`", "1:1-1:4 missing required key `date`"]
    )]
    #[case::quoted_key_counts("\"title\": x\ndate: y\n", &[])]
    fn cases(#[case] fm: &str, #[case] want: &[&str]) {
        let config = Config {
            required_keys: vec!["title".to_string(), "date".to_string()],
            ..Config::default()
        };
        assert_eq!(lint_rule(RuleId::RequiredKeys, fm, config), want);
    }
}
