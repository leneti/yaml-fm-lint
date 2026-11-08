//! The linter: configuration plus the lint pipeline.

use crate::data::build;
use crate::document::Document;
use crate::extract::{Extraction, extract, start_offset};
use crate::lint::{Context, run, severity};
use crate::parse::parse;
use crate::rules::{input_limit, require_front_matter};
use crate::{Config, ConfigError, ConfigErrorKind, Diagnostic, RuleId, TextRange};

/// Lints and fixes front matter according to a [`Config`].
#[derive(Debug, Clone)]
pub struct Linter {
    config: Config,
    schema: Option<jsonschema::Validator>,
}

/// The result of [`Linter::lint`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LintReport {
    /// From the opening delimiter to just after the closing line, if there is front matter.
    pub front_matter: Option<TextRange>,
    /// The findings, in source order.
    pub diagnostics: Vec<Diagnostic>,
}

/// The result of [`Linter::fix`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixOutcome {
    /// The fixed source.
    pub output: String,
    /// How many fixes were applied.
    pub applied: usize,
    /// The findings in `remaining` whose fix was refused because it would change data (their
    /// messages carry the refusal note). A fix refused in one pass and applied in a later one
    /// is not listed. Positions are in `output`.
    pub refused: Vec<Diagnostic>,
    /// The findings that remain in `output`.
    pub remaining: Vec<Diagnostic>,
}

fn config_error(pointer: &str, message: String) -> ConfigError {
    ConfigError {
        kind: ConfigErrorKind::Config,
        file: None,
        pointer: Some(pointer.to_string()),
        line: None,
        column: None,
        message,
    }
}

fn schema_error(file: Option<&String>, message: String) -> ConfigError {
    ConfigError {
        kind: ConfigErrorKind::Schema,
        file: file.cloned(),
        pointer: None,
        line: None,
        column: None,
        message,
    }
}

/// Compiles the JSON Schema text; `file` names it in errors.
fn compile_schema(
    source: &str,
    file: Option<&String>,
) -> Result<jsonschema::Validator, ConfigError> {
    let value: serde_json::Value = serde_json::from_str(source).map_err(|e| ConfigError {
        line: Some(e.line()),
        column: Some(e.column()),
        ..schema_error(file, format!("schema is not valid JSON: {e}"))
    })?;
    // The draft comes from `$schema` and defaults to 2020-12. External `$ref`s fail here: the
    // core crate does no I/O.
    jsonschema::options()
        .should_validate_formats(true)
        .with_pattern_options(jsonschema::PatternOptions::regex())
        .build(&value)
        .map_err(|e| schema_error(file, format!("invalid schema: {e}")))
}

impl Linter {
    /// Creates a linter, rejecting an invalid configuration.
    pub fn new(config: Config, schema_source: Option<&str>) -> Result<Linter, ConfigError> {
        if !(1..=8).contains(&config.indent_size) {
            return Err(config_error(
                "/rules/indentation",
                "indentation size must be between 1 and 8".to_string(),
            ));
        }
        if let Some(rule) = config.rules.keys().find(|rule| !rule.is_configurable()) {
            return Err(config_error(
                &format!("/rules/{rule}"),
                format!("rule `{rule}` can't be configured"),
            ));
        }
        let schema = schema_source
            .map(|source| compile_schema(source, config.schema.as_ref()))
            .transpose()?;
        Ok(Linter { config, schema })
    }

    /// Lints `source`.
    pub fn lint(&self, source: &str) -> LintReport {
        let at = |offset| TextRange {
            start: offset,
            end: offset,
        };
        let (front_matter, diagnostics) = match extract(source) {
            Extraction::Missing => (
                None,
                self.diagnostic(
                    RuleId::RequireFrontMatter,
                    require_front_matter::MISSING,
                    at(start_offset(source)),
                ),
            ),
            Extraction::Unclosed { open } => (
                None,
                self.diagnostic(
                    RuleId::RequireFrontMatter,
                    require_front_matter::UNCLOSED,
                    at(open.start),
                ),
            ),
            Extraction::TooLarge { open } => (
                None,
                self.diagnostic(RuleId::InputLimit, input_limit::TOO_LARGE, open),
            ),
            Extraction::Found(fm) => {
                let range = TextRange {
                    start: fm.open.start,
                    end: fm.end,
                };
                // A syntax error or exceeded limit stops linting; no other rule runs.
                let parsed = parse(source, &fm).and_then(|root| {
                    let data = build(&root)?;
                    Ok((root, data))
                });
                let diagnostics = match parsed {
                    Err(issue) => self.diagnostic(issue.rule, &issue.message, issue.range),
                    Ok((root, data)) => {
                        let doc = Document::new(source, &fm, root, data, &self.config.ignore_keys);
                        return LintReport {
                            front_matter: Some(range),
                            diagnostics: run(&Context::new(
                                source,
                                &fm,
                                &doc,
                                &self.config,
                                self.schema.as_ref(),
                            )),
                        };
                    }
                };
                (Some(range), diagnostics)
            }
        };
        LintReport {
            front_matter,
            diagnostics: diagnostics.into_iter().collect(),
        }
    }

    /// A diagnostic for `rule` at its configured severity, or `None` if the rule is off.
    fn diagnostic(&self, rule: RuleId, message: &str, range: TextRange) -> Option<Diagnostic> {
        Some(Diagnostic {
            rule,
            severity: severity(self.config.level(rule))?,
            message: message.to_string(),
            range,
            fix: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::testing::lint_all;
    use crate::{ConfigErrorKind, Level, RuleId, Severity};

    const MISSING: &str =
        "1:1-1:1 require-front-matter no front matter (the first line must be ---)";

    fn with_rule(rule: RuleId, level: Level) -> Config {
        let mut config = Config::default();
        config.rules.insert(rule, level);
        config
    }

    #[test]
    fn lint_no_front_matter() {
        assert_eq!(lint_all("hello\n", Config::default()), [MISSING]);
        assert_eq!(lint_all("", Config::default()), [MISSING]);
        assert_eq!(
            lint_all("hello\n", with_rule(RuleId::RequireFrontMatter, Level::Off)),
            Vec::<String>::new()
        );
        let linter = Linter::new(with_rule(RuleId::RequireFrontMatter, Level::Warn), None).unwrap();
        let report = linter.lint("hello\n");
        assert_eq!(report.front_matter, None);
        assert_eq!(report.diagnostics.len(), 1);
        assert_eq!(report.diagnostics[0].severity, Severity::Warning);
    }

    #[test]
    fn lint_no_front_matter_after_bom() {
        assert_eq!(
            lint_all("\u{FEFF}hello\n", Config::default()),
            ["1:1-1:1 require-front-matter no front matter (the first line must be ---)"]
        );
    }

    #[test]
    fn lint_unclosed() {
        assert_eq!(
            lint_all("---\na: 1\n", Config::default()),
            ["1:1-1:1 require-front-matter opening --- is never closed"]
        );
    }

    #[test]
    fn lint_too_large() {
        let source = format!("---\n{}", "a: 1\n".repeat(210_000));
        assert_eq!(
            lint_all(&source, Config::default()),
            ["1:1-1:4 input-limit front matter is larger than 1 MiB (1,048,576 bytes)"]
        );
    }

    #[test]
    fn lint_found_reports_the_block() {
        let linter = Linter::new(Config::default(), None).unwrap();
        let report = linter.lint("\u{FEFF}---\na: 1\n---\nbody");
        assert_eq!(report.front_matter, Some(TextRange { start: 3, end: 16 }));
        assert!(report.diagnostics.is_empty());
    }

    #[rstest]
    #[case::indent_zero(Config { indent_size: 0, ..Config::default() }, "/rules/indentation", "indentation size must be between 1 and 8")]
    #[case::indent_nine(Config { indent_size: 9, ..Config::default() }, "/rules/indentation", "indentation size must be between 1 and 8")]
    #[case::yaml_syntax(
        with_rule(RuleId::YamlSyntax, Level::Warn),
        "/rules/yaml-syntax",
        "rule `yaml-syntax` can't be configured"
    )]
    #[case::input_limit(
        with_rule(RuleId::InputLimit, Level::Off),
        "/rules/input-limit",
        "rule `input-limit` can't be configured"
    )]
    fn linter_rejects_bad_config(
        #[case] config: Config,
        #[case] pointer: &str,
        #[case] message: &str,
    ) {
        let error = Linter::new(config, None).unwrap_err();
        assert_eq!(error.kind, ConfigErrorKind::Config);
        assert_eq!(error.pointer.as_deref(), Some(pointer));
        assert_eq!(error.message, message);
    }

    #[test]
    fn linter_accepts_indent_bounds() {
        for indent_size in [1, 8] {
            let config = Config {
                indent_size,
                ..Config::default()
            };
            assert!(Linter::new(config, None).is_ok());
        }
    }

    #[test]
    fn linter_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Linter>();
    }
}
