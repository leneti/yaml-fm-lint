//! The parsed front matter as plain values, with aliases expanded.

use std::collections::HashMap;
use std::sync::LazyLock;

use regex::Regex;

use crate::parse::{MAX_DEPTH, SyntaxIssue};
use crate::rules::{input_limit, yaml_syntax};
use crate::tree::{Node, NodeKind, Style};
use crate::{PathStep, RuleId, TextRange};

/// The most nodes aliases may add to the tree (the "billion laughs" guard).
pub(crate) const MAX_EXPANDED_NODES: usize = 100_000;

/// The most text (summed scalar lengths, keys included) aliases may add to the tree.
pub(crate) const MAX_ALIAS_BYTES: usize = 16 * 1024 * 1024;

/// A YAML value. Mappings keep source order and may have any value as a key.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Data {
    /// `null`, `~` or an empty value.
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// An integer that fits in `i64`.
    Int(i64),
    /// A float, including `.inf` and `.nan`; also integers too big for `i64`.
    Float(f64),
    /// A string.
    Str(String),
    /// A sequence.
    Seq(Vec<Data>),
    /// A mapping, as key/value pairs in source order.
    Map(Vec<(Data, Data)>),
}

/// Structural equality: floats compare by bits (so `0.0 != -0.0`) except that any NaN equals
/// any NaN, and mappings compare in source order.
pub(crate) fn data_eq(a: &Data, b: &Data) -> bool {
    match (a, b) {
        (Data::Null, Data::Null) => true,
        (Data::Bool(x), Data::Bool(y)) => x == y,
        (Data::Int(x), Data::Int(y)) => x == y,
        (Data::Float(x), Data::Float(y)) => {
            (x.is_nan() && y.is_nan()) || x.to_bits() == y.to_bits()
        }
        (Data::Str(x), Data::Str(y)) => x == y,
        (Data::Seq(x), Data::Seq(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(x, y)| data_eq(x, y))
        }
        (Data::Map(x), Data::Map(y)) => {
            x.len() == y.len()
                && x.iter()
                    .zip(y)
                    .all(|((xk, xv), (yk, yv))| data_eq(xk, yk) && data_eq(xv, yv))
        }
        _ => false,
    }
}

static NULL: LazyLock<Regex> = LazyLock::new(|| regex(r"^(null|Null|NULL|~|)$"));
static BOOL: LazyLock<Regex> = LazyLock::new(|| regex(r"^(true|True|TRUE|false|False|FALSE)$"));
static DECIMAL: LazyLock<Regex> = LazyLock::new(|| regex(r"^[-+]?[0-9]+$"));
static OCTAL: LazyLock<Regex> = LazyLock::new(|| regex(r"^0o[0-7]+$"));
static HEX: LazyLock<Regex> = LazyLock::new(|| regex(r"^0x[0-9a-fA-F]+$"));
static FLOAT: LazyLock<Regex> =
    LazyLock::new(|| regex(r"^[-+]?(\.[0-9]+|[0-9]+(\.[0-9]*)?)([eE][-+]?[0-9]+)?$"));
static INF: LazyLock<Regex> = LazyLock::new(|| regex(r"^[-+]?(\.inf|\.Inf|\.INF)$"));
static NAN: LazyLock<Regex> = LazyLock::new(|| regex(r"^(\.nan|\.NaN|\.NAN)$"));

fn regex(pattern: &str) -> Regex {
    // The patterns are constants, checked by the tests.
    #[allow(clippy::expect_used)]
    Regex::new(pattern).expect("valid core-schema regex")
}

/// A base 8 or 16 integer from its digits; a `Float` when it does not fit in `i64`.
fn integer(digits: &str, radix: u32) -> Data {
    i64::from_str_radix(digits, radix).map_or_else(
        |_| {
            Data::Float(digits.chars().fold(0.0, |sum, c| {
                sum * f64::from(radix) + f64::from(c.to_digit(radix).unwrap_or(0))
            }))
        },
        Data::Int,
    )
}

/// The value of a plain (unquoted, untagged) scalar by the YAML 1.2.2 core schema (§10.3.2).
pub(crate) fn resolve_plain(text: &str) -> Data {
    if NULL.is_match(text) {
        Data::Null
    } else if BOOL.is_match(text) {
        Data::Bool(text.starts_with(['t', 'T']))
    } else if DECIMAL.is_match(text) {
        // Rust's parser takes the sign; too big for `i64` is a float of the same digits.
        text.parse()
            .map_or_else(|_| Data::Float(text.parse().unwrap_or(f64::NAN)), Data::Int)
    } else if OCTAL.is_match(text) {
        integer(text.get(2..).unwrap_or(""), 8)
    } else if HEX.is_match(text) {
        integer(text.get(2..).unwrap_or(""), 16)
    } else if FLOAT.is_match(text) {
        Data::Float(text.parse().unwrap_or(f64::NAN))
    } else if INF.is_match(text) {
        Data::Float(if text.starts_with('-') {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        })
    } else if NAN.is_match(text) {
        Data::Float(f64::NAN)
    } else {
        Data::Str(text.to_string())
    }
}

/// A collection being built.
struct Frame<'a> {
    node: &'a Node,
    /// Children done so far; a mapping's keys and values alternate.
    done: Vec<Data>,
    /// The next child of `node` to visit.
    next: usize,
    /// `node` is the target of an alias.
    via_alias: bool,
}

struct Builder<'a> {
    anchors: HashMap<usize, &'a Node>,
    stack: Vec<Frame<'a>>,
    /// The outermost alias being expanded, while one is.
    alias: Option<TextRange>,
    /// Alias targets being expanded, nested.
    alias_depth: usize,
    /// Nodes added by alias expansion so far.
    expanded: usize,
    /// Bytes of scalar text added by alias expansion so far.
    expanded_bytes: usize,
    result: Option<Data>,
}

fn issue(rule: RuleId, range: TextRange, message: &str) -> SyntaxIssue {
    SyntaxIssue {
        rule,
        range,
        message: message.to_string(),
    }
}

impl<'a> Builder<'a> {
    /// Starts `node`: a scalar is finished at once, a collection gets a frame, an alias
    /// enters its target.
    fn enter(&mut self, node: &'a Node, via_alias: bool) -> Result<(), SyntaxIssue> {
        let at = self.alias.unwrap_or(node.range);
        if self.alias_depth > 0 {
            self.expanded += 1;
            if self.expanded > MAX_EXPANDED_NODES {
                return Err(issue(RuleId::InputLimit, at, input_limit::TOO_MANY_NODES));
            }
        }
        match &node.kind {
            NodeKind::Scalar { value, style } => {
                if self.alias_depth > 0 {
                    self.expanded_bytes += value.len();
                    if self.expanded_bytes > MAX_ALIAS_BYTES {
                        return Err(issue(RuleId::InputLimit, at, input_limit::TOO_MUCH_TEXT));
                    }
                }
                let plain = *style == Style::Plain && !node.tagged_str;
                let data = if plain {
                    resolve_plain(value)
                } else {
                    Data::Str(value.clone())
                };
                self.finish(data, node, via_alias);
            }
            NodeKind::Sequence { .. } | NodeKind::Mapping { .. } => {
                if self.stack.len() >= MAX_DEPTH {
                    return Err(issue(RuleId::InputLimit, at, input_limit::TOO_DEEP));
                }
                self.stack.push(Frame {
                    node,
                    done: Vec::new(),
                    next: 0,
                    via_alias,
                });
            }
            NodeKind::Alias { anchor } => {
                let Some(&target) = self.anchors.get(anchor) else {
                    return Err(issue(
                        RuleId::YamlSyntax,
                        node.range,
                        yaml_syntax::UNKNOWN_ALIAS,
                    ));
                };
                if self.alias_depth == 0 {
                    self.alias = Some(node.range);
                }
                self.alias_depth += 1;
                self.enter(target, true)?;
            }
        }
        Ok(())
    }

    /// Hands a finished node's value to its parent, and remembers its anchor.
    fn finish(&mut self, data: Data, node: &'a Node, via_alias: bool) {
        if let Some(id) = node.anchor {
            self.anchors.insert(id, node);
        }
        if via_alias {
            self.alias_depth = self.alias_depth.saturating_sub(1);
            if self.alias_depth == 0 {
                self.alias = None;
            }
        }
        match self.stack.last_mut() {
            Some(parent) => parent.done.push(data),
            None => self.result = Some(data),
        }
    }
}

/// Builds the values of the whole tree, expanding aliases.
///
/// An alias takes the latest anchor defined before it (anchors are numbered per definition,
/// so that is the one it names); an anchor is usable once its node is complete. Aliases may
/// add at most [`MAX_EXPANDED_NODES`] nodes and [`MAX_ALIAS_BYTES`] bytes of text in all, and nest at most [`MAX_DEPTH`] levels.
/// The walk uses an explicit stack, so depth never reaches the call stack.
pub(crate) fn build(root: &Node) -> Result<Data, SyntaxIssue> {
    let mut b = Builder {
        anchors: HashMap::new(),
        stack: Vec::new(),
        alias: None,
        alias_depth: 0,
        expanded: 0,
        expanded_bytes: 0,
        result: None,
    };
    b.enter(root, false)?;
    while let Some(frame) = b.stack.last_mut() {
        let i = frame.next;
        let child = match &frame.node.kind {
            NodeKind::Sequence { items, .. } => items.get(i),
            NodeKind::Mapping { entries, .. } => entries
                .get(i / 2)
                .map(|e| if i % 2 == 0 { &e.key } else { &e.value }),
            NodeKind::Scalar { .. } | NodeKind::Alias { .. } => None,
        };
        match child {
            Some(child) => {
                frame.next += 1;
                b.enter(child, false)?;
            }
            None => {
                let Some(frame) = b.stack.pop() else { break };
                let data = match frame.node.kind {
                    NodeKind::Mapping { .. } => {
                        let mut it = frame.done.into_iter();
                        let mut pairs = Vec::new();
                        while let (Some(key), Some(value)) = (it.next(), it.next()) {
                            pairs.push((key, value));
                        }
                        Data::Map(pairs)
                    }
                    _ => Data::Seq(frame.done),
                };
                b.finish(data, frame.node, frame.via_alias);
            }
        }
    }
    Ok(b.result.unwrap_or(Data::Null))
}

/// The value at `path` (as handed out by [`crate::tree::walk`]), if there is one.
pub(crate) fn lookup<'a>(data: &'a Data, path: &[PathStep]) -> Option<&'a Data> {
    path.iter().try_fold(data, |data, step| match (data, step) {
        (Data::Seq(items), PathStep::Item(i)) => items.get(*i),
        (Data::Map(pairs), PathStep::Key(i)) => pairs.get(*i).map(|(k, _)| k),
        (Data::Map(pairs), PathStep::Value(i)) => pairs.get(*i).map(|(_, v)| v),
        _ => None,
    })
}

/// Like [`lookup`], to replace the value.
pub(crate) fn lookup_mut<'a>(data: &'a mut Data, path: &[PathStep]) -> Option<&'a mut Data> {
    path.iter().try_fold(data, |data, step| match (data, step) {
        (Data::Seq(items), PathStep::Item(i)) => items.get_mut(*i),
        (Data::Map(pairs), PathStep::Key(i)) => pairs.get_mut(*i).map(|(k, _)| k),
        (Data::Map(pairs), PathStep::Value(i)) => pairs.get_mut(*i).map(|(_, v)| v),
        _ => None,
    })
}

/// A float as JSON: `.inf`, `-.inf` and `.nan` have no JSON number, so they become strings.
fn special_float(f: f64) -> Option<&'static str> {
    if f.is_nan() {
        Some(".nan")
    } else if f == f64::INFINITY {
        Some(".inf")
    } else if f == f64::NEG_INFINITY {
        Some("-.inf")
    } else {
        None
    }
}

/// The text of a mapping key in JSON, which only has string keys.
pub(crate) fn key_text(key: &Data) -> String {
    match key {
        Data::Str(text) => text.clone(),
        Data::Int(n) => n.to_string(),
        Data::Bool(b) => b.to_string(),
        Data::Null => "null".to_string(),
        Data::Float(f) => special_float(*f).map_or_else(|| f.to_string(), str::to_string),
        Data::Seq(_) | Data::Map(_) => to_json(key).to_string(),
    }
}

/// The JSON value of `data` (spec §5.7). Duplicate keys: the last one wins.
pub(crate) fn to_json(data: &Data) -> serde_json::Value {
    use serde_json::Value;
    match data {
        Data::Null => Value::Null,
        Data::Bool(b) => Value::Bool(*b),
        Data::Int(n) => Value::from(*n),
        Data::Float(f) => match special_float(*f) {
            Some(text) => Value::String(text.to_string()),
            None => Value::from(*f),
        },
        Data::Str(text) => Value::String(text.clone()),
        Data::Seq(items) => Value::Array(items.iter().map(to_json).collect()),
        Data::Map(pairs) => Value::Object(
            pairs
                .iter()
                .map(|(k, v)| (key_text(k), to_json(v)))
                .collect(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::extract::{Extraction, extract};
    use crate::parse::parse;
    use crate::testing::{lint_all, wrap};
    use crate::{Config, RuleId, TextRange};

    fn data_of(source: &str) -> Data {
        let Extraction::Found(fm) = extract(source) else {
            panic!("no front matter in {source:?}");
        };
        let root = parse(source, &fm).expect("parses");
        build(&root).expect("builds")
    }

    fn fm_data(fm: &str) -> Data {
        data_of(&wrap(fm))
    }

    fn s(text: &str) -> Data {
        Data::Str(text.to_string())
    }

    fn entry<'a>(data: &'a Data, key: &str) -> &'a Data {
        let Data::Map(pairs) = data else {
            panic!("not a mapping: {data:?}");
        };
        &pairs
            .iter()
            .find(|(k, _)| *k == s(key))
            .unwrap_or_else(|| panic!("no key {key}"))
            .1
    }

    #[rstest]
    #[case::empty("", Data::Null)]
    #[case::tilde("~", Data::Null)]
    #[case::null("null", Data::Null)]
    #[case::upper_null("NULL", Data::Null)]
    #[case::bool_true("True", Data::Bool(true))]
    #[case::bool_false("false", Data::Bool(false))]
    #[case::yes_is_a_string("yes", s("yes"))]
    #[case::mixed_case_bool("tRUE", s("tRUE"))]
    #[case::int("42", Data::Int(42))]
    #[case::signed_int("-7", Data::Int(-7))]
    #[case::plus_int("+7", Data::Int(7))]
    #[case::leading_zero("012", Data::Int(12))]
    #[case::hex("0x1F", Data::Int(31))]
    #[case::octal("0o17", Data::Int(15))]
    #[case::bad_octal("0o18", s("0o18"))]
    #[case::float("2.5", Data::Float(2.5))]
    #[case::trailing_dot("1.", Data::Float(1.0))]
    #[case::leading_dot(".5", Data::Float(0.5))]
    #[case::exponent("1e3", Data::Float(1000.0))]
    #[case::signed_exponent("-1.5E-2", Data::Float(-0.015))]
    #[case::inf(".inf", Data::Float(f64::INFINITY))]
    #[case::neg_inf("-.Inf", Data::Float(f64::NEG_INFINITY))]
    #[case::nan(".NaN", Data::Float(f64::NAN))]
    #[case::date("2026-01-01", s("2026-01-01"))]
    #[case::version("1.2.3", s("1.2.3"))]
    #[case::just_dot(".", s("."))]
    #[case::just_sign("-", s("-"))]
    #[case::big_int("99999999999999999999", Data::Float(1e20))]
    #[case::big_hex("0xFFFFFFFFFFFFFFFFFF", Data::Float(4_722_366_482_869_645_213_696.0))]
    #[case::max_int("9223372036854775807", Data::Int(i64::MAX))]
    #[case::min_int("-9223372036854775808", Data::Int(i64::MIN))]
    #[case::text("hello", s("hello"))]
    fn resolve_cases(#[case] text: &str, #[case] expected: Data) {
        assert!(
            data_eq(&resolve_plain(text), &expected),
            "{text:?} resolved to {:?}",
            resolve_plain(text)
        );
    }

    #[test]
    fn quoted_and_tagged_are_strings() {
        let data = fm_data("a: \"1\"\nb: !!str 2\nc: !foo 3\nd: '~'\ne: |\n  4\nf: 5\n");
        assert_eq!(entry(&data, "a"), &s("1"));
        assert_eq!(entry(&data, "b"), &s("2"));
        assert_eq!(entry(&data, "c"), &Data::Int(3));
        assert_eq!(entry(&data, "d"), &s("~"));
        assert_eq!(entry(&data, "e"), &s("4\n"));
        assert_eq!(entry(&data, "f"), &Data::Int(5));
    }

    #[test]
    fn nulls_and_non_string_keys() {
        let data = fm_data("a:\nb: ~\n1: x\ntrue: y\n");
        assert_eq!(entry(&data, "a"), &Data::Null);
        assert_eq!(entry(&data, "b"), &Data::Null);
        let Data::Map(pairs) = &data else { panic!() };
        assert_eq!(pairs[2], (Data::Int(1), s("x")));
        assert_eq!(pairs[3], (Data::Bool(true), s("y")));
    }

    #[test]
    fn aliases_expand() {
        let data = data_of(include_str!(
            "../../../tests/fixtures/edge/anchors-aliases.md"
        ));
        let defaults = entry(&data, "defaults");
        assert_eq!(
            defaults,
            &Data::Map(vec![
                (s("author"), s("Jane")),
                (s("draft"), Data::Bool(false))
            ])
        );
        assert!(data_eq(entry(&data, "other"), defaults));
        let Data::Map(post) = entry(&data, "post") else {
            panic!("post is not a mapping");
        };
        assert_eq!(post[0], (s("<<"), defaults.clone()));
        assert_eq!(post[1], (s("title"), s("Post")));
    }

    #[test]
    fn latest_anchor_wins() {
        let data = fm_data("a: &x 1\nb: *x\nc: &x 2\nd: *x\n");
        assert_eq!(entry(&data, "b"), &Data::Int(1));
        assert_eq!(entry(&data, "d"), &Data::Int(2));
    }

    #[test]
    fn nested_aliases_expand() {
        let data = fm_data("a: &a [1, 2]\nb: &b [*a, *a]\nc: *b\n");
        let pair = Data::Seq(vec![Data::Int(1), Data::Int(2)]);
        assert_eq!(
            entry(&data, "c"),
            &Data::Seq(vec![pair.clone(), pair.clone()])
        );
    }

    fn bomb() -> String {
        let mut fm = "a0: &a0 [x]\n".to_string();
        for i in 1..10 {
            let items = vec![format!("*a{}", i - 1); 10].join(", ");
            fm.push_str(&format!("a{i}: &a{i} [{items}]\n"));
        }
        fm
    }

    #[test]
    fn alias_bomb() {
        let source = wrap(&bomb());
        let lines = lint_all(&source, Config::default());
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(
            lines[0].ends_with("input-limit aliases expand to more than 100,000 nodes"),
            "{lines:?}"
        );
    }

    #[test]
    fn alias_bomb_points_at_the_alias() {
        let source = wrap(&bomb());
        let Extraction::Found(fm) = extract(&source) else {
            panic!()
        };
        let root = parse(&source, &fm).expect("parses");
        let issue = build(&root).unwrap_err();
        assert_eq!(issue.rule, RuleId::InputLimit);
        assert_eq!(&source[issue.range.start..issue.range.end], "*a4");
    }

    fn big_string_aliased(times: usize) -> String {
        let aliases = vec!["*s"; times].join(", ");
        wrap(&format!(
            "a: &s \"{}\"\nb: [{aliases}]\n",
            "x".repeat(64 * 1024)
        ))
    }

    #[test]
    fn alias_text_is_limited() {
        let started = std::time::Instant::now();
        let source = big_string_aliased(300);
        let lines = lint_all(&source, Config::default());
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(
            lines[0].ends_with("input-limit aliases expand to more than 16 MiB of text"),
            "{lines:?}"
        );
        assert!(started.elapsed().as_secs() < 5, "{:?}", started.elapsed());

        let Extraction::Found(fm) = extract(&source) else {
            panic!()
        };
        let root = parse(&source, &fm).expect("parses");
        let issue = build(&root).unwrap_err();
        assert_eq!(issue.rule, RuleId::InputLimit);
        // The 257th alias is the first to cross 16 MiB.
        assert_eq!(&source[issue.range.start..issue.range.end], "*s");
    }

    #[test]
    fn alias_text_under_the_limit_builds() {
        let source = big_string_aliased(200);
        let Data::Map(pairs) = data_of(&source) else {
            panic!()
        };
        let Data::Seq(items) = &pairs[1].1 else {
            panic!()
        };
        assert_eq!(items.len(), 200);
    }

    #[test]
    fn alias_chain_is_depth_limited() {
        let mut fm = "a0: &a0 [x]\n".to_string();
        for i in 1..200 {
            fm.push_str(&format!("a{i}: &a{i} [*a{}]\n", i - 1));
        }
        let source = wrap(&fm);
        let lines = lint_all(&source, Config::default());
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(
            lines[0].ends_with("input-limit nesting is deeper than 128 levels"),
            "{lines:?}"
        );
    }

    #[test]
    fn many_plain_scalars_are_fine() {
        let fm: String = (0..50_000).map(|i| format!("k{i}: {i}\n")).collect();
        let Data::Map(pairs) = fm_data(&fm) else {
            panic!()
        };
        assert_eq!(pairs.len(), 50_000);
    }

    #[test]
    fn unknown_alias_is_a_syntax_issue() {
        let range = TextRange { start: 4, end: 7 };
        let leaf = |kind| Node {
            kind,
            range,
            anchor: None,
            tagged_str: false,
            has_tag: false,
        };
        let root = leaf(NodeKind::Sequence {
            flow: true,
            items: vec![leaf(NodeKind::Alias { anchor: 9 })],
        });
        let issue = build(&root).unwrap_err();
        assert_eq!(issue.rule, RuleId::YamlSyntax);
        assert_eq!(issue.message, "unknown alias");
        assert_eq!(issue.range, range);
    }

    #[test]
    fn json_conversion() {
        let data = fm_data("1: a\nb: .inf\nc: [1, 2.5, null]\n");
        assert_eq!(
            to_json(&data).to_string(),
            r#"{"1":"a","b":".inf","c":[1,2.5,null]}"#
        );
    }

    #[test]
    fn json_special_floats_and_keys() {
        let data =
            fm_data("a: -.inf\nb: .nan\ntrue: 1\nnull: 2\n2.5: 3\n? [1, b]\n: 4\n? {x: 1}\n: 5\n");
        assert_eq!(
            to_json(&data).to_string(),
            r#"{"2.5":3,"[1,\"b\"]":4,"a":"-.inf","b":".nan","null":2,"true":1,"{\"x\":1}":5}"#
        );
    }

    #[test]
    fn json_big_int_is_a_float_number() {
        let data = fm_data("a: 99999999999999999999\nb: 3\nc: 1e3\n");
        assert_eq!(
            to_json(&data).to_string(),
            r#"{"a":1e+20,"b":3,"c":1000.0}"#
        );
    }

    #[test]
    fn data_eq_nan() {
        let nan = Data::Float(f64::NAN);
        assert!(data_eq(&nan, &Data::Float(-f64::NAN)));
        assert!(data_eq(
            &Data::Seq(vec![nan.clone()]),
            &Data::Seq(vec![nan])
        ));
        assert!(!data_eq(&Data::Float(0.0), &Data::Float(-0.0)));
        assert!(data_eq(&Data::Float(1.5), &Data::Float(1.5)));
    }

    #[test]
    fn data_eq_is_structural() {
        assert!(!data_eq(&Data::Int(1), &Data::Float(1.0)));
        assert!(!data_eq(&s("a"), &Data::Null));
        let map = |k: &str, v: i64| Data::Map(vec![(s(k), Data::Int(v))]);
        assert!(data_eq(&map("a", 1), &map("a", 1)));
        assert!(!data_eq(&map("a", 1), &map("a", 2)));
        assert!(!data_eq(&map("a", 1), &map("b", 1)));
        assert!(!data_eq(&Data::Seq(vec![]), &Data::Seq(vec![Data::Null])));
    }

    #[test]
    fn lookup_follows_paths() {
        let data = fm_data("a: [x, {k: v}]\nb: 1\n");
        let at = |path: &[PathStep]| lookup(&data, path);
        assert_eq!(at(&[]), Some(&data));
        assert_eq!(at(&[PathStep::Key(0)]), Some(&s("a")));
        assert_eq!(at(&[PathStep::Value(1)]), Some(&Data::Int(1)));
        assert_eq!(at(&[PathStep::Value(0), PathStep::Item(0)]), Some(&s("x")));
        assert_eq!(
            at(&[PathStep::Value(0), PathStep::Item(1), PathStep::Key(0)]),
            Some(&s("k"))
        );
        assert_eq!(at(&[PathStep::Value(5)]), None);
        assert_eq!(at(&[PathStep::Item(0)]), None);
        assert_eq!(at(&[PathStep::Value(1), PathStep::Key(0)]), None);
    }
}
