//! `no-quotes`: a quoted string that reads the same unquoted should be unquoted.
//!
//! # Why
//!
//! Quotes around a plain word are noise, and mixing quoted and unquoted strings makes
//! a file harder to scan.
//!
//! # Examples
//!
//! Bad:
//!
//! ```yaml
//! title: "Hello"
//! 'tag': 'it''s'
//! ```
//!
//! Good:
//!
//! ```yaml
//! title: Hello
//! tag: it's
//! ```
//!
//! Quotes that are needed stay: `"a: b"`, `'@h'`, `""`, and the words and numbers
//! that YAML 1.1 readers turn into booleans, numbers or dates (`"yes"`, `"on"`,
//! `"0777"`, `"2026-01-01"`).
//!
//! # Fix
//!
//! Replaces the quoted scalar, quotes included, with its text. The fix is only offered
//! when the file, read again with the text unquoted, still has the same string there.

use std::sync::LazyLock;

use regex::{Regex, RegexSet};

use crate::data::{Data, build, lookup};
use crate::edit::apply_edits;
use crate::extract::{Extraction, extract};
use crate::lint::{Context, Finding};
use crate::parse::parse;
use crate::tree::{NodeKind, Style, walk};
use crate::{Applicability, DataEffect, Fix, PathStep, TextEdit, TextRange};

/// The message for every finding.
const MESSAGE: &str = "unnecessary quotes";

/// The most bytes one `check` parses in modified copies of the file.
///
/// One parse covers all candidates when they all pass; each further parse re-checks one
/// candidate (or half of a batch that did not parse). Counting bytes keeps small files exact
/// and a 1 MiB file to about 8 extra parses; candidates left unchecked when the budget runs
/// out are not flagged.
const MAX_REPARSE_BYTES: usize = 8 * 1024 * 1024;

/// Texts a YAML 1.1 reader turns into a bool, null, number, timestamp or merge key.
const YAML_1_1: &[&str] = &[
    r"^(y|Y|yes|Yes|YES|n|N|no|No|NO|true|True|TRUE|false|False|FALSE|on|On|ON|off|Off|OFF)$",
    r"^(~|null|Null|NULL)$",
    r"^[-+]?0b[0-1_]+$",
    r"^[-+]?0[0-7_]+$",
    r"^[-+]?(0|[1-9][0-9_]*)$",
    r"^[-+]?0x[0-9a-fA-F_]+$",
    r"^[-+]?[1-9][0-9_]*(:[0-5]?[0-9])+$",
    r"^[-+]?([0-9][0-9_]*)?\.[0-9.]*([eE][-+][0-9]+)?$",
    r"^[-+]?[0-9][0-9_]*(:[0-5]?[0-9])+\.[0-9_]*$",
    r"^[-+]?\.(inf|Inf|INF)$",
    r"^\.(nan|NaN|NAN)$",
    r"^[0-9]{4}-[0-9]{1,2}-[0-9]{1,2}([Tt]|[ \t]+)?([0-9]{1,2}:[0-9]{2}:[0-9]{2}(\.[0-9]*)?([ \t]*(Z|[-+][0-9]{1,2}(:[0-9]{2})?))?)?$",
    r"^(<<|=)$",
];

static YAML_1_1_SET: LazyLock<RegexSet> = LazyLock::new(|| {
    // The patterns are constants, checked by the tests.
    #[allow(clippy::expect_used)]
    RegexSet::new(YAML_1_1).expect("valid YAML 1.1 regexes")
});

/// Numbers with `,` separators, which Ruby's Psych (Jekyll's reader) reads as numbers.
static COMMA_NUMBER: LazyLock<Regex> = LazyLock::new(|| {
    // The pattern is a constant, checked by the tests.
    #[allow(clippy::expect_used)]
    Regex::new(r"^[-+]?(0b[01_,]+|0x[0-9a-fA-F_,]+|[0-9][0-9_,]*(\.[0-9_,]*)?([eE][-+]?[0-9]+)?)$")
        .expect("valid comma-number regex")
});

/// Characters that are control, non-printable or line breaks to some YAML reader (ruling T15a),
/// so a scalar holding one keeps its quotes and escapes.
fn unsafe_char(c: char) -> bool {
    c < ' '
        || ('\u{7f}'..='\u{9f}').contains(&c)
        || matches!(c, '\u{2028}' | '\u{2029}' | '\u{fffe}' | '\u{ffff}')
}

/// A quoted scalar that may be written plain.
struct Candidate {
    /// The scalar, quotes included.
    range: TextRange,
    /// The text, which is also the plain form.
    value: String,
    /// Where the scalar is in the tree.
    path: Vec<PathStep>,
}

/// Text that can never be a plain scalar there, whatever the rest of the file looks like.
///
/// Only a cheap first filter: the re-parse decides every other case.
fn never_plain(value: &str, in_flow: bool) -> bool {
    let mut chars = value.chars();
    let (Some(first), second) = (chars.next(), chars.next()) else {
        return true;
    };
    matches!(
        first,
        ',' | '['
            | ']'
            | '{'
            | '}'
            | '#'
            | '&'
            | '*'
            | '!'
            | '|'
            | '>'
            | '\''
            | '"'
            | '%'
            | '@'
            | '`'
    ) || (matches!(first, '-' | '?' | ':') && second.is_none_or(|c| c == ' '))
        || value.contains(": ")
        || value.contains(" #")
        || (in_flow && value.contains([',', '[', ']', '{', '}']))
}

/// The quoted scalars worth re-parsing, in source order.
fn candidates(ctx: &Context<'_>) -> Vec<Candidate> {
    let mut found = Vec::new();
    // `nodes()` and `walk` both go in pre-order, keys before values.
    let mut refs = ctx.doc.nodes().into_iter();
    walk(&ctx.doc.root, &mut |node, path| {
        let Some(r) = refs.next() else {
            return;
        };
        let NodeKind::Scalar {
            value,
            style: Style::SingleQuoted | Style::DoubleQuoted,
        } = &node.kind
        else {
            return;
        };
        if node.tagged_str
            || value.is_empty()
            || value.starts_with(' ')
            || value.ends_with(' ')
            || value.chars().any(unsafe_char)
            || YAML_1_1_SET.is_match(value)
            || (value.contains(',') && COMMA_NUMBER.is_match(value))
            || never_plain(value, r.in_flow)
        {
            return;
        }
        found.push(Candidate {
            range: node.range,
            value: value.clone(),
            path: path.to_vec(),
        });
    });
    found.sort_by_key(|c| c.range.start);
    found
}

/// The data of the file with every scalar in `cands` written plain; `None` if that does not
/// parse.
fn reparse(ctx: &Context<'_>, cands: &[&Candidate]) -> Option<Data> {
    let edits: Vec<TextEdit> = cands
        .iter()
        .map(|c| TextEdit {
            range: c.range,
            new_text: c.value.clone(),
        })
        .collect();
    // The body cannot matter, and may be large.
    let text = apply_edits(ctx.source.get(..ctx.fm.end)?, &edits)?;
    let Extraction::Found(fm) = extract(&text) else {
        return None;
    };
    build(&parse(&text, &fm).ok()?).ok()
}

/// Adds to `ok` the candidates of `cands` that read back as the same string when plain.
///
/// All of them are tried in one parse; a failing one is then tried alone, and a batch that
/// does not parse is halved, so a few bad candidates cost a few parses, not one each.
fn verify<'a>(
    ctx: &Context<'_>,
    cands: &[&'a Candidate],
    budget: &mut usize,
    ok: &mut Vec<&'a Candidate>,
) {
    let cost = ctx.fm.end;
    if cands.is_empty() || *budget < cost {
        return;
    }
    *budget -= cost;
    if let Some(data) = reparse(ctx, cands) {
        let (pass, fail): (Vec<_>, Vec<_>) = cands
            .iter()
            .partition(|c| matches!(lookup(&data, &c.path), Some(Data::Str(s)) if *s == c.value));
        ok.extend(pass);
        if cands.len() > 1 {
            for c in fail {
                verify(ctx, &[c], budget, ok);
            }
        }
    } else if cands.len() > 1 {
        let (left, right) = cands.split_at(cands.len() / 2);
        verify(ctx, left, budget, ok);
        verify(ctx, right, budget, ok);
    }
}

/// Flags quoted scalars that stay the same string when written plain.
pub(crate) fn check(ctx: &Context<'_>) -> Vec<Finding> {
    check_with_budget(ctx, MAX_REPARSE_BYTES)
}

/// [`check`] with a parse budget in bytes, so a test can run it out.
fn check_with_budget(ctx: &Context<'_>, mut budget: usize) -> Vec<Finding> {
    let cands = candidates(ctx);
    let refs: Vec<&Candidate> = cands.iter().collect();
    let mut ok = Vec::new();
    verify(ctx, &refs, &mut budget, &mut ok);
    ok.sort_by_key(|c| c.range.start);
    ok.into_iter()
        .map(|c| Finding {
            range: c.range,
            message: MESSAGE.to_string(),
            fix: Some(Fix {
                applicability: Applicability::Automatic,
                data_effect: DataEffect::Preserving,
                edits: vec![TextEdit {
                    range: c.range,
                    new_text: c.value.clone(),
                }],
            }),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::check_with_budget;
    use crate::lint::Context;
    use crate::testing::{fix_fm, lint_rule};
    use crate::{Config, RuleId};

    const RULE: RuleId = RuleId::NoQuotes;
    const MSG: &str = "unnecessary quotes";

    #[rstest]
    #[case::double("title: \"hello\"\n", &["2:8-2:15"], "title: hello\n")]
    #[case::single("a: 'world'\n", &["2:4-2:11"], "a: world\n")]
    #[case::key("\"k\": v\n", &["2:1-2:4"], "k: v\n")]
    #[case::apostrophe("a: 'it''s'\n", &["2:4-2:11"], "a: it's\n")]
    #[case::comma_text("a: \"a, b\"\n", &["2:4-2:10"], "a: a, b\n")]
    // The comment keeps `no-flow-sequence` from rewriting the list.
    #[case::in_flow("l: [\"x\", y] # c\n", &["2:5-2:8"], "l: [x, y] # c\n")]
    #[case::escape("a: \"caf\\u00e9\"\n", &["2:4-2:15"], "a: café\n")]
    #[case::item("l:\n  - 'x'\n", &["3:5-3:8"], "l:\n  - x\n")]
    #[case::two("a: 'x'\nb: \"z\"\n", &["2:4-2:7", "3:4-3:7"], "a: x\nb: z\n")]
    // `1e3` is a number for YAML 1.2 (and not for 1.1), so it keeps its quotes; the others
    // in the file are still flagged, by the one-at-a-time fallback.
    #[case::one_bad_among_good("a: 'x'\nb: \"1e3\"\nc: 'z'\n", &["2:4-2:7", "4:4-4:7"], "a: x\nb: \"1e3\"\nc: z\n")]
    // `...` ends the document, so unquoting it breaks the file; the others are still found.
    #[case::unparsable_among_good("a: 'x'\nl:\n  - \"...\"\nc: 'z'\n", &["2:4-2:7", "5:4-5:7"], "a: x\nl:\n  - \"...\"\nc: z\n")]
    // A key that must stay quoted does not stop the keys after it.
    #[case::key_needing_quotes("\"- x\": 1\n\"z\": 2\n", &["3:1-3:4"], "\"- x\": 1\nz: 2\n")]
    fn flagged(#[case] fm: &str, #[case] want: &[&str], #[case] fixed: &str) {
        let want: Vec<String> = want.iter().map(|r| format!("{r} {MSG}")).collect();
        assert_eq!(lint_rule(RULE, fm, Config::default()), want);
        assert_eq!(fix_fm(fm, Config::default()), fixed);
    }

    #[rstest]
    #[case::at("'@h'")]
    #[case::colon_space("\"a: b\"")]
    #[case::comment("\"x #y\"")]
    #[case::alias("'*s'")]
    #[case::int("\"123\"")]
    #[case::null("\"null\"")]
    #[case::bool("\"true\"")]
    #[case::yes("\"yes\"")]
    #[case::on("\"on\"")]
    #[case::octal("\"0777\"")]
    #[case::underscore_int("\"1_000\"")]
    #[case::sexagesimal("\"12:30\"")]
    #[case::date("\"2026-01-01\"")]
    #[case::empty("\"\"")]
    #[case::leading_space("\" x\"")]
    #[case::trailing_space("'x '")]
    #[case::tab_escape("\"a\\tb\"")]
    #[case::tab_literal("\"a\tb\"")]
    #[case::flow_seq("\"[x]\"")]
    #[case::dash("\"- x\"")]
    #[case::merge("\"<<\"")]
    #[case::float("\"1e3\"")]
    #[case::newline("\"a\\nb\"")]
    #[case::control("\"\\u0001\"")]
    #[case::c1_control("\"\\u0080\"")]
    #[case::next_line("\"\\u0085\"")]
    #[case::line_separator("\"\\u2028\"")]
    #[case::paragraph_separator("\"\\u2029\"")]
    #[case::non_character("\"\\ufffe\"")]
    #[case::comma_int("\"1,000\"")]
    #[case::comma_float("\"1,000.5\"")]
    #[case::comma_binary("\"0b1,0\"")]
    #[case::document_end("\"...\"")]
    fn needed(#[case] quoted: &str) {
        let fm = format!("a: {quoted}\n");
        assert_eq!(
            lint_rule(RULE, &fm, Config::default()),
            Vec::<String>::new()
        );
        assert_eq!(fix_fm(&fm, Config::default()), fm);
    }

    #[rstest]
    #[case::comma("l: [\"a,b\"]\n")]
    #[case::bracket("l: [\"a]\"]\n")]
    #[case::key_colon("\"a: b\": 1\n")]
    #[case::tagged("a: !!str \"x\"\n")]
    fn needed_in_context(#[case] fm: &str) {
        assert_eq!(lint_rule(RULE, fm, Config::default()), Vec::<String>::new());
    }

    // Plain in a block, but not in a flow collection: the comma splits it.
    #[test]
    fn comma_is_fine_in_a_block() {
        let want = [format!("2:4-2:9 {MSG}")];
        assert_eq!(lint_rule(RULE, "a: \"a,b\"\n", Config::default()), want);
    }

    #[test]
    fn crlf() {
        let source = "---\r\na: 'x'\r\n---\r\n";
        assert_eq!(
            crate::testing::lint_src(RULE, source, Config::default()),
            [format!("2:4-2:7 {MSG}")]
        );
        assert_eq!(
            crate::testing::fix_src(source, Config::default()),
            "---\r\na: x\r\n---\r\n"
        );
    }

    // Unquoting `b:` breaks the file (`b: b:`), so the batch does not parse and is halved.
    // Whatever the budget cannot reach is not flagged.
    #[test]
    fn unchecked_candidates_are_not_flagged_when_the_budget_runs_out() {
        let source = crate::testing::wrap("a: 'x'\nb: 'b:'\nc: 'z'\n");
        let (fm, doc) = crate::testing::document(&source, &[]);
        let config = Config::default();
        let ctx = Context::new(&source, &fm, &doc, &config);
        assert_eq!(check_with_budget(&ctx, usize::MAX).len(), 2);
        // The batch, then the first half: `a` is found, `c` is never reached.
        assert_eq!(check_with_budget(&ctx, 2 * fm.end).len(), 1);
        assert!(check_with_budget(&ctx, fm.end).is_empty());
        assert!(check_with_budget(&ctx, fm.end - 1).is_empty());
    }

    // Many bad candidates: nothing panics, the bad ones are never flagged, and the good
    // ones are all found by the first batch.
    #[test]
    fn many_bad_candidates_stay_correct() {
        let mut fm = String::new();
        for i in 0..300 {
            fm.push_str(&format!("a{i}: \"1e3\"\nb{i}: 'x'\n"));
        }
        let got = lint_rule(RULE, &fm, Config::default());
        assert_eq!(got.len(), 300);
        assert!(got.iter().all(|d| d.ends_with(MSG)));
    }
}
