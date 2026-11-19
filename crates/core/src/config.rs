//! Linter configuration.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::RuleId;

/// How a rule's findings are reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Level {
    /// The rule is disabled.
    Off,
    /// Findings are warnings.
    Warn,
    /// Findings are errors.
    Error,
}

/// One `files.include` or `files.exclude` pattern, with the folder it is relative to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilePattern {
    /// The absolute, `/`-separated folder of the config file that set the pattern.
    pub dir: String,
    /// The pattern as written, including any leading `!`.
    pub pattern: String,
}

/// Which files are linted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Files {
    /// Extensions of the files to lint, each with its leading dot.
    pub extensions: Vec<String>,
    /// Patterns a path must match, if there are any.
    pub include: Vec<FilePattern>,
    /// Patterns that exclude paths; `!` re-includes and the last match wins.
    pub exclude: Vec<FilePattern>,
}

impl Default for Files {
    fn default() -> Self {
        Files {
            extensions: vec![".md".to_string()],
            include: Vec::new(),
            exclude: Vec::new(),
        }
    }
}

/// Linter configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// Per-rule level overrides.
    pub rules: BTreeMap<RuleId, Level>,
    /// Keys that must be present at the top level.
    pub required_keys: Vec<String>,
    /// Spaces per indentation level.
    pub indent_size: u8,
    /// Keys whose subtrees style rules skip.
    pub ignore_keys: Vec<String>,
    /// Path of the JSON Schema to validate against. The core reads no files: the caller loads
    /// the file and passes its text to [`Linter::new`](crate::Linter::new), which uses this
    /// only to name the schema in errors, and rejects a path given without that text.
    pub schema: Option<String>,
    /// Which files are linted.
    pub files: Files,
    /// The config files that were applied, lowest priority first; a `--config` file is last.
    pub layers: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            rules: BTreeMap::new(),
            required_keys: Vec::new(),
            indent_size: 2,
            ignore_keys: Vec::new(),
            schema: None,
            files: Files::default(),
            layers: Vec::new(),
        }
    }
}

impl Config {
    /// The effective level of `rule`: the override if the rule is configurable, else its default.
    pub fn level(&self, rule: RuleId) -> Level {
        if rule.is_configurable() {
            self.rules
                .get(&rule)
                .copied()
                .unwrap_or_else(|| rule.default_level())
        } else {
            rule.default_level()
        }
    }

    /// The effective configuration as JSON, for `print-config`; `layers` is not included.
    pub fn to_json(&self) -> Value {
        let pattern = |p: &FilePattern| json!({"dir": p.dir, "pattern": p.pattern});
        let mut rules = serde_json::Map::new();
        for rule in RuleId::ALL.into_iter().filter(|r| r.is_configurable()) {
            let level = self.level(rule);
            let name = match level {
                Level::Off => "off",
                Level::Warn => "warn",
                Level::Error => "error",
            };
            let value = match (rule, level) {
                (_, Level::Off) => json!("off"),
                (RuleId::RequiredKeys, _) => json!([name, {"keys": self.required_keys}]),
                (RuleId::Indentation, _) => json!([name, {"size": self.indent_size}]),
                _ => json!(name),
            };
            rules.insert(rule.as_str().to_string(), value);
        }
        json!({
            "files": {
                "extensions": self.files.extensions,
                "include": self.files.include.iter().map(pattern).collect::<Vec<_>>(),
                "exclude": self.files.exclude.iter().map(pattern).collect::<Vec<_>>(),
            },
            "ignoreKeys": self.ignore_keys,
            "rules": rules,
            "schema": self.schema,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RuleId;

    #[test]
    fn config_level_overrides() {
        let mut config = Config::default();
        config.rules.insert(RuleId::NoCommas, Level::Off);
        config.rules.insert(RuleId::YamlSyntax, Level::Off);
        assert_eq!(config.level(RuleId::NoCommas), Level::Off);
        assert_eq!(config.level(RuleId::YamlSyntax), Level::Error);
        assert_eq!(config.level(RuleId::NoQuotes), Level::Error);
    }
}
