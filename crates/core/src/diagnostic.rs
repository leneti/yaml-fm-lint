//! Diagnostics, fixes and text ranges.

use crate::RuleId;

/// How a diagnostic is reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Fails the run.
    Error,
    /// Reported without failing the run.
    Warning,
}

/// A byte range into the whole source; `end` is exclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TextRange {
    /// Start byte offset.
    pub start: usize,
    /// End byte offset, exclusive.
    pub end: usize,
}

/// Replacement of a byte range with new text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    /// The range to replace.
    pub range: TextRange,
    /// The replacement text.
    pub new_text: String,
}

/// Whether a fix may be applied without being asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Applicability {
    /// Applied by `fix`.
    Automatic,
    /// Only offered to the user.
    Suggestion,
}

/// One step of a path through the parsed data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathStep {
    /// The key of the mapping entry at this index.
    Key(usize),
    /// The value of the mapping entry at this index.
    Value(usize),
    /// The sequence item at this index.
    Item(usize),
}

/// What a fix does to the parsed data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataEffect {
    /// The parsed data is unchanged.
    Preserving,
    /// One scalar changes to a plain scalar with this text.
    ChangesScalar {
        /// Where the scalar is.
        path: Vec<PathStep>,
        /// The scalar's new plain text.
        new_plain: String,
    },
}

/// A fix for a diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fix {
    /// Whether the fix is applied automatically.
    pub applicability: Applicability,
    /// What the fix does to the parsed data.
    pub data_effect: DataEffect,
    /// The edits, sorted and non-overlapping.
    pub edits: Vec<TextEdit>,
}

/// A finding in a source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// The rule that fired.
    pub rule: RuleId,
    /// How serious the finding is.
    pub severity: Severity,
    /// The message shown to the user.
    pub message: String,
    /// Where the finding is.
    pub range: TextRange,
    /// A fix, if one exists.
    pub fix: Option<Fix>,
}
