//! `fmlint-disable-line` and `fmlint-disable-next-line` comments.

use std::collections::HashSet;

use crate::RuleId;
use crate::document::Document;
use crate::lint::Finding;
use crate::rules::invalid_disable_comment::{cant_disable, unknown};

/// The `(rule, line)` pairs that disable comments cover; lines are indexes into `Document::lines`.
#[derive(Debug, Default)]
pub(crate) struct Disables(HashSet<(RuleId, usize)>);

impl Disables {
    /// Whether a comment disables `rule` on `line`. Only configurable rules can be covered.
    pub(crate) fn covers(&self, rule: RuleId, line: usize) -> bool {
        rule.is_configurable() && self.0.contains(&(rule, line))
    }
}

/// The disable comments of `doc`, and an `invalid-disable-comment` finding for each bad rule name.
pub(crate) fn collect(source: &str, doc: &Document) -> (Disables, Vec<Finding>) {
    let mut covered = HashSet::new();
    let mut findings = Vec::new();
    for comment in &doc.comments {
        let Some(text) = source.get(comment.range.start..comment.range.end) else {
            continue;
        };
        let text = text.trim_start_matches('#').trim_start();
        let (offset, rest) = if let Some(rest) = text.strip_prefix("fmlint-disable-next-line") {
            (1, rest)
        } else if let Some(rest) = text.strip_prefix("fmlint-disable-line") {
            (0, rest)
        } else {
            continue;
        };
        if !rest.is_empty() && !rest.starts_with(char::is_whitespace) {
            continue;
        }
        let Some(line) = doc.line_index(comment.range.start) else {
            continue;
        };
        let ids: Vec<&str> = rest
            .split(|c: char| c == ',' || c.is_whitespace())
            .filter(|id| !id.is_empty())
            .collect();
        let mut rules = Vec::new();
        for id in &ids {
            match RuleId::from_name(id) {
                Some(rule) if rule.is_configurable() => rules.push(rule),
                known => findings.push(Finding {
                    range: comment.range,
                    message: if known.is_some() {
                        cant_disable(id)
                    } else {
                        unknown(id)
                    },
                    fix: None,
                }),
            }
        }
        if ids.is_empty() {
            rules.extend(RuleId::ALL.into_iter().filter(|r| r.is_configurable()));
        }
        covered.extend(rules.into_iter().map(|rule| (rule, line + offset)));
    }
    (Disables(covered), findings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Config;
    use crate::testing::{document, lint_all, wrap};

    const DISABLE_COMMENTS: &str = include_str!("../../../tests/fixtures/edge/disable-comments.md");

    #[test]
    #[ignore = "enabled in Task 9"]
    fn fixture_disable_comments() {
        assert_eq!(
            lint_all(DISABLE_COMMENTS, Config::default()),
            [
                "4:19-4:20 no-trailing-spaces trailing whitespace",
                "6:1-6:38 invalid-disable-comment unknown rule `not-a-rule` in disable comment",
                "7:7-7:8 no-trailing-spaces trailing whitespace",
            ]
        );
    }

    /// Covered `(rule, 1-based line)` pairs, sorted, and the finding messages.
    fn cases(fm: &str) -> (Vec<(RuleId, usize)>, Vec<String>) {
        let source = wrap(fm);
        let (_, doc) = document(&source, &[]);
        let (disables, findings) = collect(&source, &doc);
        let mut covered: Vec<_> = disables.0.iter().map(|&(r, l)| (r, l + 1)).collect();
        covered.sort();
        (covered, findings.into_iter().map(|f| f.message).collect())
    }

    #[test]
    fn unit_cases() {
        let all: Vec<_> = RuleId::ALL
            .into_iter()
            .filter(|r| r.is_configurable())
            .collect();

        // Line 2 is the comment; the next line is 3.
        let (covered, findings) = cases("# fmlint-disable-next-line no-commas, no-quotes\na: 1\n");
        assert_eq!(covered, [(RuleId::NoQuotes, 3), (RuleId::NoCommas, 3)]);
        assert!(findings.is_empty());

        let (covered, findings) = cases("a: 1 # fmlint-disable-line yaml-syntax\n");
        assert!(covered.is_empty());
        assert_eq!(findings, ["rule `yaml-syntax` can't be disabled"]);

        let (covered, findings) = cases("# fmlint-disable-lines\na: 1\n");
        assert!(covered.is_empty() && findings.is_empty());

        let (covered, _) = cases("a: 1 # fmlint-disable-line\n");
        assert_eq!(covered, all.iter().map(|&r| (r, 2)).collect::<Vec<_>>());

        // An unknown name does not turn the comment into "every rule".
        let (covered, findings) = cases("#fmlint-disable-line nope\n");
        assert!(covered.is_empty());
        assert_eq!(findings, ["unknown rule `nope` in disable comment"]);
    }

    #[test]
    fn run_reports_invalid_comments_and_cannot_disable_them() {
        let source = wrap(
            "# fmlint-disable-line nope\n# fmlint-disable-next-line invalid-disable-comment\n# fmlint-disable-line\n",
        );
        assert_eq!(
            lint_all(&source, Config::default()),
            [
                "2:1-2:27 invalid-disable-comment unknown rule `nope` in disable comment",
                "3:1-3:51 invalid-disable-comment rule `invalid-disable-comment` can't be disabled",
            ]
        );
    }
}
