//! Front-matter linting engine for yaml-fm-lint 2.0; see AGENTS.md.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod config;
mod data;
mod diagnostic;
mod document;
mod edit;
mod error;
mod extract;
mod line_index;
mod lint;
mod linter;
mod parse;
mod rule_id;
mod rules;
#[cfg(test)]
mod testing;
mod tree;

pub use config::{Config, Level};
pub use diagnostic::{
    Applicability, DataEffect, Diagnostic, Fix, PathStep, Severity, TextEdit, TextRange,
};
pub use error::{ConfigError, ConfigErrorKind, FixError};
pub use line_index::{LineIndex, Position};
pub use linter::{FixOutcome, LintReport, Linter};
pub use rule_id::RuleId;
