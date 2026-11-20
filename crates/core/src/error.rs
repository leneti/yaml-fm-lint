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
#[error("{}{}{}", location(.file, .line, .column), prefix(.pointer), .message)]
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

/// `file[:line:column]: `, or nothing without a file.
fn location(file: &Option<String>, line: &Option<usize>, column: &Option<usize>) -> String {
    match (file, line, column) {
        (Some(file), Some(line), Some(column)) => format!("{file}:{line}:{column}: "),
        _ => prefix(file),
    }
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

    #[test]
    fn config_error_display_with_position() {
        let e = ConfigError {
            kind: ConfigErrorKind::Config,
            file: Some("a.json".into()),
            pointer: None,
            line: Some(3),
            column: Some(11),
            message: "x".into(),
        };
        assert_eq!(e.to_string(), "a.json:3:11: x");
    }
}
