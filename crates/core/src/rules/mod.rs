//! The lint rules, one module per rule.

pub(crate) mod input_limit;
pub(crate) mod invalid_disable_comment;
pub(crate) mod require_front_matter;
pub(crate) mod required_keys;
pub(crate) mod yaml_syntax;

use crate::RuleId;
use crate::lint::RuleFn;

/// Every rule that runs on a parsed document; each rule task adds its row.
pub(crate) const RULES: &[(RuleId, RuleFn)] = &[(RuleId::RequiredKeys, required_keys::check)];
