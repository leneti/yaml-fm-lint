//! The `json` format: one document with every linted file, clean ones included.

use serde::Serialize;
use yaml_fm_lint_core::{Applicability, Severity};

use super::{Pos, position_of};
use crate::run::{FileResult, Summary};

/// The output of `yaml-fm-lint --format json`.
#[derive(Serialize)]
#[cfg_attr(
    test,
    derive(schemars::JsonSchema),
    schemars(
        title = "yaml-fm-lint output",
        extend("$id" = "https://unpkg.com/yaml-fm-lint@2/output.schema.json"),
        deny_unknown_fields
    )
)]
struct JsonReport {
    /// The version of this format.
    #[cfg_attr(test, schemars(extend("const" = 1)))]
    version: u32,
    /// Every linted file, sorted by path.
    files: Vec<JsonFile>,
    /// The totals.
    summary: JsonSummary,
}

#[derive(Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema), schemars(deny_unknown_fields))]
struct JsonFile {
    /// The path, as given on the command line or found by walking, with `/` separators.
    path: String,
    /// The problems in the file; empty when it is clean.
    diagnostics: Vec<JsonDiagnostic>,
}

#[derive(Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema), schemars(deny_unknown_fields))]
struct JsonDiagnostic {
    /// The rule that found the problem.
    rule: String,
    severity: JsonSeverity,
    message: String,
    /// Where the problem starts.
    start: JsonPosition,
    /// Where it ends, exclusive.
    end: JsonPosition,
    /// How to fix it, when there is a fix.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(test, schemars(with = "JsonFix", default))]
    fix: Option<JsonFix>,
}

#[derive(Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
enum JsonSeverity {
    Error,
    Warning,
}

#[derive(Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema), schemars(deny_unknown_fields))]
struct JsonPosition {
    /// Counted from 1.
    #[cfg_attr(test, schemars(range(min = 1)))]
    line: usize,
    /// Counted from 1, in characters.
    #[cfg_attr(test, schemars(range(min = 1)))]
    column: usize,
    /// Counted from 0, in bytes from the start of the file.
    offset: usize,
}

#[derive(Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema), schemars(deny_unknown_fields))]
struct JsonFix {
    /// `automatic` fixes are applied by `--fix`; `suggestion`s are only offered.
    applicability: JsonApplicability,
    /// The changes, sorted and not overlapping.
    edits: Vec<JsonEdit>,
}

#[derive(Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
enum JsonApplicability {
    Automatic,
    Suggestion,
}

#[derive(Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema), schemars(deny_unknown_fields))]
struct JsonEdit {
    /// The start of the text replaced.
    start: JsonPosition,
    /// The end of the text replaced, exclusive.
    end: JsonPosition,
    /// What replaces it.
    #[serde(rename = "newText")]
    new_text: String,
}

#[derive(Serialize)]
#[cfg_attr(test, derive(schemars::JsonSchema), schemars(deny_unknown_fields))]
struct JsonSummary {
    /// Problems with severity `error`.
    errors: usize,
    /// Problems with severity `warning`.
    warnings: usize,
    /// Problems with an automatic fix.
    fixable: usize,
    /// Files linted.
    files: usize,
}

impl From<Pos> for JsonPosition {
    fn from(pos: Pos) -> Self {
        JsonPosition {
            line: pos.line,
            column: pos.column,
            offset: pos.offset,
        }
    }
}

pub fn render(results: &[FileResult], summary: &Summary) -> String {
    let report = JsonReport {
        version: 1,
        files: results.iter().map(file).collect(),
        summary: JsonSummary {
            errors: summary.errors,
            warnings: summary.warnings,
            fixable: summary.fixable,
            files: summary.files,
        },
    };
    // These types always serialise.
    let mut out = serde_json::to_string(&report).unwrap_or_default();
    out.push('\n');
    out
}

fn file(result: &FileResult) -> JsonFile {
    let pos = position_of(result);
    let diagnostics = result
        .diagnostics
        .iter()
        .map(|d| JsonDiagnostic {
            rule: d.rule.to_string(),
            severity: match d.severity {
                Severity::Error => JsonSeverity::Error,
                Severity::Warning => JsonSeverity::Warning,
            },
            message: d.message.clone(),
            start: pos(d.range.start).into(),
            end: pos(d.range.end).into(),
            fix: d.fix.as_ref().map(|fix| JsonFix {
                applicability: match fix.applicability {
                    Applicability::Automatic => JsonApplicability::Automatic,
                    Applicability::Suggestion => JsonApplicability::Suggestion,
                },
                edits: fix
                    .edits
                    .iter()
                    .map(|edit| JsonEdit {
                        start: pos(edit.range.start).into(),
                        end: pos(edit.range.end).into(),
                        new_text: edit.new_text.clone(),
                    })
                    .collect(),
            }),
        })
        .collect();
    JsonFile {
        path: result.display.clone(),
        diagnostics,
    }
}

/// The JSON Schema of the output, generated from the types above and checked in at
/// `schemas/output.schema.json`.
#[cfg(test)]
mod tests {
    use super::JsonReport;

    const SCHEMA_PATH: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../schemas/output.schema.json"
    );

    #[test]
    fn output_schema_is_current() {
        let mut text = serde_json::to_string_pretty(&schemars::schema_for!(JsonReport)).unwrap();
        text.push('\n');
        if std::env::var("UPDATE_SCHEMAS").is_ok_and(|v| v == "1") {
            std::fs::write(SCHEMA_PATH, &text).unwrap();
        }
        let on_disk = std::fs::read_to_string(SCHEMA_PATH).unwrap_or_default();
        assert!(
            on_disk == text,
            "schemas/output.schema.json is out of date; run with UPDATE_SCHEMAS=1"
        );
        let schema: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(
            schema["$id"],
            "https://unpkg.com/yaml-fm-lint@2/output.schema.json"
        );
        assert_eq!(schema["additionalProperties"], false);
    }
}
