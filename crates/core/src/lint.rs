//! The rule context and the rule runner.

use crate::config::{Config, Level};
use crate::document::Document;
use crate::extract::FrontMatter;
use crate::rules::RULES;
use crate::{Diagnostic, Fix, RuleId, Severity, TextRange};

/// What a rule looks at.
#[allow(dead_code)] // read by the rules
pub(crate) struct Context<'a> {
    /// The whole source file.
    pub source: &'a str,
    /// Where the front matter is.
    pub fm: &'a FrontMatter,
    /// The front matter, parsed.
    pub doc: &'a Document,
    /// The configuration.
    pub config: &'a Config,
    /// The first line ending in `source`, `"\n"` if there is none.
    pub line_ending: &'static str,
}

impl<'a> Context<'a> {
    pub(crate) fn new(
        source: &'a str,
        fm: &'a FrontMatter,
        doc: &'a Document,
        config: &'a Config,
    ) -> Context<'a> {
        let crlf = source
            .find('\n')
            .and_then(|i| source.get(..i))
            .is_some_and(|before| before.ends_with('\r'));
        Context {
            source,
            fm,
            doc,
            config,
            line_ending: if crlf { "\r\n" } else { "\n" },
        }
    }
}

/// A problem a rule found; the runner adds the rule and severity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Finding {
    /// Where the problem is.
    pub range: TextRange,
    /// The message shown to the user.
    pub message: String,
    /// A fix, if one exists.
    pub fix: Option<Fix>,
}

/// A rule's check.
pub(crate) type RuleFn = fn(&Context<'_>) -> Vec<Finding>;

/// The severity of a level; `None` for a rule that is off.
pub(crate) fn severity(level: Level) -> Option<Severity> {
    match level {
        Level::Off => None,
        Level::Warn => Some(Severity::Warning),
        Level::Error => Some(Severity::Error),
    }
}

/// Runs every rule that is not off, in source order.
pub(crate) fn run(ctx: &Context<'_>) -> Vec<Diagnostic> {
    run_rules(ctx, RULES)
}

fn run_rules(ctx: &Context<'_>, rules: &[(RuleId, RuleFn)]) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for &(rule, check) in rules {
        let Some(severity) = severity(ctx.config.level(rule)) else {
            continue;
        };
        for finding in check(ctx) {
            let ignored = ctx
                .doc
                .line_index(finding.range.start)
                .and_then(|i| ctx.doc.lines.get(i))
                .is_some_and(|line| line.ignored);
            if rule.is_style() && ignored {
                continue;
            }
            diagnostics.push(Diagnostic {
                rule,
                severity,
                message: finding.message,
                range: finding.range,
                fix: finding.fix,
            });
        }
    }
    // Stable, so findings of one rule at one place keep the rule's order.
    diagnostics.sort_by_key(|d| (d.range.start, d.rule.as_str()));
    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{document, wrap};

    fn finding(start: usize, message: &str) -> Finding {
        Finding {
            range: TextRange {
                start,
                end: start + 1,
            },
            message: message.to_string(),
            fix: None,
        }
    }

    // Offsets in `wrap("raw: 1\nb: 2\n")`: line 2 starts at 4, line 3 at 11.
    fn style_at_4_and_11(_: &Context<'_>) -> Vec<Finding> {
        vec![finding(11, "second"), finding(4, "first")]
    }

    fn other_at_4(_: &Context<'_>) -> Vec<Finding> {
        vec![finding(4, "other-a"), finding(4, "other-b")]
    }

    fn run_with(
        config: &Config,
        rules: &[(RuleId, RuleFn)],
    ) -> Vec<(usize, RuleId, Severity, String)> {
        let source = wrap("raw: 1\nb: 2\n");
        let (fm, doc) = document(&source, &config.ignore_keys);
        let ctx = Context::new(&source, &fm, &doc, config);
        run_rules(&ctx, rules)
            .into_iter()
            .map(|d| (d.range.start, d.rule, d.severity, d.message))
            .collect()
    }

    #[test]
    fn runner_sorts_and_maps_severity() {
        let rules: &[(RuleId, RuleFn)] = &[
            (RuleId::NoQuotes, style_at_4_and_11),
            (RuleId::NoCommas, other_at_4),
            (RuleId::Indentation, other_at_4),
        ];
        let got = run_with(&Config::default(), rules);
        let want = [
            (4, RuleId::Indentation, Severity::Error, "other-a"),
            (4, RuleId::Indentation, Severity::Error, "other-b"),
            (4, RuleId::NoCommas, Severity::Warning, "other-a"),
            (4, RuleId::NoCommas, Severity::Warning, "other-b"),
            (4, RuleId::NoQuotes, Severity::Error, "first"),
            (11, RuleId::NoQuotes, Severity::Error, "second"),
        ]
        .map(|(o, r, s, m)| (o, r, s, m.to_string()));
        assert_eq!(got, want);
    }

    #[test]
    fn runner_skips_rules_that_are_off() {
        let mut config = Config::default();
        config.rules.insert(RuleId::NoQuotes, Level::Off);
        let got = run_with(&config, &[(RuleId::NoQuotes, style_at_4_and_11)]);
        assert!(got.is_empty());
    }

    #[test]
    fn runner_drops_style_findings_on_ignored_lines() {
        let config = Config {
            ignore_keys: vec!["raw".to_string()],
            ..Config::default()
        };
        let rules: &[(RuleId, RuleFn)] = &[
            (RuleId::NoQuotes, style_at_4_and_11),
            // `required-keys` is not a style rule, so it is kept.
            (RuleId::RequiredKeys, other_at_4),
        ];
        let got: Vec<_> = run_with(&config, rules)
            .into_iter()
            .map(|(offset, rule, _, message)| (offset, rule, message))
            .collect();
        assert_eq!(
            got,
            [
                (4, RuleId::RequiredKeys, "other-a".to_string()),
                (4, RuleId::RequiredKeys, "other-b".to_string()),
                (11, RuleId::NoQuotes, "second".to_string()),
            ]
        );
    }

    #[test]
    fn context_line_ending() {
        let config = Config::default();
        for (source, want) in [
            ("---\na: 1\n---\n", "\n"),
            ("---\r\na: 1\r\n---\r\n", "\r\n"),
            ("---\na: 1\n---\r\nb\r\n", "\n"),
        ] {
            let (fm, doc) = document(source, &[]);
            assert_eq!(Context::new(source, &fm, &doc, &config).line_ending, want);
        }
    }
}
