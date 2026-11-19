//! GitHub Actions workflow commands, one annotation per problem.

use std::fmt::Write;

use super::{positions, severity_name};
use crate::run::FileResult;

pub fn render(results: &[FileResult]) -> String {
    let mut out = String::new();
    for result in results {
        for (d, (start, end)) in result.diagnostics.iter().zip(positions(result)) {
            let _ = writeln!(
                out,
                "::{} file={},line={},col={},endLine={},endColumn={},title={}::{}",
                severity_name(d.severity),
                property(&result.display),
                start.line,
                start.column,
                end.line,
                end.column,
                d.rule,
                data(&d.message),
            );
        }
    }
    out
}

/// The text of a workflow command: `%`, CR and LF are written as `%25`, `%0D` and `%0A`.
fn data(s: &str) -> String {
    s.replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
}

/// A property value: like [`data`], and `:` and `,` as `%3A` and `%2C`.
fn property(s: &str) -> String {
    data(s).replace(':', "%3A").replace(',', "%2C")
}

#[cfg(test)]
mod tests {
    use super::{data, property};

    #[test]
    fn escapes() {
        assert_eq!(data("50%\r\n: ,"), "50%25%0D%0A: ,");
        assert_eq!(property("a%b:c,d\ne"), "a%25b%3Ac%2Cd%0Ae");
    }
}
