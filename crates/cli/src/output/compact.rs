//! `path:line:col: severity [rule] message`, one line per problem.

use std::fmt::Write;

use yaml_fm_lint_core::Severity;

use super::{escape, positions};
use crate::run::FileResult;

pub fn render(results: &[FileResult]) -> String {
    let mut out = String::new();
    for result in results {
        for (d, (start, _)) in result.diagnostics.iter().zip(positions(result)) {
            let severity = match d.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
            };
            let _ = writeln!(
                out,
                "{}:{}:{}: {severity} [{}] {}",
                escape(&result.display),
                start.line,
                start.column,
                d.rule,
                escape(&d.message)
            );
        }
    }
    out
}
