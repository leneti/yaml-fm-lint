//! The lint rules, one module per rule.

pub(crate) mod flow_fix;
pub(crate) mod input_limit;
pub(crate) mod invalid_disable_comment;
pub(crate) mod no_blank_lines;
pub(crate) mod no_commas;
pub(crate) mod no_flow_sequence;
pub(crate) mod no_quotes;
pub(crate) mod no_space_before_colon;
pub(crate) mod no_trailing_commas;
pub(crate) mod no_trailing_spaces;
pub(crate) mod require_front_matter;
pub(crate) mod required_keys;
pub(crate) mod single_space_after_colon;
pub(crate) mod yaml_syntax;

use crate::RuleId;
use crate::lint::RuleFn;

/// Every rule that runs on a parsed document; each rule task adds its row.
pub(crate) const RULES: &[(RuleId, RuleFn)] = &[
    (RuleId::RequiredKeys, required_keys::check),
    (RuleId::NoTrailingCommas, no_trailing_commas::check),
    (RuleId::NoTrailingSpaces, no_trailing_spaces::check),
    (RuleId::NoBlankLines, no_blank_lines::check),
    (RuleId::NoSpaceBeforeColon, no_space_before_colon::check),
    (
        RuleId::SingleSpaceAfterColon,
        single_space_after_colon::check,
    ),
    (RuleId::NoQuotes, no_quotes::check),
    (RuleId::NoCommas, no_commas::check),
    (RuleId::NoFlowSequence, no_flow_sequence::check),
];
