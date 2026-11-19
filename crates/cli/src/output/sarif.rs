//! SARIF 2.1.0, one run with every rule and one result per problem.

use std::fmt::Write;

use serde_json::{Value, json};
use yaml_fm_lint_core::RuleId;

use super::{positions, severity_name};
use crate::run::FileResult;

const REPO: &str = "https://github.com/leneti/yaml-fm-lint";

pub fn render(results: &[FileResult]) -> String {
    let rules: Vec<Value> = RuleId::ALL
        .iter()
        .map(|rule| json!({"id": rule.as_str(), "helpUri": format!("{REPO}/blob/main/docs/rules.md#{rule}")}))
        .collect();
    let mut sarif_results = Vec::new();
    for result in results {
        let uri = uri(&result.display);
        for (d, (start, end)) in result.diagnostics.iter().zip(positions(result)) {
            sarif_results.push(json!({
                "ruleId": d.rule.as_str(),
                // `ALL` lists every rule.
                "ruleIndex": RuleId::ALL.iter().position(|r| *r == d.rule).unwrap_or_default(),
                "level": severity_name(d.severity),
                "message": {"text": d.message},
                "locations": [{"physicalLocation": {
                    "artifactLocation": {"uri": uri},
                    "region": {
                        "startLine": start.line,
                        "startColumn": start.column,
                        "endLine": end.line,
                        "endColumn": end.column,
                    },
                }}],
            }));
        }
    }
    let sarif = json!({
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": {"driver": {
                "name": "yaml-fm-lint",
                "version": env!("CARGO_PKG_VERSION"),
                "informationUri": REPO,
                "rules": rules,
            }},
            "columnKind": "unicodeCodePoints",
            "results": sarif_results,
        }],
    });
    // A `Value` always serialises.
    let mut out = serde_json::to_string(&sarif).unwrap_or_default();
    out.push('\n');
    out
}

/// `path` as a URI reference: every byte outside `A-Za-z0-9-._~/` is percent-encoded.
fn uri(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for &byte in path.as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~' | b'/') {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::uri;

    #[test]
    fn percent_encodes_everything_else() {
        assert_eq!(uri("docs/a-b_c.d~e/f.md"), "docs/a-b_c.d~e/f.md");
        assert_eq!(uri("my file ü#?%.md"), "my%20file%20%C3%BC%23%3F%25.md");
    }
}
