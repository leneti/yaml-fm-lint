//! Linter configuration.

use std::collections::BTreeMap;

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
}

impl Default for Config {
    fn default() -> Self {
        Config {
            rules: BTreeMap::new(),
            required_keys: Vec::new(),
            indent_size: 2,
            ignore_keys: Vec::new(),
            schema: None,
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
