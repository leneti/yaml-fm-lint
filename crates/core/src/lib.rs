//! Front-matter linting engine for yaml-fm-lint 2.0; see AGENTS.md.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod config;
mod diagnostic;
mod edit;
mod error;
mod rule_id;

pub use config::{Config, Level};
pub use diagnostic::{
    Applicability, DataEffect, Diagnostic, Fix, PathStep, Severity, TextEdit, TextRange,
};
pub use error::{ConfigError, ConfigErrorKind, FixError};
pub use rule_id::RuleId;
