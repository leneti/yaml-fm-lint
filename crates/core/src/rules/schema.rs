//! `schema`: the front matter must match the JSON Schema.
//!
//! # Why
//!
//! A schema states what the front matter may contain: required keys, value types, patterns,
//! formats. Checking it here catches a wrong value at the line that has it.
//!
//! # Examples
//!
//! Bad, with a schema that requires `tags` items to match `^[a-z0-9-]+$`:
//!
//! ```yaml
//! tags:
//!   - ok
//!   - Bad
//! ```
//!
//! Good:
//!
//! ```yaml
//! tags:
//!   - ok
//!   - fine-too
//! ```
//!
//! # Fix
//!
//! No fix.

use std::collections::HashMap;

use jsonschema::error::ValidationErrorKind;

use crate::data::{Data, key_text, to_json};
use crate::lint::{Context, Finding};
use crate::tree::{Node, NodeKind};

/// For each mapping a path went through: its keys' JSON text to the index of the last entry
/// with that key. Built once per mapping, so placing N errors costs O(N + keys), not O(N * keys).
type KeyIndex = HashMap<*const Node, HashMap<String, usize>>;

/// One finding per schema error; none when no schema is configured.
pub(crate) fn check(ctx: &Context<'_>) -> Vec<Finding> {
    let Some(schema) = ctx.schema else {
        return Vec::new();
    };
    let instance = to_json(&ctx.doc.data);
    let mut index = KeyIndex::new();
    schema
        .iter_errors(&instance)
        .map(|error| {
            // The pointer text, not `Location`'s iterator: that drops a trailing empty segment,
            // so a key `""` would read as the root.
            let pointer = error.instance_path().to_string();
            let segments = segments(&pointer);
            let (node, complete) = resolve(&mut index, &ctx.doc.root, &ctx.doc.data, &segments);
            let range = match (error.kind(), &node.kind) {
                _ if pointer.is_empty()
                    && matches!(error.kind(), ValidationErrorKind::Required { .. }) =>
                {
                    ctx.fm.open
                }
                (ValidationErrorKind::Required { .. }, NodeKind::Mapping { entries, .. })
                    if complete =>
                {
                    entries.first().map_or(node.range, |entry| entry.key.range)
                }
                _ => node.range,
            };
            let path = if pointer.is_empty() {
                "(root)"
            } else {
                &pointer
            };
            Finding {
                range,
                message: format!("{path}: {error}"),
                fix: None,
            }
        })
        .collect()
}

/// The unescaped segments of a JSON pointer (`""` has none, `"/"` has one empty one).
fn segments(pointer: &str) -> Vec<String> {
    pointer
        .split('/')
        .skip(1)
        .map(|part| part.replace("~1", "/").replace("~0", "~"))
        .collect()
}

/// The node at the longest prefix of `segments` that exists in the tree, and whether that is
/// all of `segments`. Mapping segments match the key's JSON text (the last duplicate wins, as
/// in the JSON value); an alias ends the walk, since its target is not below it in the tree.
fn resolve<'a>(
    index: &mut KeyIndex,
    root: &'a Node,
    data: &'a Data,
    segments: &[String],
) -> (&'a Node, bool) {
    let (mut node, mut data) = (root, data);
    for segment in segments {
        let next = match (&node.kind, data) {
            (NodeKind::Mapping { entries, .. }, Data::Map(pairs)) => index
                .entry(node)
                .or_insert_with(|| {
                    let keys = pairs.iter().enumerate();
                    keys.map(|(i, (key, _))| (key_text(key), i)).collect()
                })
                .get(segment)
                .and_then(|&i| Some((&entries.get(i)?.value, &pairs.get(i)?.1))),
            (NodeKind::Sequence { items, .. }, Data::Seq(values)) => {
                let i = segment.parse::<usize>().ok();
                i.and_then(|i| items.get(i).zip(values.get(i)))
            }
            _ => None,
        };
        let Some((next_node, next_data)) = next else {
            return (node, false);
        };
        (node, data) = (next_node, next_data);
    }
    (node, true)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::testing::{show, wrap};
    use crate::{Config, ConfigErrorKind, Level, Linter, RuleId};

    /// The `schema` findings for front matter `fm` as `L:C-L:C rule message`.
    fn lint(schema: &str, fm: &str, config: Config) -> Vec<String> {
        let source = wrap(fm);
        Linter::new(config, Some(schema))
            .expect("valid schema")
            .lint(&source)
            .diagnostics
            .iter()
            .filter(|d| d.rule == RuleId::Schema)
            .map(|d| format!("{} {} {}", show(&source, d.range), d.rule, d.message))
            .collect()
    }

    #[rstest]
    #[case::pattern_error_at_item(
        r#"{"properties":{"tags":{"items":{"pattern":"^[a-z0-9-]+$"}}}}"#,
        "tags:\n  - ok\n  - Bad\n",
        &[r#"4:5-4:8 schema /tags/1: "Bad" does not match "^[a-z0-9-]+$""#]
    )]
    #[case::required_top_level(
        r#"{"required":["title"]}"#,
        "a: 1\n",
        &[r#"1:1-1:4 schema (root): "title" is a required property"#]
    )]
    #[case::required_nested(
        r#"{"properties":{"m":{"required":["x"]}}}"#,
        "m:\n  y: 1\n",
        &[r#"3:3-3:4 schema /m: "x" is a required property"#]
    )]
    #[case::alias_path(
        r#"{"properties":{"b":{"items":{"type":"string"}}}}"#,
        "a: &x [1]\nb: *x\n",
        &[r#"3:4-3:6 schema /b/0: 1 is not of type "string""#]
    )]
    #[case::inf_is_string(
        r#"{"properties":{"n":{"type":"number"}}}"#,
        "n: .inf\n",
        &[r#"2:4-2:8 schema /n: ".inf" is not of type "number""#]
    )]
    #[case::int_key_text(r#"{"required":["1"]}"#, "1: a\n", &[])]
    #[case::format_date(
        r#"{"properties":{"d":{"format":"date"}}}"#,
        "d: 2026-13-45\n",
        &[r#"2:4-2:14 schema /d: "2026-13-45" is not a "date""#]
    )]
    fn cases(#[case] schema: &str, #[case] fm: &str, #[case] want: &[&str]) {
        assert_eq!(lint(schema, fm, Config::default()), want);
    }

    #[rstest]
    #[case::external_ref(r#"{"$ref":"other.json#/x"}"#)]
    #[case::lookahead(r#"{"properties":{"a":{"pattern":"(?=x)x"}}}"#)]
    #[case::not_json("{")]
    fn schema_errors(#[case] schema: &str) {
        let error = Linter::new(Config::default(), Some(schema)).unwrap_err();
        assert_eq!(error.kind, ConfigErrorKind::Schema);
    }

    #[rstest]
    #[case::relative(r#"{"$ref":"other.json#/x"}"#, "other.json#/x")]
    #[case::nested(r#"{"properties":{"a":{"$ref":"other.json"}}}"#, "other.json")]
    #[case::http(
        r#"{"$ref":"https://example.com/s.json"}"#,
        "https://example.com/s.json"
    )]
    fn external_ref_error_message(#[case] schema: &str, #[case] uri: &str) {
        let error = Linter::new(Config::default(), Some(schema)).unwrap_err();
        assert_eq!(error.kind, ConfigErrorKind::Schema);
        assert_eq!(
            error.message,
            format!("schema `$ref` to another document is not allowed: {uri}")
        );
    }

    #[test]
    fn schema_error_details() {
        let config = Config {
            schema: Some("s.json".to_string()),
            ..Config::default()
        };
        let error = Linter::new(config, Some("{\n  \"a\" 1}")).unwrap_err();
        assert_eq!(error.file.as_deref(), Some("s.json"));
        assert_eq!((error.line, error.column), (Some(2), Some(7)));
        assert!(error.message.starts_with("schema is not valid JSON: "));
        let error = Linter::new(Config::default(), Some(r#"{"type":5}"#)).unwrap_err();
        assert!(error.message.starts_with("invalid schema: "), "{error}");
    }

    #[rstest]
    #[case::empty_key(
        r#"{"properties":{"":{"type":"string"}}}"#,
        "\"\": 5\n",
        &[r#"2:5-2:6 schema /: 5 is not of type "string""#]
    )]
    #[case::escaped_key(
        r#"{"properties":{"a/b":{"type":"string"}}}"#,
        "a/b: 5\n",
        &[r#"2:6-2:7 schema /a~1b: 5 is not of type "string""#]
    )]
    fn odd_keys(#[case] schema: &str, #[case] fm: &str, #[case] want: &[&str]) {
        assert_eq!(lint(schema, fm, Config::default()), want);
    }

    /// Placement must not be quadratic: one error per key, thousands of keys.
    #[test]
    fn many_keys_are_placed_in_linear_time() {
        let source = wrap(
            &(0..5_000)
                .map(|i| format!("key{i}: {i}\n"))
                .collect::<String>(),
        );
        let schema = r#"{"additionalProperties":{"type":"string"}}"#;
        let linter = Linter::new(Config::default(), Some(schema)).expect("valid schema");
        let started = std::time::Instant::now();
        let report = linter.lint(&source);
        let found = report
            .diagnostics
            .iter()
            .filter(|d| d.rule == RuleId::Schema);
        assert_eq!(found.count(), 5_000);
        assert!(started.elapsed() < std::time::Duration::from_secs(30));
    }

    #[test]
    fn root_error_is_at_the_root_node() {
        let schema = r#"{"minProperties":1}"#;
        assert_eq!(
            lint(schema, "{}\n", Config::default()),
            ["2:1-2:3 schema (root): {} has less than 1 property"]
        );
    }

    #[test]
    fn schema_off() {
        let mut config = Config::default();
        config.rules.insert(RuleId::Schema, Level::Off);
        assert!(lint(r#"{"required":["title"]}"#, "a: 1\n", config).is_empty());
    }
}
