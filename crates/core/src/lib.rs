//! Front-matter linting engine for yaml-fm-lint 2.0; see AGENTS.md.
//!
//! The crate finds the YAML front matter in a markdown file, parses it, runs the lint
//! rules and reports [`Diagnostic`]s with byte ranges. It can also fix what it finds:
//! every fix is re-parsed and checked before it is accepted, and a fix that would change
//! the data is refused.
//!
//! The crate does no I/O of any kind: no files, processes, network or environment.
//! Callers pass in text (and a schema as text) and receive results.
//!
//! ```
//! use yaml_fm_lint_core::{Config, Linter};
//!
//! let linter = Linter::new(Config::default(), None).unwrap();
//! let report = linter.lint("---\ntitle: \"Hello\"\n---\n# Body\n");
//! assert!(!report.diagnostics.is_empty());
//! let fixed = linter.fix("---\ntitle: \"Hello\"\n---\n# Body\n").unwrap();
//! assert_eq!(fixed.output, "---\ntitle: Hello\n---\n# Body\n");
//! ```
#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod config;
mod data;
mod diagnostic;
mod disable;
mod document;
mod edit;
mod error;
mod extract;
mod fix;
#[cfg(feature = "invariants")]
pub mod invariants;
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
pub use fix::{REFUSED_NOTE, SKIPPED_NOTE};
pub use line_index::{LineIndex, Position};
pub use linter::{FixOutcome, LintReport, Linter};
pub use rule_id::RuleId;
