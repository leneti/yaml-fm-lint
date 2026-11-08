//! Property tests of the guarantees `fix` makes (spec §12), on generated front matter.

use proptest::prelude::*;
use proptest::test_runner::FileFailurePersistence;
use yaml_fm_lint_core::invariants::check_fix;
use yaml_fm_lint_core::{Config, Linter};

/// Quoted scalars that must stay quoted (the "needed" list of `no-quotes`).
const NEEDED: &[&str] = &[
    "\"a: b\"",
    "'@h'",
    "\"x #y\"",
    "'*s'",
    "\"123\"",
    "\"null\"",
    "\"true\"",
    "\"yes\"",
    "\"on\"",
    "\"0777\"",
    "\"1_000\"",
    "\"12:30\"",
    "\"2026-01-01\"",
    "\"\"",
    "\" x\"",
    "'x '",
    "\"[x]\"",
    "\"- x\"",
    "\"<<\"",
    "\"1e3\"",
    "\"a\\nb\"",
    "\"1,000\"",
    "\"...\"",
];

/// Words that read as non-strings when plain.
const SPECIAL_WORDS: &[&str] = &["yes", "true", "null", "5", "0x1f", "1.5", "~", ".inf"];

/// A value on the right of `key:` or after `- `.
#[derive(Debug, Clone)]
enum Value {
    /// The text goes on the same line.
    Inline(String),
    /// A `|` or `>` block of lines; the lines may be empty or end in spaces.
    Block {
        folded: bool,
        lines: Vec<String>,
    },
    Map(Vec<(String, Value)>),
    Seq(Vec<Value>),
}

fn word() -> impl Strategy<Value = String> {
    prop_oneof![
        8 => "[a-z]{1,8}",
        1 => prop::sample::select(SPECIAL_WORDS).prop_map(str::to_string),
        1 => "[a-z]{1,4}, [a-z]{1,4}",
        1 => "[a-z]{1,4}  [a-z]{1,4}",
    ]
}

fn key() -> impl Strategy<Value = String> {
    prop_oneof![
        6 => "[a-z]{1,6}",
        1 => "[a-z]{1,6}".prop_map(|k| format!("\"{k}\"")),
        1 => "[a-z]{1,6}".prop_map(|k| format!("'{k}'")),
    ]
}

fn flow() -> impl Strategy<Value = String> {
    prop_oneof![
        prop::collection::vec(word(), 0..4).prop_map(|w| format!("[{}]", w.join(", "))),
        prop::collection::vec(("[a-z]{1,4}", "[a-z]{1,4}"), 0..4).prop_map(|pairs| {
            let pairs: Vec<_> = pairs.iter().map(|(k, v)| format!("{k}: {v}")).collect();
            format!("{{{}}}", pairs.join(", "))
        }),
    ]
}

fn leaf() -> impl Strategy<Value = Value> {
    let block_line = prop_oneof![3 => word(), 1 => Just(String::new()), 1 => word().prop_map(|w| format!("{w}  "))];
    prop_oneof![
        6 => word().prop_map(Value::Inline),
        2 => word().prop_map(|w| Value::Inline(format!("\"{w}\""))),
        2 => word().prop_map(|w| Value::Inline(format!("'{w}'"))),
        2 => prop::sample::select(NEEDED).prop_map(|q| Value::Inline(q.to_string())),
        2 => (any::<bool>(), prop::collection::vec(block_line, 1..4))
            .prop_map(|(folded, lines)| Value::Block { folded, lines }),
        2 => flow().prop_map(Value::Inline),
    ]
}

fn value() -> impl Strategy<Value = Value> {
    leaf().prop_recursive(3, 12, 3, |inner| {
        prop_oneof![
            prop::collection::vec((key(), inner.clone()), 1..4).prop_map(Value::Map),
            prop::collection::vec(inner, 1..4).prop_map(Value::Seq),
        ]
    })
}

fn spaces(n: usize) -> String {
    " ".repeat(n)
}

/// The lines of the mapping `entries`, indented by `pad`.
fn map_lines(entries: &[(String, Value)], pad: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for (key, value) in entries {
        let head = format!("{}{key}:", spaces(pad));
        value_lines(head, value, pad, &mut lines);
    }
    lines
}

/// The lines of the sequence `items`, indented by `pad`.
fn seq_lines(items: &[Value], pad: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for item in items {
        let head = format!("{}-", spaces(pad));
        value_lines(head, item, pad, &mut lines);
    }
    lines
}

/// Appends `head` (a `key:` or `-` line start) and the lines of `value` below it.
fn value_lines(head: String, value: &Value, pad: usize, out: &mut Vec<String>) {
    match value {
        Value::Inline(text) => out.push(format!("{head} {text}")),
        Value::Block { folded, lines } => {
            out.push(format!("{head} {}", if *folded { ">" } else { "|" }));
            for line in lines {
                let indent = if line.is_empty() { 0 } else { pad + 2 };
                out.push(format!("{}{line}", spaces(indent)));
            }
        }
        Value::Map(entries) if head.ends_with('-') => {
            // `- a: 1`: the first entry shares the dash line.
            let mut inner = map_lines(entries, pad + 2).into_iter();
            if let Some(first) = inner.next() {
                out.push(format!("{head} {}", first.trim_start()));
            }
            out.extend(inner);
        }
        Value::Seq(items) if head.ends_with('-') => {
            let mut inner = seq_lines(items, pad + 2).into_iter();
            if let Some(first) = inner.next() {
                out.push(format!("{head} {}", first.trim_start()));
            }
            out.extend(inner);
        }
        Value::Map(entries) => {
            out.push(head);
            out.extend(map_lines(entries, pad + 2));
        }
        Value::Seq(items) => {
            out.push(head);
            out.extend(seq_lines(items, pad + 2));
        }
    }
}

/// A defect added to one rendered line.
#[derive(Debug, Clone, Copy)]
enum Defect {
    TrailingSpaces,
    TrailingTab,
    BlankLineAfter,
    SpaceBeforeColon,
    TwoSpacesAfterColon,
    TrailingComma,
    IndentMore,
    IndentLess,
}

fn defect() -> impl Strategy<Value = Defect> {
    prop_oneof![
        2 => Just(Defect::TrailingSpaces),
        1 => Just(Defect::TrailingTab),
        2 => Just(Defect::BlankLineAfter),
        2 => Just(Defect::SpaceBeforeColon),
        2 => Just(Defect::TwoSpacesAfterColon),
        3 => Just(Defect::TrailingComma),
        1 => Just(Defect::IndentMore),
        1 => Just(Defect::IndentLess),
    ]
}

/// Adds each defect to the line its index picks.
fn with_defects(mut lines: Vec<String>, defects: &[(prop::sample::Index, Defect)]) -> Vec<String> {
    for (index, defect) in defects {
        if lines.is_empty() {
            break;
        }
        let at = index.index(lines.len());
        let Some(line) = lines.get_mut(at) else {
            continue;
        };
        match defect {
            Defect::TrailingSpaces => line.push_str("  "),
            Defect::TrailingTab => line.push('\t'),
            Defect::BlankLineAfter => lines.insert(at + 1, String::new()),
            Defect::SpaceBeforeColon => {
                if let Some(i) = line.find(':') {
                    line.insert(i, ' ');
                }
            }
            Defect::TwoSpacesAfterColon => *line = line.replacen(": ", ":  ", 1),
            // After `key:`, `|`, a quote or a bracket a comma is always a syntax error.
            Defect::TrailingComma => {
                if line.ends_with(|c: char| c.is_ascii_alphanumeric()) {
                    line.push(',');
                }
            }
            Defect::IndentMore => line.insert(0, ' '),
            Defect::IndentLess => {
                if line.starts_with(' ') {
                    line.remove(0);
                }
            }
        }
    }
    lines
}

/// The lines of generated front matter: entries, an optional anchor with a later alias,
/// and up to six defects.
fn front_matter() -> impl Strategy<Value = Vec<String>> {
    let anchored = prop_oneof![word(), flow()];
    (
        prop::collection::vec((key(), value()), 1..=12),
        prop::option::of(anchored),
        prop::collection::vec((any::<prop::sample::Index>(), defect()), 0..=6),
    )
        .prop_map(|(mut entries, anchored, defects)| {
            let mut lines = Vec::new();
            if let Some(text) = anchored {
                lines.push(format!("anc: &a {text}"));
                entries.push(("ref".to_string(), Value::Inline("*a".to_string())));
            }
            lines.extend(map_lines(&entries, 0));
            with_defects(lines, &defects)
        })
}

/// A whole file: front matter with `\n` or `\r\n`, an optional BOM and a body.
fn file() -> impl Strategy<Value = String> {
    let body_line =
        prop::sample::select(&["---", "a: 1 ", "# fmlint-disable-line", "text", "", "- x,"]);
    (
        front_matter(),
        any::<bool>(),
        any::<bool>(),
        prop::collection::vec(body_line, 0..=5),
        any::<bool>(),
    )
        .prop_map(|(lines, crlf, bom, body, final_newline)| {
            let eol = if crlf { "\r\n" } else { "\n" };
            let mut text = String::new();
            if bom {
                text.push('\u{FEFF}');
            }
            text.push_str(&format!("---{eol}"));
            for line in lines {
                text.push_str(&format!("{line}{eol}"));
            }
            text.push_str("---");
            if !body.is_empty() || final_newline {
                text.push_str(eol);
            }
            let body: Vec<_> = body.iter().map(|line| format!("{line}{eol}")).collect();
            text.push_str(&body.concat());
            text
        })
}

/// Arbitrary text between delimiters, for robustness.
fn noise() -> impl Strategy<Value = String> {
    any::<String>().prop_map(|s| format!("---\n{s}\n---\n"))
}

// Clippy only exempts `#[test]` functions, and the config is always valid.
#[allow(clippy::expect_used)]
fn linter(indent_size: u8) -> Linter {
    let config = Config {
        indent_size,
        ..Config::default()
    };
    Linter::new(config, None).expect("valid config")
}

/// 256 cases, or `PROPTEST_CASES`; failures are saved in the crate's `proptest-regressions`.
fn config() -> ProptestConfig {
    ProptestConfig {
        // Relative to the crate directory, where `cargo test` runs the tests.
        failure_persistence: Some(Box::new(FileFailurePersistence::Direct(
            "proptest-regressions/fix_props.txt",
        ))),
        ..ProptestConfig::default()
    }
}

proptest! {
    #![proptest_config(config())]

    #[test]
    fn fix_invariants_hold(input in file()) {
        prop_assert_eq!(check_fix(&linter(2), &input), Ok(()));
    }

    #[test]
    fn fix_invariants_with_size_4(input in file()) {
        prop_assert_eq!(check_fix(&linter(4), &input), Ok(()));
    }

    #[test]
    fn fix_invariants_hold_on_noise(input in noise()) {
        prop_assert_eq!(check_fix(&linter(2), &input), Ok(()));
    }
}
