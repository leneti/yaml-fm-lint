//! `path:line:col: severity [rule] message`, one line per problem.

use std::fmt::Write;

use super::{escape, positions, severity_name};
use crate::run::FileResult;

pub fn render(results: &[FileResult]) -> String {
    let mut out = String::new();
    for result in results {
        for (d, (start, _)) in result.diagnostics.iter().zip(positions(result)) {
            let severity = severity_name(d.severity);
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
