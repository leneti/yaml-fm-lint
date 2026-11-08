//! Rule identifiers.

use std::fmt;

use crate::Level;

/// Every rule the linter can report, in the order of the spec's rule table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RuleId {
    /// The front matter is not valid YAML.
    YamlSyntax,
    /// The front matter exceeds a size, depth or expansion limit.
    InputLimit,
    /// The file has no front matter.
    RequireFrontMatter,
    /// A required key is missing.
    RequiredKeys,
    /// The front matter contains a blank line.
    NoBlankLines,
    /// A key has whitespace before its colon.
    NoSpaceBeforeColon,
    /// A quoted scalar that reads the same unquoted.
    NoQuotes,
    /// A sequence is written in flow style.
    NoFlowSequence,
    /// A mapping is written in flow style.
    NoFlowMapping,
    /// A line, delimiter lines included, ends with whitespace.
    NoTrailingSpaces,
    /// A single-line plain scalar ends with a run of commas.
    NoTrailingCommas,
    /// A nested block collection is not indented exactly the configured size more than its parent.
    Indentation,
    /// A value contains repeated spaces.
    NoRepeatedSpaces,
    /// A colon is followed by more than one space.
    SingleSpaceAfterColon,
    /// A value contains a comma.
    NoCommas,
    /// The data does not match the JSON Schema.
    Schema,
    /// A disable comment is malformed or names an unknown rule.
    InvalidDisableComment,
}

impl RuleId {
    /// All rules, in table order.
    pub const ALL: [RuleId; 17] = [
        RuleId::YamlSyntax,
        RuleId::InputLimit,
        RuleId::RequireFrontMatter,
        RuleId::RequiredKeys,
        RuleId::NoBlankLines,
        RuleId::NoSpaceBeforeColon,
        RuleId::NoQuotes,
        RuleId::NoFlowSequence,
        RuleId::NoFlowMapping,
        RuleId::NoTrailingSpaces,
        RuleId::NoTrailingCommas,
        RuleId::Indentation,
        RuleId::NoRepeatedSpaces,
        RuleId::SingleSpaceAfterColon,
        RuleId::NoCommas,
        RuleId::Schema,
        RuleId::InvalidDisableComment,
    ];

    /// The kebab-case name used in configuration and output.
    pub fn as_str(self) -> &'static str {
        match self {
            RuleId::YamlSyntax => "yaml-syntax",
            RuleId::InputLimit => "input-limit",
            RuleId::RequireFrontMatter => "require-front-matter",
            RuleId::RequiredKeys => "required-keys",
            RuleId::NoBlankLines => "no-blank-lines",
            RuleId::NoSpaceBeforeColon => "no-space-before-colon",
            RuleId::NoQuotes => "no-quotes",
            RuleId::NoFlowSequence => "no-flow-sequence",
            RuleId::NoFlowMapping => "no-flow-mapping",
            RuleId::NoTrailingSpaces => "no-trailing-spaces",
            RuleId::NoTrailingCommas => "no-trailing-commas",
            RuleId::Indentation => "indentation",
            RuleId::NoRepeatedSpaces => "no-repeated-spaces",
            RuleId::SingleSpaceAfterColon => "single-space-after-colon",
            RuleId::NoCommas => "no-commas",
            RuleId::Schema => "schema",
            RuleId::InvalidDisableComment => "invalid-disable-comment",
        }
    }

    /// The rule with this kebab-case name, if any.
    pub fn from_name(name: &str) -> Option<RuleId> {
        RuleId::ALL.into_iter().find(|rule| rule.as_str() == name)
    }

    /// Whether configuration may change the rule's level. `yaml-syntax`, `input-limit` and
    /// `invalid-disable-comment` cannot be turned off or have their level changed.
    pub fn is_configurable(self) -> bool {
        !matches!(
            self,
            RuleId::YamlSyntax | RuleId::InputLimit | RuleId::InvalidDisableComment
        )
    }

    /// The level used when configuration does not override it.
    pub fn default_level(self) -> Level {
        match self {
            RuleId::NoRepeatedSpaces
            | RuleId::SingleSpaceAfterColon
            | RuleId::NoCommas
            | RuleId::InvalidDisableComment => Level::Warn,
            _ => Level::Error,
        }
    }

    /// Whether the rule is a style rule, the ones `ignoreKeys` exempts.
    pub fn is_style(self) -> bool {
        RuleId::NoBlankLines <= self && self <= RuleId::NoCommas
    }
}

impl fmt::Display for RuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Level;

    #[test]
    fn rule_names_round_trip() {
        for rule in RuleId::ALL {
            assert_eq!(RuleId::from_name(rule.as_str()), Some(rule));
        }
        assert_eq!(RuleId::NoSpaceBeforeColon.as_str(), "no-space-before-colon");
        assert_eq!(RuleId::from_name("nope"), None);
    }

    #[test]
    fn defaults_match_spec_table() {
        let names = |f: &dyn Fn(RuleId) -> bool| -> Vec<&'static str> {
            RuleId::ALL
                .into_iter()
                .filter(|r| f(*r))
                .map(RuleId::as_str)
                .collect()
        };
        assert_eq!(
            names(&|r| r.default_level() == Level::Warn),
            [
                "no-repeated-spaces",
                "single-space-after-colon",
                "no-commas",
                "invalid-disable-comment"
            ]
        );
        assert_eq!(
            names(&|r| r.is_style()),
            [
                "no-blank-lines",
                "no-space-before-colon",
                "no-quotes",
                "no-flow-sequence",
                "no-flow-mapping",
                "no-trailing-spaces",
                "no-trailing-commas",
                "indentation",
                "no-repeated-spaces",
                "single-space-after-colon",
                "no-commas"
            ]
        );
        assert_eq!(
            names(&|r| !r.is_configurable()),
            ["yaml-syntax", "input-limit", "invalid-disable-comment"]
        );
    }
}
