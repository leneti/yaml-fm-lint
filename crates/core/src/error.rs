//! Error types.

use thiserror::Error;

use crate::Diagnostic;

/// What kind of problem a [`ConfigError`] reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigErrorKind {
    /// The configuration is invalid.
    Config,
    /// The JSON Schema is invalid.
    Schema,
}

/// An invalid configuration or schema.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{}{}{}", prefix(.file), prefix(.pointer), .message)]
pub struct ConfigError {
    /// Whether the configuration or the schema is at fault.
    pub kind: ConfigErrorKind,
    /// The file the problem is in.
    pub file: Option<String>,
    /// JSON pointer to the problem.
    pub pointer: Option<String>,
    /// 1-based line of the problem.
    pub line: Option<usize>,
    /// 1-based column of the problem.
    pub column: Option<usize>,
    /// What is wrong.
    pub message: String,
}

fn prefix(part: &Option<String>) -> String {
    part.as_ref().map(|p| format!("{p}: ")).unwrap_or_default()
}

/// `fix` could not run.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum FixError {
    /// The input cannot be fixed; these diagnostics say why.
    #[error("input has syntax errors or exceeds limits")]
    Unparseable(Vec<Diagnostic>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_error_display() {
        let mut e = ConfigError {
            kind: ConfigErrorKind::Config,
            file: Some("a.json".into()),
            pointer: Some("/rules/x".into()),
            line: None,
            column: None,
            message: "unknown rule".into(),
        };
        assert_eq!(e.to_string(), "a.json: /rules/x: unknown rule");
        e.file = None;
        e.pointer = None;
        assert_eq!(e.to_string(), "unknown rule");
    }
}
